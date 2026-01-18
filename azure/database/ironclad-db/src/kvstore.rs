/// KVStore: High-level Key-Value Store interface
/// 
/// Provides ACID-compliant key-value operations on top of the buffer pool and WAL.
/// Each key-value pair is stored in its own page with checksums for integrity.

use crate::azure_disk::{AzureDisk, PAGE_SIZE};
use crate::buffer_pool::BufferPool;
use crate::wal::{WriteAheadLog, WalEntry};
use crate::error::{IronCladError, Result};
use std::sync::Arc;
use dashmap::DashMap;
use tracing::{info, warn, debug};

// Page format:
// [Magic: 4 bytes] [Version: 2 bytes] [Checksum: 4 bytes] [LSN: 8 bytes]
// [Key Length: 4 bytes] [Value Length: 4 bytes]
// [Key Data] [Value Data] [Padding to 4096]

const MAGIC_NUMBER: u32 = 0x49524F4E; // "IRON" in hex
const VERSION: u16 = 1;
const HEADER_SIZE: usize = 4 + 2 + 4 + 8 + 4 + 4; // 26 bytes
const MAX_KEY_SIZE: usize = 256;
const MAX_VALUE_SIZE: usize = PAGE_SIZE - HEADER_SIZE - MAX_KEY_SIZE - 4; // Leave room for lengths

pub struct KVStore {
    disk: Arc<AzureDisk>,
    buffer_pool: Arc<BufferPool>,
    wal: Arc<WriteAheadLog>,
    index: Arc<DashMap<String, u64>>, // key -> page_id mapping
    next_page_id: Arc<parking_lot::RwLock<u64>>,
}

impl KVStore {
    pub async fn new(connection_string: &str) -> Result<Self> {
        info!("Initializing KVStore");

        let container_name = "ironclad-data";
        let disk = Arc::new(
            AzureDisk::new(connection_string, container_name, "db-pages").await?
        );
        let buffer_pool = Arc::new(BufferPool::new(disk.clone()));
        let wal = Arc::new(
            WriteAheadLog::new(connection_string, container_name, "db-wal").await?
        );

        let store = Self {
            disk: disk.clone(),
            buffer_pool: buffer_pool.clone(),
            wal: wal.clone(),
            index: Arc::new(DashMap::new()),
            next_page_id: Arc::new(parking_lot::RwLock::new(1)),
        };

        // Recover from WAL if needed
        store.recover().await?;

        Ok(store)
    }

    /// Recover from WAL
    async fn recover(&self) -> Result<()> {
        let entries = self.wal.get_entries();
        if entries.is_empty() {
            info!("No WAL entries to recover");
            return Ok(());
        }

        info!("Recovering {} WAL entries", entries.len());

        for entry in entries {
            // Replay the operation
            match entry.operation.as_str() {
                "PUT" => {
                    // Write page to buffer pool
                    self.buffer_pool.put_page(entry.page_id, entry.data.clone()).await?;
                    
                    // Decode key from page to rebuild index
                    if let Ok((key, _)) = decode_kv_page(&entry.data) {
                        self.index.insert(key.clone(), entry.page_id);
                        
                        // Update next_page_id
                        let mut next = self.next_page_id.write();
                        *next = (*next).max(entry.page_id + 1);
                    }
                }
                "DELETE" => {
                    // Decode key and remove from index
                    if let Ok((key, _)) = decode_kv_page(&entry.data) {
                        self.index.remove(&key);
                    }
                }
                _ => {
                    warn!("Unknown WAL operation: {}", entry.operation);
                }
            }
        }

        info!("Recovery complete: {} keys in index", self.index.len());
        Ok(())
    }

    /// Set a key-value pair
    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        if key.len() > MAX_KEY_SIZE {
            return Err(IronCladError::KeyTooLarge { 
                size: key.len(), 
                max: MAX_KEY_SIZE 
            });
        }
        if value.len() > MAX_VALUE_SIZE {
            return Err(IronCladError::ValueTooLarge { 
                size: value.len(), 
                max: MAX_VALUE_SIZE 
            });
        }

        debug!("Setting key: {}", key);

        // Get or allocate page ID
        let page_id = match self.index.get(key) {
            Some(entry) => *entry.value(),
            None => {
                let mut next = self.next_page_id.write();
                let id = *next;
                *next += 1;
                id
            }
        };

        // Encode key-value into page
        let page_data = encode_kv_page(key, value, self.wal.current_lsn())?;

        // Write to WAL first (CRITICAL for durability)
        let wal_entry = WalEntry {
            lsn: 0, // Will be assigned by WAL
            page_id,
            operation: "PUT".to_string(),
            data: page_data.clone(),
        };
        self.wal.append_entry(wal_entry).await?;

        // Now write to buffer pool (marks page dirty)
        self.buffer_pool.put_page(page_id, page_data).await?;

        // Update index
        self.index.insert(key.to_string(), page_id);

        Ok(())
    }

    /// Get a value by key
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        debug!("Getting key: {}", key);

        // Look up page ID in index
        let page_id = match self.index.get(key) {
            Some(entry) => *entry.value(),
            None => return Ok(None),
        };

        // Pin the page to prevent eviction while we're reading it
        self.buffer_pool.pin_page(page_id)?;

        // Get page from buffer pool
        let page_data = match self.buffer_pool.get_page(page_id).await? {
            Some(data) => data,
            None => {
                self.buffer_pool.unpin_page(page_id)?;
                return Ok(None);
            }
        };

        // Decode page
        let result = match decode_kv_page(&page_data) {
            Ok((decoded_key, value)) => {
                if decoded_key == key {
                    Ok(Some(value))
                } else {
                    warn!("Key mismatch in page {}: expected '{}', got '{}'", 
                        page_id, key, decoded_key);
                    Ok(None)
                }
            }
            Err(e) => {
                warn!("Failed to decode page {}: {}", page_id, e);
                Ok(None)
            }
        };

        // Unpin the page
        self.buffer_pool.unpin_page(page_id)?;

        result
    }

    /// Delete a key
    pub async fn delete(&self, key: &str) -> Result<bool> {
        debug!("Deleting key: {}", key);

        let page_id = match self.index.get(key) {
            Some(entry) => *entry.value(),
            None => return Ok(false),
        };

        // Create empty page for deletion marker
        let empty_page = vec![0u8; PAGE_SIZE];

        // Write to WAL
        let wal_entry = WalEntry {
            lsn: 0,
            page_id,
            operation: "DELETE".to_string(),
            data: empty_page.clone(),
        };
        self.wal.append_entry(wal_entry).await?;

        // Remove from index
        self.index.remove(key);

        // Mark page as deleted (write empty page)
        self.buffer_pool.put_page(page_id, empty_page).await?;

        Ok(true)
    }

    /// Scan all keys
    pub async fn scan(&self) -> Result<Vec<(String, String)>> {
        debug!("Scanning all keys");

        let mut results = Vec::new();
        
        for entry in self.index.iter() {
            let key = entry.key().clone();
            if let Ok(Some(value)) = self.get(&key).await {
                results.push((key, value));
            }
        }

        Ok(results)
    }

    /// Flush all dirty pages to disk
    pub async fn flush(&self) -> Result<()> {
        let dirty_pages = self.buffer_pool.get_dirty_pages();
        info!("Flushing {} dirty pages to Azure", dirty_pages.len());

        for (page_id, data) in dirty_pages {
            // Write to Azure
            self.disk.write_page(page_id, &data).await?;
            
            // Clear dirty flag
            self.buffer_pool.clear_dirty(page_id)?;
        }

        // Flush WAL
        self.wal.flush().await?;

        // Flush disk
        self.disk.flush().await?;

        info!("Flush complete");
        Ok(())
    }

    /// Get statistics
    pub fn stats(&self) -> String {
        let bp_stats = self.buffer_pool.stats();
        format!(
            "Keys: {}, Buffer: {}/{} frames, Dirty: {}, Pinned: {}, WAL LSN: {}",
            self.index.len(),
            bp_stats.used_frames,
            bp_stats.total_frames,
            bp_stats.dirty_pages,
            bp_stats.pinned_pages,
            self.wal.current_lsn()
        )
    }
}

/// Encode a key-value pair into a page with checksum
fn encode_kv_page(key: &str, value: &str, lsn: u64) -> Result<Vec<u8>> {
    let mut page = vec![0u8; PAGE_SIZE];

    let key_bytes = key.as_bytes();
    let value_bytes = value.as_bytes();

    if key_bytes.len() > MAX_KEY_SIZE {
        return Err(IronCladError::KeyTooLarge { 
            size: key_bytes.len(), 
            max: MAX_KEY_SIZE 
        });
    }

    if value_bytes.len() > MAX_VALUE_SIZE {
        return Err(IronCladError::ValueTooLarge { 
            size: value_bytes.len(), 
            max: MAX_VALUE_SIZE 
        });
    }

    let mut offset = 0;

    // Magic number
    page[offset..offset + 4].copy_from_slice(&MAGIC_NUMBER.to_le_bytes());
    offset += 4;

    // Version
    page[offset..offset + 2].copy_from_slice(&VERSION.to_le_bytes());
    offset += 2;

    // Checksum placeholder (will compute after)
    offset += 4;

    // LSN
    page[offset..offset + 8].copy_from_slice(&lsn.to_le_bytes());
    offset += 8;

    // Key length
    let key_len = key_bytes.len() as u32;
    page[offset..offset + 4].copy_from_slice(&key_len.to_le_bytes());
    offset += 4;

    // Value length
    let value_len = value_bytes.len() as u32;
    page[offset..offset + 4].copy_from_slice(&value_len.to_le_bytes());
    offset += 4;

    // Key data
    page[offset..offset + key_bytes.len()].copy_from_slice(key_bytes);
    offset += key_bytes.len();

    // Value data
    page[offset..offset + value_bytes.len()].copy_from_slice(value_bytes);

    // Compute checksum (skip magic and version, checksum the rest)
    let checksum = crc32fast::hash(&page[10..]);
    page[6..10].copy_from_slice(&checksum.to_le_bytes());

    Ok(page)
}

/// Decode a key-value page and verify checksum
fn decode_kv_page(page: &[u8]) -> Result<(String, String)> {
    if page.len() != PAGE_SIZE {
        return Err(IronCladError::InvalidPageFormat {
            reason: format!("Invalid page size: {}", page.len())
        });
    }

    let mut offset = 0;

    // Magic number
    let magic = u32::from_le_bytes([page[0], page[1], page[2], page[3]]);
    if magic != MAGIC_NUMBER {
        return Err(IronCladError::InvalidPageFormat {
            reason: format!("Invalid magic number: {:#x}", magic)
        });
    }
    offset += 4;

    // Version
    let version = u16::from_le_bytes([page[offset], page[offset + 1]]);
    if version != VERSION {
        return Err(IronCladError::InvalidPageFormat {
            reason: format!("Unsupported version: {}", version)
        });
    }
    offset += 2;

    // Checksum
    let stored_checksum = u32::from_le_bytes([
        page[offset], page[offset + 1], page[offset + 2], page[offset + 3]
    ]);
    offset += 4;

    // Verify checksum
    let computed_checksum = crc32fast::hash(&page[10..]);
    if stored_checksum != computed_checksum {
        return Err(IronCladError::ChecksumMismatch {
            page_id: 0,
            expected: stored_checksum,
            actual: computed_checksum,
        });
    }

    // LSN (we just skip it for now)
    offset += 8;

    // Key length
    let key_len = u32::from_le_bytes([
        page[offset], page[offset + 1], page[offset + 2], page[offset + 3]
    ]) as usize;
    offset += 4;

    // Value length
    let value_len = u32::from_le_bytes([
        page[offset], page[offset + 1], page[offset + 2], page[offset + 3]
    ]) as usize;
    offset += 4;

    // Key data
    let key = String::from_utf8(page[offset..offset + key_len].to_vec())?;
    offset += key_len;

    // Value data
    let value = String::from_utf8(page[offset..offset + value_len].to_vec())?;

    Ok((key, value))
}

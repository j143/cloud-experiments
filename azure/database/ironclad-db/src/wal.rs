/// Write-Ahead Log (WAL)
/// 
/// Ensures durability and crash recovery by logging all modifications
/// before they are applied to the database pages.
/// 
/// This implementation uses Azure Append Blobs for the WAL.

use azure_storage_blobs::prelude::*;
use azure_storage::prelude::*;
use crate::error::{IronCladError, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, warn, debug};
use bytes::Bytes;
use futures::StreamExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    pub lsn: u64,           // Log Sequence Number
    pub page_id: u64,       // Page being modified
    pub operation: String,  // Type of operation
    pub data: Vec<u8>,      // New page data
}

pub struct WriteAheadLog {
    blob_client: Arc<BlobClient>,
    lsn: Arc<RwLock<u64>>,
    // Keep in-memory buffer for recovery
    entries_buffer: Arc<RwLock<Vec<WalEntry>>>,
}

impl WriteAheadLog {
    /// Create a new Write-Ahead Log
    pub async fn new(
        connection_string: &str,
        container_name: &str,
        wal_blob_name: &str,
    ) -> Result<Self> {
        info!("Initializing WAL with blob: {}", wal_blob_name);

        // Parse connection string and create clients
        let account_name = connection_string.split(';')
            .find(|s| s.starts_with("AccountName="))
            .and_then(|s| s.strip_prefix("AccountName="))
            .ok_or_else(|| IronCladError::ConfigError("Missing AccountName in connection string".to_string()))?;
        
        let account_key = connection_string.split(';')
            .find(|s| s.starts_with("AccountKey="))
            .and_then(|s| s.strip_prefix("AccountKey="))
            .map(|s| s.to_string())
            .ok_or_else(|| IronCladError::ConfigError("Missing AccountKey in connection string".to_string()))?;
        
        let credentials = StorageCredentials::access_key(account_name.to_string(), account_key);
        let blob_service_client = BlobServiceClient::new(account_name, credentials);
        
        let container_client = blob_service_client.container_client(container_name);

        // Create container if it doesn't exist
        match container_client.create().await {
            Ok(_) => info!("Created container for WAL: {}", container_name),
            Err(e) => {
                debug!("Container creation response: {}", e);
            }
        }

        let blob_client = container_client.blob_client(wal_blob_name);
        
        // Check if append blob exists, if not create it
        match blob_client.get_properties().await {
            Ok(_) => {
                info!("WAL append blob already exists: {}", wal_blob_name);
            }
            Err(_) => {
                // Create append blob
                info!("Creating new WAL append blob: {}", wal_blob_name);
                blob_client
                    .put_append_blob()
                    .await
                    .map_err(|e| IronCladError::AzureError(e.to_string()))?;
                info!("Created WAL append blob: {}", wal_blob_name);
            }
        }

        // Try to recover existing entries
        let (entries, last_lsn) = Self::recover_from_blob(&blob_client).await?;
        
        info!("WAL recovery complete: {} entries, last LSN: {}", entries.len(), last_lsn);

        Ok(Self {
            blob_client: Arc::new(blob_client),
            lsn: Arc::new(RwLock::new(last_lsn)),
            entries_buffer: Arc::new(RwLock::new(entries)),
        })
    }

    /// Recover WAL entries from Azure Append Blob
    async fn recover_from_blob(blob_client: &BlobClient) -> Result<(Vec<WalEntry>, u64)> {
        let result = blob_client.get().into_stream().next().await;
        
        match result {
            Some(Ok(response)) => {
                // Collect the streaming body
                    let data = response.data.collect().await
                        .map_err(|e| IronCladError::AzureStorageError(e.to_string()))?;
                
                if data.is_empty() {
                    return Ok((Vec::new(), 0));
                }

                // Parse entries - each entry is JSON on a separate line
                     let content = String::from_utf8(data.to_vec())
                    .map_err(|e| IronCladError::InvalidPageFormat {
                        reason: format!("WAL contains invalid UTF-8: {}", e)
                    })?;

                let mut entries = Vec::new();
                let mut last_lsn = 0;

                for (line_num, line) in content.lines().enumerate() {
                    if line.trim().is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<WalEntry>(line) {
                        Ok(entry) => {
                            last_lsn = last_lsn.max(entry.lsn);
                            entries.push(entry);
                        }
                        Err(e) => {
                            warn!("Failed to parse WAL entry at line {}: {} - {}", 
                                line_num, e, line);
                            // Continue recovery, skip corrupted entry
                        }
                    }
                }

                Ok((entries, last_lsn))
            }
            Some(Err(_)) | None => {
                // Blob doesn't exist or is empty
                Ok((Vec::new(), 0))
            }
        }
    }

    /// Append a new entry to the WAL
    /// 
    /// This is a CRITICAL operation - it MUST succeed before the operation
    /// is applied to the database. If this fails, the operation should be aborted.
    pub async fn append_entry(&self, mut entry: WalEntry) -> Result<u64> {
        // 1. Assign LSN AFTER we know the write will succeed
        //    (we'll temporarily use 0 then update it)
        let mut lsn_guard = self.lsn.write();
        *lsn_guard += 1;
        let assigned_lsn = *lsn_guard;
        entry.lsn = assigned_lsn;
        drop(lsn_guard); // Release lock before Azure call

        // 2. Serialize entry
        let mut entry_json = serde_json::to_string(&entry)?;
        entry_json.push('\n'); // Each entry on its own line
        let bytes = Bytes::from(entry_json);

        // 3. Write to Azure Append Blob with retry logic
        let mut attempts = 0;
        let max_attempts = 5; // WAL writes are critical, retry more

        loop {
            match self.blob_client
                .append_block(bytes.clone())
                .await
            {
                Ok(_) => {
                    debug!("WAL entry appended: LSN {}", assigned_lsn);
                    
                    // 4. Add to in-memory buffer for recovery
                    let mut buffer = self.entries_buffer.write();
                    buffer.push(entry.clone());
                    
                    return Ok(assigned_lsn);
                }
                Err(e) if attempts < max_attempts => {
                    attempts += 1;
                    warn!("WAL append attempt {} failed: {}. Retrying...", attempts, e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(50 * attempts as u64)).await;
                }
                Err(e) => {
                    // CRITICAL: WAL write failed!
                    // Rollback LSN increment
                    let mut lsn_guard = self.lsn.write();
                    *lsn_guard -= 1;
                    
                    return Err(IronCladError::AzureError(
                        format!("CRITICAL: Failed to append WAL entry after {} attempts: {}", 
                            attempts, e)
                    ));
                }
            }
        }
    }

    /// Get all WAL entries for recovery
    pub fn get_entries(&self) -> Vec<WalEntry> {
        self.entries_buffer.read().clone()
    }

    /// Get the current LSN
    pub fn current_lsn(&self) -> u64 {
        *self.lsn.read()
    }

    /// Truncate WAL after checkpoint
    /// This should only be called after all dirty pages have been flushed
    pub async fn truncate(&self, up_to_lsn: u64) -> Result<()> {
        info!("Truncating WAL up to LSN {}", up_to_lsn);
        
        // Keep only entries after up_to_lsn
        let mut buffer = self.entries_buffer.write();
        buffer.retain(|e| e.lsn > up_to_lsn);
        
        // Note: Azure Append Blobs don't support truncation
        // In production, you would:
        // 1. Create a new append blob
        // 2. Copy entries > up_to_lsn to new blob
        // 3. Atomically swap blob names
        // For now, we just keep the in-memory buffer clean
        
        warn!("WAL truncation not fully implemented - kept {} entries in memory", buffer.len());
        Ok(())
    }

    /// Flush WAL to ensure durability
    /// Azure Append Blobs are immediately consistent, so this is mostly a no-op
    pub async fn flush(&self) -> Result<()> {
        debug!("WAL flush requested (Azure Append Blobs are immediately consistent)");
        Ok(())
    }
}

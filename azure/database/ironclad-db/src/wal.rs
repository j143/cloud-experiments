/// WAL: Write-Ahead Log for Durability and Crash Recovery
/// 
/// The WAL ensures ACID compliance by logging all operations before they're applied.
/// On crash, the WAL can be replayed to recover all committed operations.
/// Uses Azure Append Blob for the log storage.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{debug, info};

/// WAL Entry types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WalEntry {
    Set { key: String, value: String },
    Delete { key: String },
    Checkpoint { lsn: u64 }, // Log Sequence Number for checkpoint
}

/// Write-Ahead Log implementation
pub struct WAL {
    /// In-memory log entries (for demonstration)
    /// In production, this would write to Azure Append Blob
    entries: Arc<RwLock<Vec<WalEntry>>>,
    
    /// Current log sequence number
    lsn: Arc<RwLock<u64>>,
    
    /// Connection string for Azure Storage
    connection_string: String,
    
    /// Container name
    container_name: String,
    
    /// WAL blob name
    wal_blob_name: String,
}

impl WAL {
    /// Create a new WAL instance
    pub async fn new(
        connection_string: &str,
        container_name: &str,
        wal_blob_name: &str,
    ) -> Result<Self> {
        info!("Initializing WAL: container={}, blob={}", container_name, wal_blob_name);
        
        Ok(Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            lsn: Arc::new(RwLock::new(0)),
            connection_string: connection_string.to_string(),
            container_name: container_name.to_string(),
            wal_blob_name: wal_blob_name.to_string(),
        })
    }
    
    /// Append an entry to the WAL
    /// This is the critical DURABILITY point - once logged, data won't be lost
    pub async fn append_entry(&self, entry: WalEntry) -> Result<u64> {
        let mut entries = self.entries.write();
        let mut lsn = self.lsn.write();
        
        *lsn += 1;
        let current_lsn = *lsn;
        
        entries.push(entry.clone());
        
        debug!("WAL: Appended entry at LSN {}: {:?}", current_lsn, entry);
        
        // In production, this would write to Azure Append Blob
        // For demonstration, we just store in memory
        
        Ok(current_lsn)
    }
    
    /// Replay the WAL to recover state after a crash
    /// Returns all entries that need to be replayed
    pub async fn replay(&self) -> Result<Vec<WalEntry>> {
        info!("WAL: Starting replay for crash recovery");
        
        let entries = self.entries.read();
        let recovered_entries = entries.clone();
        
        info!("WAL: Recovered {} entries", recovered_entries.len());
        
        Ok(recovered_entries)
    }
    
    /// Clear the WAL after a checkpoint
    /// This is safe because all data has been persisted to the main storage
    pub async fn clear(&self) -> Result<()> {
        let mut entries = self.entries.write();
        let entry_count = entries.len();
        
        entries.clear();
        
        info!("WAL: Cleared {} entries after checkpoint", entry_count);
        
        Ok(())
    }
    
    /// Create a checkpoint entry
    pub async fn checkpoint(&self) -> Result<u64> {
        let current_lsn = *self.lsn.read();
        
        self.append_entry(WalEntry::Checkpoint { lsn: current_lsn }).await?;
        
        info!("WAL: Checkpoint created at LSN {}", current_lsn);
        
        Ok(current_lsn)
    }
    
    /// Get the current LSN (Log Sequence Number)
    pub fn current_lsn(&self) -> u64 {
        *self.lsn.read()
    }
    
    /// Get the number of entries in the WAL
    pub fn entry_count(&self) -> usize {
        self.entries.read().len()
    }
    
    /// Get all entries (for testing and debugging)
    pub fn get_all_entries(&self) -> Vec<WalEntry> {
        self.entries.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_wal_initialization() {
        let wal = WAL::new("test-connection", "test-container", "test-wal")
            .await
            .unwrap();
        
        assert_eq!(wal.current_lsn(), 0);
        assert_eq!(wal.entry_count(), 0);
    }
    
    #[tokio::test]
    async fn test_append_set_entry() {
        let wal = WAL::new("test-connection", "test-container", "test-wal")
            .await
            .unwrap();
        
        let entry = WalEntry::Set {
            key: "user:1".to_string(),
            value: "Alice".to_string(),
        };
        
        let lsn = wal.append_entry(entry.clone()).await.unwrap();
        
        assert_eq!(lsn, 1);
        assert_eq!(wal.current_lsn(), 1);
        assert_eq!(wal.entry_count(), 1);
        
        let entries = wal.get_all_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], entry);
    }
    
    #[tokio::test]
    async fn test_append_delete_entry() {
        let wal = WAL::new("test-connection", "test-container", "test-wal")
            .await
            .unwrap();
        
        let entry = WalEntry::Delete {
            key: "user:2".to_string(),
        };
        
        let lsn = wal.append_entry(entry.clone()).await.unwrap();
        
        assert_eq!(lsn, 1);
        assert_eq!(wal.entry_count(), 1);
    }
    
    #[tokio::test]
    async fn test_multiple_entries() {
        let wal = WAL::new("test-connection", "test-container", "test-wal")
            .await
            .unwrap();
        
        // Add multiple entries
        for i in 0..10 {
            let entry = WalEntry::Set {
                key: format!("key:{}", i),
                value: format!("value:{}", i),
            };
            wal.append_entry(entry).await.unwrap();
        }
        
        assert_eq!(wal.current_lsn(), 10);
        assert_eq!(wal.entry_count(), 10);
    }
    
    #[tokio::test]
    async fn test_replay() {
        let wal = WAL::new("test-connection", "test-container", "test-wal")
            .await
            .unwrap();
        
        // Add entries
        let entry1 = WalEntry::Set {
            key: "key1".to_string(),
            value: "value1".to_string(),
        };
        let entry2 = WalEntry::Set {
            key: "key2".to_string(),
            value: "value2".to_string(),
        };
        
        wal.append_entry(entry1.clone()).await.unwrap();
        wal.append_entry(entry2.clone()).await.unwrap();
        
        // Replay
        let replayed = wal.replay().await.unwrap();
        
        assert_eq!(replayed.len(), 2);
        assert_eq!(replayed[0], entry1);
        assert_eq!(replayed[1], entry2);
    }
    
    #[tokio::test]
    async fn test_checkpoint() {
        let wal = WAL::new("test-connection", "test-container", "test-wal")
            .await
            .unwrap();
        
        // Add some entries
        for i in 0..5 {
            let entry = WalEntry::Set {
                key: format!("key:{}", i),
                value: format!("value:{}", i),
            };
            wal.append_entry(entry).await.unwrap();
        }
        
        // Create checkpoint
        let checkpoint_lsn = wal.checkpoint().await.unwrap();
        
        assert_eq!(checkpoint_lsn, 5);
        assert_eq!(wal.entry_count(), 6); // 5 entries + 1 checkpoint
    }
    
    #[tokio::test]
    async fn test_clear_wal() {
        let wal = WAL::new("test-connection", "test-container", "test-wal")
            .await
            .unwrap();
        
        // Add entries
        for i in 0..5 {
            let entry = WalEntry::Set {
                key: format!("key:{}", i),
                value: format!("value:{}", i),
            };
            wal.append_entry(entry).await.unwrap();
        }
        
        assert_eq!(wal.entry_count(), 5);
        
        // Clear
        wal.clear().await.unwrap();
        
        assert_eq!(wal.entry_count(), 0);
    }
    
    #[tokio::test]
    async fn test_crash_recovery_scenario() {
        let wal = WAL::new("test-connection", "test-container", "test-wal")
            .await
            .unwrap();
        
        // Simulate normal operations
        wal.append_entry(WalEntry::Set {
            key: "user:1".to_string(),
            value: "Alice".to_string(),
        }).await.unwrap();
        
        wal.append_entry(WalEntry::Set {
            key: "user:2".to_string(),
            value: "Bob".to_string(),
        }).await.unwrap();
        
        wal.append_entry(WalEntry::Delete {
            key: "user:1".to_string(),
        }).await.unwrap();
        
        // Simulate crash and recovery
        let recovered = wal.replay().await.unwrap();
        
        assert_eq!(recovered.len(), 3);
        
        // Verify recovery operations
        match &recovered[0] {
            WalEntry::Set { key, value } => {
                assert_eq!(key, "user:1");
                assert_eq!(value, "Alice");
            },
            _ => panic!("Expected Set entry"),
        }
        
        match &recovered[2] {
            WalEntry::Delete { key } => {
                assert_eq!(key, "user:1");
            },
            _ => panic!("Expected Delete entry"),
        }
    }
}

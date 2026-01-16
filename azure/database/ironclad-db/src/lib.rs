/// Project IronClad - Azure Page Blob KV Store
/// 
/// A persistent, crash-safe Key-Value Store built on Azure Page Blobs.
/// Inspired by Azure SQL and Rubrik's internal architecture.

pub mod azure_disk;
pub mod buffer_pool;
pub mod wal;
pub mod kvstore;

// Re-export main types for convenience
pub use azure_disk::AzureDisk;
pub use buffer_pool::{BufferPool, BufferPoolStats};
pub use wal::{WAL, WalEntry};
pub use kvstore::{KVStore, KVStoreStats};

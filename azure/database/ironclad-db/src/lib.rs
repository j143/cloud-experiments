/// IronClad Database - Azure Page Blob KV Store
/// 
/// A crash-safe, ACID-compliant key-value store built on Azure Page Blobs.
/// 
/// Architecture:
/// 1. AzureDisk: Block device layer using Azure Page Blobs
/// 2. BufferPool: LRU-based memory management
/// 3. WriteAheadLog: Durability and crash recovery using Azure Append Blobs
/// 4. KVStore: High-level key-value interface with checksums

pub mod error;
pub mod azure_disk;
pub mod buffer_pool;
pub mod wal;
pub mod kvstore;

pub use error::{IronCladError, Result};
pub use kvstore::KVStore;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IronCladError {
    #[error("Page {page_id} not found")]
    PageNotFound { page_id: u64 },
    
    #[error("Buffer pool exhausted, cannot evict any pages (all pinned)")]
    BufferPoolExhausted,
    
    #[error("Page {page_id} checksum mismatch: expected {expected:x}, got {actual:x}")]
    ChecksumMismatch {
        page_id: u64,
        expected: u32,
        actual: u32,
    },
    
    #[error("Invalid page format: {reason}")]
    InvalidPageFormat { reason: String },
    
    #[error("Page {page_id} is currently pinned and cannot be evicted")]
    PagePinned { page_id: u64 },
    
    #[error("Key too large: {size} bytes (max: {max})")]
    KeyTooLarge { size: usize, max: usize },
    
    #[error("Value too large: {size} bytes (max: {max})")]
    ValueTooLarge { size: usize, max: usize },
    
    #[error("Azure storage error: {0}")]
    AzureStorageError(String),
    
    #[error("WAL corruption detected at LSN {lsn}")]
    WalCorruption { lsn: u64 },
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    
    #[error("Azure SDK error: {0}")]
    AzureError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, IronCladError>;

impl From<azure_core::Error> for IronCladError {
    fn from(err: azure_core::Error) -> Self {
        IronCladError::AzureError(err.to_string())
    }
}

impl From<azure_storage::Error> for IronCladError {
    fn from(err: azure_storage::Error) -> Self {
        IronCladError::AzureStorageError(err.to_string())
    }
}

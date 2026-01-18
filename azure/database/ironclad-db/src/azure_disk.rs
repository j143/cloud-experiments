/// AzureDisk: Pager Layer - Treats Azure Page Blobs as raw block devices
/// 
/// This layer provides a block device abstraction over Azure Page Blobs.
/// Each page is 4KB (4096 bytes) - standard database page size.
/// Operations are async due to network I/O.

use azure_storage_blobs::prelude::*;
use azure_storage::prelude::*;
use azure_storage::CloudLocation;
use crate::error::{IronCladError, Result};
use std::sync::Arc;
use tracing::{info, warn, debug};
use bytes::Bytes;
use futures::StreamExt;

pub const PAGE_SIZE: usize = 4096; // 4KB pages - standard database page size
const AZURE_PAGE_ALIGNMENT: usize = 512; // Azure Page Blobs require 512-byte alignment
const MAX_BLOB_SIZE: u64 = 1024 * 1024 * 1024 * 100; // 100GB initial size

/// AzureDisk provides a block device abstraction over Azure Page Blobs
pub struct AzureDisk {
    blob_client: Arc<BlobClient>,
}

impl AzureDisk {
    /// Create a new AzureDisk instance
    /// 
    /// # Arguments
    /// * `connection_string` - Azure Storage connection string
    /// * `container_name` - Container name for the page blob
    /// * `blob_name` - Name of the page blob (e.g., "db-data")
    pub async fn new(
        connection_string: &str,
        container_name: &str,
        blob_name: &str,
    ) -> Result<Self> {
        info!("Initializing AzureDisk: container={}, blob={}", container_name, blob_name);

        // Validate page size alignment
        if PAGE_SIZE % AZURE_PAGE_ALIGNMENT != 0 {
            return Err(IronCladError::ConfigError(
                format!("PAGE_SIZE ({}) must be multiple of Azure page alignment ({})", 
                    PAGE_SIZE, AZURE_PAGE_ALIGNMENT)
            ));
        }

        // Parse connection string and create clients; support Azurite via BlobEndpoint
        let account_name = connection_string.split(';')
            .find(|s| s.starts_with("AccountName="))
            .and_then(|s| s.strip_prefix("AccountName="))
            .ok_or_else(|| IronCladError::ConfigError("Missing AccountName in connection string".to_string()))?;
        
        let account_key = connection_string.split(';')
            .find(|s| s.starts_with("AccountKey="))
            .and_then(|s| s.strip_prefix("AccountKey="))
            .map(|s| s.to_string())
            .ok_or_else(|| IronCladError::ConfigError("Missing AccountKey in connection string".to_string()))?;

        let blob_endpoint = connection_string.split(';')
            .find(|s| s.starts_with("BlobEndpoint="))
            .and_then(|s| s.strip_prefix("BlobEndpoint="))
            .map(|s| s.to_string());

        let (credentials, cloud_location) = match blob_endpoint {
            Some(uri) => (
                StorageCredentials::emulator(),
                CloudLocation::Custom {
                    account: account_name.to_string(),
                    uri,
                },
            ),
            None => (
                StorageCredentials::access_key(account_name.to_string(), account_key),
                CloudLocation::Public {
                    account: account_name.to_string(),
                },
            ),
        };

        let blob_service_client = BlobServiceClient::builder(account_name, credentials)
            .cloud_location(cloud_location)
            .blob_service_client();

        let container_client = blob_service_client.container_client(container_name);

        // Create container if it doesn't exist
        match container_client.create().await {
            Ok(_) => info!("Created container: {}", container_name),
            Err(e) => {
                // Ignore "already exists" error
                debug!("Container creation response: {}", e);
            }
        }

        let blob_client = container_client.blob_client(blob_name);
        
        // Check if page blob exists, if not create it
        match blob_client.get_properties().await {
            Ok(_) => {
                info!("Page blob already exists: {}", blob_name);
            }
            Err(_) => {
                // Create page blob with initial size
                info!("Creating new page blob: {} with size {} bytes", blob_name, MAX_BLOB_SIZE);
                blob_client
                    .put_page_blob(MAX_BLOB_SIZE as u128)
                    .await
                    .map_err(|e| IronCladError::AzureError(e.to_string()))?;
                info!("Created page blob: {}", blob_name);
            }
        }

        Ok(Self {
            blob_client: Arc::new(blob_client),
        })
    }

    /// Read a page from Azure Page Blob
    /// 
    /// # Performance
    /// - Network latency: ~50-100ms for Azure
    /// - Should be cached in BufferPool to avoid repeated reads
    /// 
    /// # Errors
    /// Returns error if:
    /// - Azure read operation fails
    /// - Network issues
    /// - Page size mismatch
    pub async fn read_page(&self, page_id: u64) -> Result<Vec<u8>> {
        let offset = page_id * PAGE_SIZE as u64;
        let end = offset + PAGE_SIZE as u64 - 1;
        
        debug!("Reading page {} from offset {}", page_id, offset);
        
        // Retry logic with exponential backoff
        let mut attempts = 0;
        let max_attempts = 3;
        
        loop {
            let result = self.blob_client
                .get()
                .range(offset..end+1)
                .into_stream()
                .next()
                .await;
            
            match result {
                Some(Ok(response)) => {
                    // Collect the streaming body into bytes
                        let data_vec = response.data.collect().await
                            .map_err(|e| IronCladError::AzureStorageError(e.to_string()))?;
                    
                    if data_vec.len() != PAGE_SIZE {
                        return Err(IronCladError::InvalidPageFormat {
                            reason: format!("Expected {} bytes, got {}", PAGE_SIZE, data_vec.len())
                        });
                    }
                    
                    debug!("Successfully read page {}", page_id);
                    return Ok(data_vec.to_vec());
                }
                Some(Err(e)) if attempts < max_attempts => {
                    attempts += 1;
                    warn!("Read attempt {} failed for page {}: {}. Retrying...", 
                        attempts, page_id, e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempts as u64)).await;
                }
                Some(Err(ref err)) => {
                    return Err(IronCladError::AzureError(
                        format!("Failed to read page {} after {} attempts: {}", 
                            page_id, attempts, err)
                    ));
                }
                None => {
                    return Err(IronCladError::AzureError(
                        format!("Failed to read page {} after {} attempts: No data received", 
                            page_id, attempts)
                    ));
                }
            }
        }
    }

    /// Write a page to Azure Page Blob
    /// 
    /// # Performance
    /// - Network latency: ~50-100ms for Azure
    /// - Should be batched when possible
    /// 
    /// # Errors
    /// Returns error if:
    /// - Page size is not 4KB
    /// - Azure write operation fails
    /// - Network issues
    pub async fn write_page(&self, page_id: u64, data: &[u8]) -> Result<()> {
        if data.len() != PAGE_SIZE {
            return Err(IronCladError::ConfigError(
                format!("Page size must be {}, got {}", PAGE_SIZE, data.len())
            ));
        }

        let offset = page_id * PAGE_SIZE as u64;
        let end = offset + PAGE_SIZE as u64 - 1;
        let bytes = Bytes::copy_from_slice(data);
        
        debug!("Writing page {} at offset {}", page_id, offset);
        
        // Create BA512Range for page blob
        let ba512_range = BA512Range::new(offset, end)
            .map_err(|e| IronCladError::ConfigError(format!("Invalid range: {}", e)))?;
        
        // Retry logic with exponential backoff
        let mut attempts = 0;
        let max_attempts = 3;
        
        loop {
            match self.blob_client
                .put_page(ba512_range.clone(), bytes.clone())
                .await
            {
                Ok(_) => {
                    info!("Successfully wrote page {} to Azure at offset {}", page_id, offset);
                    return Ok(());
                }
                Err(e) if attempts < max_attempts => {
                    attempts += 1;
                    warn!("Write attempt {} failed for page {}: {}. Retrying...", 
                        attempts, page_id, e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempts as u64)).await;
                }
                Err(e) => {
                    return Err(IronCladError::AzureError(
                        format!("Failed to write page {} after {} attempts: {}", 
                            page_id, attempts, e)
                    ));
                }
            }
        }
    }

    /// Flush all pending writes
    /// Note: Azure Page Blobs are immediately consistent, so this is a no-op
    /// but we keep it for API compatibility
    pub async fn flush(&self) -> Result<()> {
        debug!("Flush called (Azure Page Blobs are immediately consistent)");
        Ok(())
    }

    /// Get the page size used by this disk
    pub fn page_size(&self) -> usize {
        PAGE_SIZE
    }
}

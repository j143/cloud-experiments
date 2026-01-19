/// AzureDisk: Pager Layer - Treats Azure Page Blobs as raw block devices
/// 
/// This layer provides a block device abstraction over Azure Page Blobs.
/// Each page is 4KB (4096 bytes) - standard database page size.
/// Operations are async due to network I/O.

use anyhow::Result;
use azure_storage::prelude::*;
use azure_storage_blobs::prelude::*;
use futures::StreamExt;
use std::sync::Arc;
use tracing::{debug, info};
use bytes::Bytes;

const PAGE_SIZE: usize = 4096; // 4KB pages - standard database page size
const BLOB_SIZE: usize = 1024 * 1024 * 1024; // 1GB total capacity

/// AzureDisk provides a block device abstraction over Azure Page Blobs
pub struct AzureDisk {
    blob_client: Arc<BlobClient>,
    container_name: String,
    blob_name: String,
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
        
        // Manual connection string parsing
        let mut account_name = String::new();
        let mut account_key = String::new();
        
        for part in connection_string.split(';') {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "AccountName" => account_name = value.to_string(),
                    "AccountKey" => account_key = value.to_string(),
                    _ => {}
                }
            }
        }
        
        if account_name.is_empty() || account_key.is_empty() {
             anyhow::bail!("Invalid connection string: missing AccountName or AccountKey");
        }
        
        let creds = StorageCredentials::access_key(account_name.clone(), account_key);
        let blob_service_client = BlobServiceClient::new(account_name, creds);
        let container_client = blob_service_client.container_client(container_name);
        
        // Ensure container exists
        if !container_client.exists().await? {
            info!("Creating container {}", container_name);
            container_client.create().await?;
        }
        
        let blob_client = container_client.blob_client(blob_name);
        
        // Ensure blob exists and is of correct size
        // Use put_page_blob for Page Blobs
        if !blob_client.exists().await? {
            info!("Creating page blob {} with size {} bytes", blob_name, BLOB_SIZE);
            blob_client.put_page_blob(BLOB_SIZE as u128).await?;
        }
        
        Ok(Self {
            blob_client: Arc::new(blob_client),
            container_name: container_name.to_string(),
            blob_name: blob_name.to_string(),
        })
    }
    
    /// Read a page from the blob storage
    /// 
    /// # Arguments
    /// * `page_id` - The page ID (0-indexed)
    /// 
    /// # Returns
    /// A 4KB byte array containing the page data
    pub async fn read_page(&self, page_id: u64) -> Result<Vec<u8>> {
        let offset = page_id * PAGE_SIZE as u64;
        
        debug!("Reading page {} from offset {}", page_id, offset);
        
        // We need to read PAGE_SIZE bytes at the calculated offset
        let range = offset..offset + PAGE_SIZE as u64;
        
        // Execute the request using into_stream()
        let mut stream = self.blob_client.get().range(range).into_stream();
        let mut data = Vec::with_capacity(PAGE_SIZE);
        
        while let Some(response_res) = stream.next().await {
            let response = response_res?;
            let mut body = response.data;
            while let Some(chunk_res) = body.next().await {
                let chunk: Bytes = chunk_res?;
                data.extend_from_slice(&chunk);
            }
        }

        if data.len() < PAGE_SIZE {
             data.resize(PAGE_SIZE, 0);
        }
        
        if data.len() > PAGE_SIZE {
            data.truncate(PAGE_SIZE);
        }
        
        Ok(data)
    }
    
    /// Write a page to the blob storage
    /// 
    /// # Arguments
    /// * `page_id` - The page ID (0-indexed)
    /// * `data` - The 4KB data to write
    pub async fn write_page(&self, page_id: u64, data: &[u8]) -> Result<()> {
        if data.len() != PAGE_SIZE {
            anyhow::bail!("Invalid page size: expected {}, got {}", PAGE_SIZE, data.len());
        }
        
        let offset = page_id * PAGE_SIZE as u64;
        
        debug!("Writing page {} at offset {}", page_id, offset);
        
        let bytes = Bytes::copy_from_slice(data);
        
        let range = BA512Range::new(offset, offset + PAGE_SIZE as u64)?;
        
        // update_pages takes u64 offset
        self.blob_client
            .put_page(range, bytes)
            .await?;
            
        Ok(())
    }
    
    /// Flush all pending writes to storage
    pub async fn flush(&self) -> Result<()> {
        debug!("Flushing all pending writes");
        // Direct writes to Azure Page Blob are durable upon success response.
        // No explicit flush needed for the client itself, as we await the calls.
        Ok(())
    }
    
    /// Get the page size (4KB)
    pub fn page_size(&self) -> usize {
        PAGE_SIZE
    }
    
    /// Get maximum number of pages
    pub fn max_pages(&self) -> u64 {
        (BLOB_SIZE / PAGE_SIZE) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_page_size() {
        assert_eq!(PAGE_SIZE, 4096);
    }
    
    #[test]
    fn test_page_calculations() {
        // Test offset calculations
        let page_0_offset = 0 * PAGE_SIZE as u64;
        let page_1_offset = 1 * PAGE_SIZE as u64;
        let page_100_offset = 100 * PAGE_SIZE as u64;
        
        assert_eq!(page_0_offset, 0);
        assert_eq!(page_1_offset, 4096);
        assert_eq!(page_100_offset, 409600);
    }
    
    // Check if we can run integration tests
    // Using simple conditional compilation or checking env var at runtime would be better
    // But for now, we'll just skip them if no connection string
}


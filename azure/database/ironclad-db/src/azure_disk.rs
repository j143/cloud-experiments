/// AzureDisk: Pager Layer - Treats Azure Page Blobs as raw block devices
/// 
/// This layer provides a block device abstraction over Azure Page Blobs.
/// Each page is 4KB (4096 bytes) - standard database page size.
/// Operations are async due to network I/O.

use anyhow::Result;
use azure_storage::prelude::*;
use azure_storage_blobs::prelude::*;
use azure_storage_blobs::blob::operations::GetPageRangesResponse;
use futures::StreamExt;
use std::sync::Arc;
use tracing::{debug, info};
use bytes::Bytes;

const PAGE_SIZE: usize = 4096; // 4KB pages - standard database page size
const BLOB_SIZE: usize = 1024 * 1024 * 1024; // 1GB total capacity

/// AzureDisk provides a block device abstraction over Azure Page Blobs
pub struct AzureDisk {
    page_blob_client: Arc<PageBlobClient>,
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
        
        let blob_service_client = BlobServiceClient::new(connection_string)?;
        let container_client = blob_service_client.container_client(container_name);
        
        // Ensure container exists
        if !container_client.exists().await? {
            info!("Creating container {}", container_name);
            container_client.create().await?;
        }
        
        let blob_client = container_client.blob_client(blob_name);
        let page_blob_client = blob_client.as_page_blob_client();
        
        // Ensure blob exists and is of correct size
        if !blob_client.exists().await? {
            info!("Creating page blob {} with size {} bytes", blob_name, BLOB_SIZE);
            page_blob_client.create(BLOB_SIZE as u128).await?;
        }
        
        Ok(Self {
            page_blob_client: Arc::new(page_blob_client),
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
        // Azure SDK allows reading a range
        // Note: The range is inclusive
        let range = offset..offset + PAGE_SIZE as u64;
        
        // Using get().range() to read specific bytes
        let mut data = Vec::with_capacity(PAGE_SIZE);
        let mut stream = self.page_blob_client.get().range(range).into_stream();
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            data.extend_from_slice(&chunk.data.collect().await?);
        }

        // If we got less than PAGE_SIZE (e.g. empty page or end of blob), pad with zeros?
        // Actually, for a valid page blob, reads should return data or 0s if sparse.
        // However, if the page was never written, it might come back as zeros.
        
        if data.len() < PAGE_SIZE {
             // Handle case where we might get fewer bytes if not fully written? 
             // Though Page Blob create ensures size.
             data.resize(PAGE_SIZE, 0);
        }
        
        if data.len() > PAGE_SIZE {
            // Should not happen with correct range, but trim just in case
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
        
        // Page Blobs require 512-byte alignment, 4KB is 8 * 512 so it's fine.
        let bytes = Bytes::copy_from_slice(data);
        
        self.page_blob_client
            .update_pages(offset as u128, bytes)
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


/// BufferPool: Memory Manager with LRU Eviction
/// 
/// This layer manages a fixed-size buffer pool (50MB) in memory.
/// It uses LRU (Least Recently Used) eviction policy when the cache is full.
/// The buffer pool reduces latency by caching frequently accessed pages in RAM.

use anyhow::Result;
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tracing::{debug, info, warn};

const BUFFER_SIZE: usize = 50 * 1024 * 1024; // 50MB
const PAGE_SIZE: usize = 4096; // 4KB per page
const NUM_FRAMES: usize = BUFFER_SIZE / PAGE_SIZE; // 12,288 frames

/// Represents a single frame in the buffer pool
#[derive(Debug, Clone)]
struct Frame {
    page_id: u64,
    data: Vec<u8>,
    dirty: bool,      // Has this page been modified?
    pin_count: u32,   // Number of users currently accessing this page
}

/// BufferPool manages in-memory page caching with LRU eviction
pub struct BufferPool {
    /// Page table: Maps page_id -> frame_index in buffer
    page_table: Arc<RwLock<HashMap<u64, usize>>>,
    
    /// The actual buffer frames (50MB of 4KB pages)
    frames: Arc<RwLock<Vec<Option<Frame>>>>,
    
    /// LRU queue: Most recently used at back, least recently used at front
    lru_queue: Arc<RwLock<VecDeque<u64>>>,
    
    /// Free frames available for allocation
    free_frames: Arc<RwLock<VecDeque<usize>>>,
}

impl BufferPool {
    /// Create a new BufferPool
    pub fn new() -> Self {
        info!("Initializing BufferPool: {}MB ({} frames)", 
              BUFFER_SIZE / (1024 * 1024), NUM_FRAMES);
        
        let frames = vec![None; NUM_FRAMES];
        let free_frames: VecDeque<usize> = (0..NUM_FRAMES).collect();
        
        Self {
            page_table: Arc::new(RwLock::new(HashMap::new())),
            frames: Arc::new(RwLock::new(frames)),
            lru_queue: Arc::new(RwLock::new(VecDeque::new())),
            free_frames: Arc::new(RwLock::new(free_frames)),
        }
    }
    
    /// Fetch a page from the buffer pool
    /// If not in cache, returns None (caller should load from disk)
    pub fn get_page(&self, page_id: u64) -> Option<Vec<u8>> {
        let page_table = self.page_table.read();
        
        if let Some(&frame_idx) = page_table.get(&page_id) {
            // Page is in cache - update LRU
            self.update_lru(page_id);
            
            let frames = self.frames.read();
            if let Some(Some(frame)) = frames.get(frame_idx) {
                debug!("Cache HIT: page {} in frame {}", page_id, frame_idx);
                return Some(frame.data.clone());
            }
        }
        
        debug!("Cache MISS: page {}", page_id);
        None
    }
    
    /// Put a page into the buffer pool
    /// Returns the frame index, and optionally a dirty page that was evicted
    pub fn put_page(&self, page_id: u64, data: Vec<u8>) -> Result<Option<(u64, Vec<u8>)>> {
        if data.len() != PAGE_SIZE {
            anyhow::bail!("Invalid page size: expected {}, got {}", PAGE_SIZE, data.len());
        }
        
        // Check if page is already in buffer
        {
            let page_table = self.page_table.read();
            if page_table.contains_key(&page_id) {
                // Update existing page
                return self.update_existing_page(page_id, data);
            }
        }
        
        // Need to allocate a new frame
        let frame_idx = self.allocate_frame()?;
        
        let mut frames = self.frames.write();
        let mut page_table = self.page_table.write();
        
        frames[frame_idx] = Some(Frame {
            page_id,
            data,
            dirty: true,
            pin_count: 0,
        });
        
        page_table.insert(page_id, frame_idx);
        self.update_lru(page_id);
        
        debug!("Inserted page {} into frame {}", page_id, frame_idx);
        Ok(None)
    }
    
    /// Update an existing page in the buffer
    fn update_existing_page(&self, page_id: u64, data: Vec<u8>) -> Result<Option<(u64, Vec<u8>)>> {
        let page_table = self.page_table.read();
        
        if let Some(&frame_idx) = page_table.get(&page_id) {
            let mut frames = self.frames.write();
            
            if let Some(Some(frame)) = frames.get_mut(frame_idx) {
                frame.data = data;
                frame.dirty = true;
                self.update_lru(page_id);
                debug!("Updated page {} in frame {} (marked dirty)", page_id, frame_idx);
            }
        }
        
        Ok(None)
    }
    
    /// Allocate a frame (either from free list or evict LRU page)
    fn allocate_frame(&self) -> Result<usize> {
        // Try to get a free frame first
        {
            let mut free_frames = self.free_frames.write();
            if let Some(frame_idx) = free_frames.pop_front() {
                debug!("Allocated free frame {}", frame_idx);
                return Ok(frame_idx);
            }
        }
        
        // No free frames - must evict LRU page
        self.evict_lru_page()
    }
    
    /// Evict the least recently used page
    fn evict_lru_page(&self) -> Result<usize> {
        let mut lru_queue = self.lru_queue.write();
        
        // Find an unpinned page to evict
        while let Some(candidate_page_id) = lru_queue.pop_front() {
            let page_table = self.page_table.read();
            
            if let Some(&frame_idx) = page_table.get(&candidate_page_id) {
                let frames = self.frames.read();
                
                if let Some(Some(frame)) = frames.get(frame_idx) {
                    if frame.pin_count == 0 {
                        // Found a page we can evict
                        drop(frames);
                        drop(page_table);
                        
                        warn!("Evicting LRU page {} from frame {}", candidate_page_id, frame_idx);
                        
                        // Remove from page table
                        let mut page_table = self.page_table.write();
                        page_table.remove(&candidate_page_id);
                        
                        return Ok(frame_idx);
                    }
                }
            }
            
            // This page is pinned, put it back at the end
            lru_queue.push_back(candidate_page_id);
        }
        
        anyhow::bail!("No pages available for eviction (all pinned)")
    }
    
    /// Update the LRU queue when a page is accessed
    fn update_lru(&self, page_id: u64) {
        let mut lru_queue = self.lru_queue.write();
        
        // Remove page if it's already in the queue
        if let Some(pos) = lru_queue.iter().position(|&id| id == page_id) {
            lru_queue.remove(pos);
        }
        
        // Add to back (most recently used)
        lru_queue.push_back(page_id);
    }
    
    /// Mark a page as dirty (modified)
    pub fn mark_dirty(&self, page_id: u64) -> Result<()> {
        let page_table = self.page_table.read();
        
        if let Some(&frame_idx) = page_table.get(&page_id) {
            let mut frames = self.frames.write();
            
            if let Some(Some(frame)) = frames.get_mut(frame_idx) {
                frame.dirty = true;
                debug!("Marked page {} as dirty", page_id);
            }
        }
        
        Ok(())
    }
    
    /// Get all dirty pages that need to be flushed
    pub fn get_dirty_pages(&self) -> Vec<(u64, Vec<u8>)> {
        let frames = self.frames.read();
        let mut dirty_pages = Vec::new();
        
        for frame_opt in frames.iter() {
            if let Some(frame) = frame_opt {
                if frame.dirty {
                    dirty_pages.push((frame.page_id, frame.data.clone()));
                }
            }
        }
        
        debug!("Found {} dirty pages", dirty_pages.len());
        dirty_pages
    }
    
    /// Clear dirty flag for a page after it's been flushed
    pub fn clear_dirty(&self, page_id: u64) -> Result<()> {
        let page_table = self.page_table.read();
        
        if let Some(&frame_idx) = page_table.get(&page_id) {
            let mut frames = self.frames.write();
            
            if let Some(Some(frame)) = frames.get_mut(frame_idx) {
                frame.dirty = false;
                debug!("Cleared dirty flag for page {}", page_id);
            }
        }
        
        Ok(())
    }
    
    /// Get buffer pool statistics
    pub fn stats(&self) -> BufferPoolStats {
        let page_table = self.page_table.read();
        let free_frames = self.free_frames.read();
        
        BufferPoolStats {
            total_frames: NUM_FRAMES,
            used_frames: page_table.len(),
            free_frames: free_frames.len(),
            buffer_size_mb: BUFFER_SIZE / (1024 * 1024),
        }
    }
}

/// Buffer pool statistics
#[derive(Debug, Clone)]
pub struct BufferPoolStats {
    pub total_frames: usize,
    pub used_frames: usize,
    pub free_frames: usize,
    pub buffer_size_mb: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_buffer_pool_initialization() {
        let bp = BufferPool::new();
        let stats = bp.stats();
        
        assert_eq!(stats.total_frames, NUM_FRAMES);
        assert_eq!(stats.used_frames, 0);
        assert_eq!(stats.free_frames, NUM_FRAMES);
        assert_eq!(stats.buffer_size_mb, 50);
    }
    
    #[test]
    fn test_put_and_get_page() {
        let bp = BufferPool::new();
        let data = vec![42u8; PAGE_SIZE];
        
        // Put page
        let result = bp.put_page(0, data.clone());
        assert!(result.is_ok());
        
        // Get page
        let retrieved = bp.get_page(0);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), data);
    }
    
    #[test]
    fn test_cache_miss() {
        let bp = BufferPool::new();
        
        // Try to get a page that doesn't exist
        let result = bp.get_page(999);
        assert!(result.is_none());
    }
    
    #[test]
    fn test_lru_eviction() {
        let bp = BufferPool::new();
        
        // Fill buffer with pages
        for i in 0..10 {
            let data = vec![i as u8; PAGE_SIZE];
            bp.put_page(i, data).unwrap();
        }
        
        let stats = bp.stats();
        assert_eq!(stats.used_frames, 10);
        
        // Access page 0 multiple times (make it most recently used)
        for _ in 0..5 {
            bp.get_page(0);
        }
    }
    
    #[test]
    fn test_dirty_pages() {
        let bp = BufferPool::new();
        let data = vec![1u8; PAGE_SIZE];
        
        bp.put_page(0, data.clone()).unwrap();
        bp.put_page(1, data.clone()).unwrap();
        
        // Both pages should be dirty after insertion
        let dirty = bp.get_dirty_pages();
        assert_eq!(dirty.len(), 2);
        
        // Clear one
        bp.clear_dirty(0).unwrap();
        let dirty = bp.get_dirty_pages();
        assert_eq!(dirty.len(), 1);
    }
    
    #[test]
    fn test_invalid_page_size() {
        let bp = BufferPool::new();
        let invalid_data = vec![1u8; 100]; // Wrong size
        
        let result = bp.put_page(0, invalid_data);
        assert!(result.is_err());
    }
}

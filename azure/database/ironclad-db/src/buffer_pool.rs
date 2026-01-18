/// Buffer Pool: LRU-based memory management for database pages
/// 
/// Manages a fixed-size pool of in-memory page frames with LRU eviction.
/// Handles page pinning/unpinning and dirty page tracking.

use crate::azure_disk::{AzureDisk, PAGE_SIZE};
use crate::error::{IronCladError, Result};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, warn, debug};

const BUFFER_SIZE: usize = 50 * 1024 * 1024; // 50MB buffer pool
const NUM_FRAMES: usize = BUFFER_SIZE / PAGE_SIZE; // ~12,800 frames

#[derive(Debug, Clone)]
struct Frame {
    data: Vec<u8>,
    is_dirty: bool,
    pin_count: u32,
}

pub struct BufferPool {
    /// The underlying disk layer
    disk: Arc<AzureDisk>,
    /// Frame storage - fixed size array
    frames: Arc<RwLock<Vec<Option<Frame>>>>,
    /// Page ID to frame index mapping
    page_table: Arc<RwLock<HashMap<u64, usize>>>,
    /// LRU queue for eviction
    lru_queue: Arc<RwLock<VecDeque<u64>>>,
}

impl BufferPool {
    pub fn new(disk: Arc<AzureDisk>) -> Self {
        info!("Initializing buffer pool: {} frames ({} MB)", 
            NUM_FRAMES, BUFFER_SIZE / (1024 * 1024));

        let mut frames = Vec::with_capacity(NUM_FRAMES);
        for _ in 0..NUM_FRAMES {
            frames.push(None);
        }

        Self {
            disk,
            frames: Arc::new(RwLock::new(frames)),
            page_table: Arc::new(RwLock::new(HashMap::new())),
            lru_queue: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Get a page from the buffer pool
    /// Returns None if page doesn't exist
    pub async fn get_page(&self, page_id: u64) -> Result<Option<Vec<u8>>> {
        // Check if page is in buffer
        {
            let page_table = self.page_table.read();
            if let Some(&frame_idx) = page_table.get(&page_id) {
                let frames = self.frames.read();
                if let Some(Some(frame)) = frames.get(frame_idx) {
                    debug!("Buffer hit for page {}", page_id);
                    self.update_lru(page_id);
                    return Ok(Some(frame.data.clone()));
                }
            }
        }

        // Page not in buffer - need to load from disk
        debug!("Buffer miss for page {}", page_id);
        
        // Try to read from disk
        match self.disk.read_page(page_id).await {
            Ok(data) => {
                // Page exists, load it into buffer
                self.load_page_into_buffer(page_id, data.clone()).await?;
                Ok(Some(data))
            }
            Err(IronCladError::PageNotFound { .. }) | 
            Err(IronCladError::InvalidPageFormat { .. }) => {
                // Page doesn't exist or is invalid
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Load a page into the buffer pool
    async fn load_page_into_buffer(&self, page_id: u64, data: Vec<u8>) -> Result<()> {
        // Find a free frame or evict one
        let frame_idx = self.get_free_frame().await?;

        // Load page into frame
        let mut frames = self.frames.write();
        frames[frame_idx] = Some(Frame {
            data,
            is_dirty: false,
            pin_count: 1, // Pin it since we're using it
        });

        // Update page table
        let mut page_table = self.page_table.write();
        page_table.insert(page_id, frame_idx);

        // Update LRU
        self.update_lru(page_id);

        Ok(())
    }

    /// Write a page to the buffer pool (marks it dirty)
    pub async fn put_page(&self, page_id: u64, data: Vec<u8>) -> Result<()> {
        if data.len() != PAGE_SIZE {
            return Err(IronCladError::ConfigError(
                format!("Page size must be {}, got {}", PAGE_SIZE, data.len())
            ));
        }

        // Check if page is already in buffer
        let frame_idx = {
            let page_table = self.page_table.read();
            page_table.get(&page_id).copied()
        };

        match frame_idx {
            Some(idx) => {
                // Update existing frame
                let mut frames = self.frames.write();
                if let Some(Some(frame)) = frames.get_mut(idx) {
                    frame.data = data;
                    frame.is_dirty = true;
                    self.update_lru(page_id);
                }
            }
            None => {
                // Load into buffer
                let idx = self.get_free_frame().await?;
                let mut frames = self.frames.write();
                frames[idx] = Some(Frame {
                    data,
                    is_dirty: true,
                    pin_count: 1,
                });

                let mut page_table = self.page_table.write();
                page_table.insert(page_id, idx);
                self.update_lru(page_id);
            }
        }

        Ok(())
    }

    /// Pin a page to prevent it from being evicted
    pub fn pin_page(&self, page_id: u64) -> Result<()> {
        let page_table = self.page_table.read();
        if let Some(&frame_idx) = page_table.get(&page_id) {
            let mut frames = self.frames.write();
            if let Some(Some(frame)) = frames.get_mut(frame_idx) {
                frame.pin_count += 1;
                debug!("Pinned page {}, pin_count now {}", page_id, frame.pin_count);
                return Ok(());
            }
        }
        Err(IronCladError::PageNotFound { page_id })
    }

    /// Unpin a page to allow it to be evicted
    pub fn unpin_page(&self, page_id: u64) -> Result<()> {
        let page_table = self.page_table.read();
        if let Some(&frame_idx) = page_table.get(&page_id) {
            let mut frames = self.frames.write();
            if let Some(Some(frame)) = frames.get_mut(frame_idx) {
                if frame.pin_count > 0 {
                    frame.pin_count -= 1;
                    debug!("Unpinned page {}, pin_count now {}", page_id, frame.pin_count);
                }
                return Ok(());
            }
        }
        Err(IronCladError::PageNotFound { page_id })
    }

    /// Get a free frame, evicting if necessary
    async fn get_free_frame(&self) -> Result<usize> {
        // First try to find an empty frame
        {
            let frames = self.frames.read();
            for (idx, frame) in frames.iter().enumerate() {
                if frame.is_none() {
                    debug!("Found empty frame {}", idx);
                    return Ok(idx);
                }
            }
        }

        // All frames occupied, need to evict
        self.evict_page().await
    }

    /// Evict a page using LRU policy
    async fn evict_page(&self) -> Result<usize> {
        let mut lru_queue = self.lru_queue.write();
        let mut attempts = 0;
        let max_attempts = lru_queue.len() + 1; // Prevent infinite loop

        while let Some(candidate_page_id) = lru_queue.pop_front() {
            attempts += 1;
            
            if attempts > max_attempts {
                return Err(IronCladError::BufferPoolExhausted);
            }

            // Check if this page can be evicted
            let page_table = self.page_table.read();
            if let Some(&frame_idx) = page_table.get(&candidate_page_id) {
                let frames = self.frames.read();
                if let Some(Some(frame)) = frames.get(frame_idx) {
                    if frame.pin_count == 0 {
                        // This page can be evicted
                        drop(frames);
                        drop(page_table);
                        
                        // Write back if dirty BEFORE evicting
                        let is_dirty = {
                            let frames = self.frames.read();
                            frames[frame_idx].as_ref().map(|f| f.is_dirty).unwrap_or(false)
                        };

                        if is_dirty {
                            info!("Writing back dirty page {} before eviction", candidate_page_id);
                            let data = {
                                let frames = self.frames.read();
                                frames[frame_idx].as_ref().unwrap().data.clone()
                            };
                            
                            // CRITICAL: Write to disk before evicting
                            self.disk.write_page(candidate_page_id, &data).await?;
                        }

                        // Now evict the page
                        warn!("Evicting LRU page {} from frame {}", candidate_page_id, frame_idx);
                        let mut page_table = self.page_table.write();
                        page_table.remove(&candidate_page_id);
                        
                        return Ok(frame_idx);
                    } else {
                        // Page is pinned, put it back at the end
                        lru_queue.push_back(candidate_page_id);
                    }
                }
            }
        }

        Err(IronCladError::BufferPoolExhausted)
    }

    /// Update LRU queue
    fn update_lru(&self, page_id: u64) {
        let mut lru_queue = self.lru_queue.write();
        
        // Remove from current position
        if let Some(pos) = lru_queue.iter().position(|&id| id == page_id) {
            lru_queue.remove(pos);
        }
        
        // Add to back (most recently used)
        lru_queue.push_back(page_id);
    }

    /// Get all dirty pages (for flushing)
    pub fn get_dirty_pages(&self) -> Vec<(u64, Vec<u8>)> {
        let mut dirty = Vec::new();
        let page_table = self.page_table.read();
        let frames = self.frames.read();

        for (page_id, frame_idx) in page_table.iter() {
            if let Some(Some(frame)) = frames.get(*frame_idx) {
                if frame.is_dirty {
                    dirty.push((*page_id, frame.data.clone()));
                }
            }
        }

        debug!("Found {} dirty pages", dirty.len());
        dirty
    }

    /// Clear dirty flag for a page
    pub fn clear_dirty(&self, page_id: u64) -> Result<()> {
        let page_table = self.page_table.read();
        if let Some(&frame_idx) = page_table.get(&page_id) {
            let mut frames = self.frames.write();
            if let Some(Some(frame)) = frames.get_mut(frame_idx) {
                frame.is_dirty = false;
                return Ok(());
            }
        }
        Err(IronCladError::PageNotFound { page_id })
    }

    /// Get buffer pool statistics
    pub fn stats(&self) -> BufferPoolStats {
        let frames = self.frames.read();
        let page_table = self.page_table.read();
        
        let mut used_frames = 0;
        let mut dirty_pages = 0;
        let mut pinned_pages = 0;

        for frame in frames.iter() {
            if let Some(f) = frame {
                used_frames += 1;
                if f.is_dirty {
                    dirty_pages += 1;
                }
                if f.pin_count > 0 {
                    pinned_pages += 1;
                }
            }
        }

        BufferPoolStats {
            total_frames: NUM_FRAMES,
            used_frames,
            dirty_pages,
            pinned_pages,
            page_table_size: page_table.len(),
        }
    }
}

#[derive(Debug)]
pub struct BufferPoolStats {
    pub total_frames: usize,
    pub used_frames: usize,
    pub dirty_pages: usize,
    pub pinned_pages: usize,
    pub page_table_size: usize,
}

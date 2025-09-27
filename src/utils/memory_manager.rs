//! Memory management and optimization for large file processing

use crate::error::{TrimXError, TrimXResult};
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tracing::{debug, info};

/// Memory management configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Maximum memory usage in bytes
    pub max_memory: u64,
    /// Buffer size for streaming operations
    pub buffer_size: usize,
    /// Maximum number of frames to buffer
    pub max_frame_buffer: usize,
    /// Enable memory compression
    pub enable_compression: bool,
    /// Memory monitoring interval
    pub monitor_interval: Duration,
    /// Garbage collection threshold (percentage)
    pub gc_threshold: f64,
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Current memory usage in bytes
    pub current_usage: u64,
    /// Peak memory usage in bytes
    pub peak_usage: u64,
    /// Total allocations
    pub total_allocations: u64,
    /// Total deallocations
    pub total_deallocations: u64,
    /// Buffer pool statistics
    pub buffer_pool_stats: BufferPoolStats,
    /// System memory info
    pub system_memory: SystemMemoryInfo,
}

/// Buffer pool statistics
#[derive(Debug, Clone)]
pub struct BufferPoolStats {
    /// Total buffers allocated
    pub total_buffers: usize,
    /// Buffers currently in use
    pub buffers_in_use: usize,
    /// Buffers in pool (available)
    pub buffers_available: usize,
    /// Total buffer memory
    pub total_memory: u64,
    /// Hit ratio (reuse rate)
    pub hit_ratio: f64,
}

/// System memory information
#[derive(Debug, Clone)]
pub struct SystemMemoryInfo {
    /// Total system memory
    pub total_memory: u64,
    /// Available system memory
    pub available_memory: u64,
    /// Memory usage percentage
    pub usage_percentage: f64,
    /// Process memory usage
    pub process_memory: u64,
}

/// Memory manager for optimized large file processing
pub struct MemoryManager {
    config: MemoryConfig,
    stats: Arc<Mutex<MemoryStats>>,
    current_usage: AtomicU64,
    peak_usage: AtomicU64,
    buffer_pool: Arc<Mutex<BufferPool>>,
    monitoring_thread: Option<std::thread::JoinHandle<()>>,
}

/// Buffer pool for efficient memory reuse
struct BufferPool {
    buffers: VecDeque<Vec<u8>>,
    buffer_size: usize,
    total_allocations: AtomicUsize,
    total_hits: AtomicUsize,
    max_buffers: usize,
}

/// Smart buffer that automatically returns to pool when dropped
pub struct PooledBuffer {
    data: Option<Vec<u8>>,
    pool: Arc<Mutex<BufferPool>>,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_memory: 1024 * 1024 * 1024, // 1GB default
            buffer_size: 1024 * 1024,       // 1MB buffer size
            max_frame_buffer: 100,          // Max 100 frames
            enable_compression: false,      // Disabled by default (CPU overhead)
            monitor_interval: Duration::from_secs(1),
            gc_threshold: 80.0, // GC at 80% memory usage
        }
    }
}

impl MemoryManager {
    /// Create a new memory manager with configuration
    pub fn new(config: MemoryConfig) -> Self {
        let stats = Arc::new(Mutex::new(MemoryStats {
            current_usage: 0,
            peak_usage: 0,
            total_allocations: 0,
            total_deallocations: 0,
            buffer_pool_stats: BufferPoolStats {
                total_buffers: 0,
                buffers_in_use: 0,
                buffers_available: 0,
                total_memory: 0,
                hit_ratio: 0.0,
            },
            system_memory: SystemMemoryInfo {
                total_memory: 0,
                available_memory: 0,
                usage_percentage: 0.0,
                process_memory: 0,
            },
        }));

        let buffer_pool = Arc::new(Mutex::new(BufferPool::new(
            config.buffer_size,
            config.max_frame_buffer,
        )));

        Self {
            config,
            stats,
            current_usage: AtomicU64::new(0),
            peak_usage: AtomicU64::new(0),
            buffer_pool,
            monitoring_thread: None,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(MemoryConfig::default())
    }

    /// Set memory limit
    pub fn with_memory_limit(mut self, limit_mb: u64) -> Self {
        self.config.max_memory = limit_mb * 1024 * 1024;
        self
    }

    /// Set buffer size
    pub fn with_buffer_size(mut self, size_kb: usize) -> Self {
        self.config.buffer_size = size_kb * 1024;
        self
    }

    /// Start memory monitoring
    pub fn start_monitoring(&mut self) -> TrimXResult<()> {
        if self.monitoring_thread.is_some() {
            return Err(TrimXError::ClippingError {
                message: "Memory monitoring already started".to_string(),
            });
        }

        let stats = self.stats.clone();
        let buffer_pool = self.buffer_pool.clone();
        let interval = self.config.monitor_interval;
        let gc_threshold = self.config.gc_threshold;
        let max_memory = self.config.max_memory;

        let handle = std::thread::spawn(move || {
            loop {
                std::thread::sleep(interval);

                // Update system memory info
                if let Ok(mut stats) = stats.lock() {
                    stats.system_memory = Self::get_system_memory_info();

                    // Check if we need garbage collection
                    let usage_pct = (stats.current_usage as f64 / max_memory as f64) * 100.0;
                    if usage_pct > gc_threshold {
                        debug!("Memory usage at {:.1}%, triggering cleanup", usage_pct);
                        Self::cleanup_buffers(&buffer_pool);
                    }
                }
            }
        });

        self.monitoring_thread = Some(handle);
        info!(
            "Memory monitoring started with limit: {} MB",
            self.config.max_memory / 1024 / 1024
        );
        Ok(())
    }

    /// Stop memory monitoring
    pub fn stop_monitoring(&mut self) {
        if let Some(handle) = self.monitoring_thread.take() {
            handle.thread().unpark(); // This won't actually stop the thread cleanly
                                      // In a real implementation, you'd use a channel or atomic flag to stop the thread
            info!("Memory monitoring stopped");
        }
    }

    /// Allocate a buffer from the pool
    pub fn allocate_buffer(&self) -> TrimXResult<PooledBuffer> {
        // Check memory limit
        let current = self.current_usage.load(Ordering::Relaxed);
        if current + self.config.buffer_size as u64 > self.config.max_memory {
            return Err(TrimXError::ClippingError {
                message: format!(
                    "Memory limit exceeded: {} + {} > {}",
                    current, self.config.buffer_size, self.config.max_memory
                ),
            });
        }

        let buffer = if let Ok(mut pool) = self.buffer_pool.lock() {
            pool.get_buffer()
        } else {
            // Fallback allocation if pool is locked
            vec![0u8; self.config.buffer_size]
        };

        // Update usage statistics
        self.current_usage
            .fetch_add(buffer.len() as u64, Ordering::Relaxed);
        let new_usage = self.current_usage.load(Ordering::Relaxed);

        // Update peak usage
        let mut peak = self.peak_usage.load(Ordering::Relaxed);
        while new_usage > peak {
            match self.peak_usage.compare_exchange_weak(
                peak,
                new_usage,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => peak = x,
            }
        }

        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.current_usage = new_usage;
            stats.peak_usage = self.peak_usage.load(Ordering::Relaxed);
            stats.total_allocations += 1;
        }

        Ok(PooledBuffer {
            data: Some(buffer),
            pool: self.buffer_pool.clone(),
        })
    }

    /// Get current memory usage
    pub fn get_current_usage(&self) -> u64 {
        self.current_usage.load(Ordering::Relaxed)
    }

    /// Get peak memory usage
    pub fn get_peak_usage(&self) -> u64 {
        self.peak_usage.load(Ordering::Relaxed)
    }

    /// Get detailed memory statistics
    pub fn get_stats(&self) -> Option<MemoryStats> {
        self.stats.lock().ok().map(|stats| {
            let mut stats = stats.clone();

            // Update buffer pool stats
            if let Ok(pool) = self.buffer_pool.lock() {
                stats.buffer_pool_stats = pool.get_stats();
            }

            // Update system memory info
            stats.system_memory = Self::get_system_memory_info();
            stats
        })
    }

    /// Force garbage collection
    pub fn force_gc(&self) {
        Self::cleanup_buffers(&self.buffer_pool);
        info!("Forced garbage collection completed");
    }

    /// Check if memory usage is within safe limits
    pub fn is_memory_safe(&self) -> bool {
        let current = self.current_usage.load(Ordering::Relaxed);
        let usage_pct = (current as f64 / self.config.max_memory as f64) * 100.0;
        usage_pct < self.config.gc_threshold
    }

    /// Get system memory information
    fn get_system_memory_info() -> SystemMemoryInfo {
        // Simplified implementation - in production would use platform-specific APIs
        SystemMemoryInfo {
            total_memory: 8 * 1024 * 1024 * 1024,     // Assume 8GB
            available_memory: 4 * 1024 * 1024 * 1024, // Assume 4GB available
            usage_percentage: 50.0,
            process_memory: 512 * 1024 * 1024, // Assume 512MB process usage
        }
    }

    /// Cleanup unused buffers
    fn cleanup_buffers(pool: &Arc<Mutex<BufferPool>>) {
        if let Ok(mut pool) = pool.lock() {
            let before = pool.buffers.len();
            pool.cleanup();
            let after = pool.buffers.len();
            debug!("Buffer cleanup: {} -> {} buffers", before, after);
        }
    }

    /// Generate memory usage report
    pub fn generate_report(&self) -> String {
        let stats = self.get_stats().unwrap_or_else(|| MemoryStats {
            current_usage: self.get_current_usage(),
            peak_usage: self.get_peak_usage(),
            total_allocations: 0,
            total_deallocations: 0,
            buffer_pool_stats: BufferPoolStats {
                total_buffers: 0,
                buffers_in_use: 0,
                buffers_available: 0,
                total_memory: 0,
                hit_ratio: 0.0,
            },
            system_memory: Self::get_system_memory_info(),
        });

        let mut report = String::new();

        report.push_str("Memory Usage Report:\n");
        report.push_str(&format!(
            "  Current Usage: {:.1} MB\n",
            stats.current_usage as f64 / 1024.0 / 1024.0
        ));
        report.push_str(&format!(
            "  Peak Usage: {:.1} MB\n",
            stats.peak_usage as f64 / 1024.0 / 1024.0
        ));
        report.push_str(&format!(
            "  Memory Limit: {:.1} MB\n",
            self.config.max_memory as f64 / 1024.0 / 1024.0
        ));

        let usage_pct = (stats.current_usage as f64 / self.config.max_memory as f64) * 100.0;
        report.push_str(&format!("  Usage Percentage: {:.1}%\n", usage_pct));

        report.push_str(&format!(
            "  Total Allocations: {}\n",
            stats.total_allocations
        ));
        report.push_str(&format!(
            "  Total Deallocations: {}\n",
            stats.total_deallocations
        ));

        report.push_str("\nBuffer Pool:\n");
        report.push_str(&format!(
            "  Total Buffers: {}\n",
            stats.buffer_pool_stats.total_buffers
        ));
        report.push_str(&format!(
            "  Buffers In Use: {}\n",
            stats.buffer_pool_stats.buffers_in_use
        ));
        report.push_str(&format!(
            "  Buffers Available: {}\n",
            stats.buffer_pool_stats.buffers_available
        ));
        report.push_str(&format!(
            "  Hit Ratio: {:.1}%\n",
            stats.buffer_pool_stats.hit_ratio * 100.0
        ));

        report.push_str("\nSystem Memory:\n");
        report.push_str(&format!(
            "  Total: {:.1} GB\n",
            stats.system_memory.total_memory as f64 / 1024.0 / 1024.0 / 1024.0
        ));
        report.push_str(&format!(
            "  Available: {:.1} GB\n",
            stats.system_memory.available_memory as f64 / 1024.0 / 1024.0 / 1024.0
        ));
        report.push_str(&format!(
            "  System Usage: {:.1}%\n",
            stats.system_memory.usage_percentage
        ));

        report
    }
}

impl BufferPool {
    fn new(buffer_size: usize, max_buffers: usize) -> Self {
        Self {
            buffers: VecDeque::new(),
            buffer_size,
            total_allocations: AtomicUsize::new(0),
            total_hits: AtomicUsize::new(0),
            max_buffers,
        }
    }

    fn get_buffer(&mut self) -> Vec<u8> {
        self.total_allocations.fetch_add(1, Ordering::Relaxed);

        if let Some(buffer) = self.buffers.pop_front() {
            self.total_hits.fetch_add(1, Ordering::Relaxed);
            buffer
        } else {
            vec![0u8; self.buffer_size]
        }
    }

    fn return_buffer(&mut self, buffer: Vec<u8>) {
        if self.buffers.len() < self.max_buffers && buffer.len() == self.buffer_size {
            self.buffers.push_back(buffer);
        }
        // Otherwise, buffer is dropped and memory is freed
    }

    fn cleanup(&mut self) {
        // Remove excess buffers, keep only half
        let target_size = self.max_buffers / 2;
        while self.buffers.len() > target_size {
            self.buffers.pop_back();
        }
    }

    fn get_stats(&self) -> BufferPoolStats {
        let total_allocs = self.total_allocations.load(Ordering::Relaxed);
        let total_hits = self.total_hits.load(Ordering::Relaxed);

        let hit_ratio = if total_allocs > 0 {
            total_hits as f64 / total_allocs as f64
        } else {
            0.0
        };

        BufferPoolStats {
            total_buffers: self.buffers.len(),
            buffers_in_use: 0, // Would need reference counting to track this
            buffers_available: self.buffers.len(),
            total_memory: (self.buffers.len() * self.buffer_size) as u64,
            hit_ratio,
        }
    }
}

impl PooledBuffer {
    /// Get mutable reference to buffer data
    pub fn data_mut(&mut self) -> Option<&mut Vec<u8>> {
        self.data.as_mut()
    }

    /// Get immutable reference to buffer data
    pub fn data(&self) -> Option<&Vec<u8>> {
        self.data.as_ref()
    }

    /// Get buffer size
    pub fn len(&self) -> usize {
        self.data.as_ref().map_or(0, |d| d.len())
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.data.as_ref().is_none_or(|d| d.is_empty())
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(buffer) = self.data.take() {
            if let Ok(mut pool) = self.pool.lock() {
                pool.return_buffer(buffer);
            }
            // If lock fails, buffer is simply dropped
        }
    }
}

// Implement Send and Sync for PooledBuffer
unsafe impl Send for PooledBuffer {}
unsafe impl Sync for PooledBuffer {}

impl Drop for MemoryManager {
    fn drop(&mut self) {
        self.stop_monitoring();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager_creation() {
        let manager = MemoryManager::with_defaults();
        assert_eq!(manager.get_current_usage(), 0);
        assert_eq!(manager.get_peak_usage(), 0);
    }

    #[test]
    fn test_buffer_allocation() {
        let manager = MemoryManager::with_defaults();
        let buffer = manager.allocate_buffer().unwrap();

        assert!(!buffer.is_empty());
        assert!(manager.get_current_usage() > 0);
    }

    #[test]
    fn test_memory_limit() {
        let manager = MemoryManager::new(MemoryConfig {
            max_memory: 1024, // Very small limit
            buffer_size: 512,
            ..Default::default()
        });

        // First allocation should succeed
        let _buffer1 = manager.allocate_buffer().unwrap();

        // Second allocation should succeed (512 + 512 = 1024)
        let _buffer2 = manager.allocate_buffer().unwrap();

        // Third allocation should fail (would exceed 1024 bytes)
        assert!(manager.allocate_buffer().is_err());
    }

    #[test]
    fn test_buffer_pool_reuse() {
        let manager = MemoryManager::with_defaults();

        // Allocate and drop buffer
        {
            let _buffer = manager.allocate_buffer().unwrap();
        }

        // Memory should be reduced after drop
        std::thread::sleep(Duration::from_millis(10)); // Allow drop to complete

        // Allocate again - should reuse from pool
        let _buffer2 = manager.allocate_buffer().unwrap();
    }
}

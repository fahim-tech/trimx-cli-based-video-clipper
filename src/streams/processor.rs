//! Stream processing implementation


/// Stream processor for handling video/audio streams
pub struct StreamProcessor;

impl StreamProcessor {
    /// Create a new stream processor
    pub fn new() -> Self {
        Self
    }

    /// Get optimal thread count
    pub fn get_thread_count(&self) -> usize {
        num_cpus::get().min(16)
    }

    /// Get optimal buffer size
    pub fn get_buffer_size(&self) -> usize {
        8192
    }
}
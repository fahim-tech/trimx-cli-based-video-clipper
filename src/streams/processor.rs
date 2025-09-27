//! Stream processing implementation

/// Stream processor for handling video/audio streams
pub struct StreamProcessor;

impl StreamProcessor {
    /// Create a new stream processor
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }
}

impl Default for StreamProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamProcessor {
    /// Get optimal thread count
    pub fn get_thread_count(&self) -> usize {
        num_cpus::get().min(16)
    }

    /// Get optimal buffer size
    pub fn get_buffer_size(&self) -> usize {
        8192
    }
}

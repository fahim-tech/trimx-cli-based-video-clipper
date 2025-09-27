//! TrimX CLI Video Clipper Library
//!
//! A Windows-native command-line tool for precise video clipping with intelligent
//! lossless stream-copy and fallback re-encoding capabilities.

pub mod cli;
pub mod domain;
pub mod engine;
pub mod planner;
pub mod probe;
pub mod streams;
pub mod output;
pub mod utils;
pub mod error;

// Re-export commonly used types
pub use error::{TrimXError, TrimXResult};
pub use domain::errors::DomainError;
pub use domain::model::{MediaInfo, VideoStreamInfo, AudioStreamInfo, SubtitleStreamInfo};

/// Initialize TrimX library
pub fn init() -> TrimXResult<()> {
    // Initialize FFmpeg
    ffmpeg_next::init().map_err(|e| TrimXError::FFmpegInitError {
        message: e.to_string(),
    })?;

    Ok(())
}

/// Cleanup TrimX library resources
pub fn cleanup() {
    // FFmpeg cleanup is handled automatically
}

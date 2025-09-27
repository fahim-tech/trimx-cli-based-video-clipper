#![allow(unused_variables)]
#![allow(dead_code)]
//! TrimX CLI Video Clipper Library
//!
//! A Windows-native command-line tool for precise video clipping with intelligent
//! lossless stream-copy and fallback re-encoding capabilities.

pub mod cli;
pub mod domain;
pub mod engine;
pub mod error;
pub mod output;
pub mod planner;
pub mod probe;
pub mod streams;
pub mod utils;

// Re-export commonly used types
pub use domain::errors::DomainError;
pub use domain::model::{AudioStreamInfo, MediaInfo, SubtitleStreamInfo, VideoStreamInfo};
pub use error::{TrimXError, TrimXResult};

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

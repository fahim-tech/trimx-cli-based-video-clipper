//! TrimX CLI Video Clipper Library
//!
//! A Windows-native command-line tool for precise video clipping with intelligent
//! lossless stream-copy and fallback re-encoding capabilities.

pub mod cli;
pub mod error;
pub mod probe;
pub mod planner;
pub mod engine;
pub mod streams;
pub mod output;
pub mod utils;

// Re-export commonly used types
pub use error::{TrimXError, TrimXResult};
pub use probe::{MediaInfo, VideoStreamInfo, AudioStreamInfo, SubtitleStreamInfo};
pub use planner::{CutPlan, ClippingStrategy, KeyframeInfo, StreamMapping};
pub use engine::{EngineConfig, ClippingProgress, ClippingPhase};
pub use output::{OutputConfig, OverwritePolicy, VerificationResult};
pub use utils::{Utils, TimeParser, PathUtils};

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

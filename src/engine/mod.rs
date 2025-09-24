//! Core clipping engine module

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod clipper;
pub mod copy;
pub mod reencode;
pub mod hybrid;

/// Clipping engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Input file path
    pub input_path: String,
    /// Output file path
    pub output_path: String,
    /// Start time in seconds
    pub start_time: f64,
    /// End time in seconds
    pub end_time: f64,
    /// Video codec
    pub video_codec: String,
    /// Audio codec
    pub audio_codec: Option<String>,
    /// CRF quality setting
    pub crf: u8,
    /// Encoding preset
    pub preset: String,
    /// Remove audio streams
    pub no_audio: bool,
    /// Remove subtitle streams
    pub no_subs: bool,
}

/// Clipping progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClippingProgress {
    /// Current phase
    pub phase: ClippingPhase,
    /// Progress percentage (0-100)
    pub progress: f32,
    /// Current operation description
    pub description: String,
    /// Estimated time remaining
    pub eta: Option<std::time::Duration>,
}

/// Clipping phases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClippingPhase {
    /// Analyzing input file
    Analyzing,
    /// Planning strategy
    Planning,
    /// Executing clipping
    Clipping,
    /// Writing output
    Writing,
    /// Verifying output
    Verifying,
    /// Completed
    Completed,
}

//! Cut strategy planning and GOP analysis module

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod strategy;
pub mod gop;

/// Clipping strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClippingStrategy {
    /// Lossless stream copy (fast, approximate)
    Copy,
    /// Full re-encoding (slow, exact)
    Reencode,
    /// Hybrid approach (GOP-spanning method)
    Hybrid {
        /// Re-encode leading segment
        leading_reencode: bool,
        /// Stream copy middle segments
        middle_copy: bool,
        /// Re-encode trailing segment
        trailing_reencode: bool,
    },
}

/// Cut plan information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CutPlan {
    /// Selected clipping strategy
    pub strategy: ClippingStrategy,
    /// Start time in seconds
    pub start_time: f64,
    /// End time in seconds
    pub end_time: f64,
    /// Keyframe analysis
    pub keyframe_info: KeyframeInfo,
    /// Stream mapping
    pub stream_mapping: StreamMapping,
}

/// Keyframe information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeInfo {
    /// Nearest keyframe before start
    pub start_keyframe: Option<f64>,
    /// Nearest keyframe after start
    pub next_keyframe: Option<f64>,
    /// Nearest keyframe before end
    pub end_keyframe: Option<f64>,
    /// GOP size estimate
    pub gop_size: Option<f64>,
}

/// Stream mapping information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMapping {
    /// Video stream index
    pub video_stream: Option<usize>,
    /// Audio stream indices
    pub audio_streams: Vec<usize>,
    /// Subtitle stream indices
    pub subtitle_streams: Vec<usize>,
}

//! Stream handling and mapping module

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod mapper;
pub mod processor;

/// Stream mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMapping {
    /// Video stream mapping
    pub video: Option<VideoStreamMapping>,
    /// Audio stream mappings
    pub audio: Vec<AudioStreamMapping>,
    /// Subtitle stream mappings
    pub subtitles: Vec<SubtitleStreamMapping>,
}

/// Video stream mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStreamMapping {
    /// Input stream index
    pub input_index: usize,
    /// Output stream index
    pub output_index: usize,
    /// Processing mode
    pub mode: VideoProcessingMode,
}

/// Audio stream mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStreamMapping {
    /// Input stream index
    pub input_index: usize,
    /// Output stream index
    pub output_index: usize,
    /// Processing mode
    pub mode: AudioProcessingMode,
}

/// Subtitle stream mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleStreamMapping {
    /// Input stream index
    pub input_index: usize,
    /// Output stream index
    pub output_index: usize,
    /// Processing mode
    pub mode: SubtitleProcessingMode,
}

/// Video processing modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoProcessingMode {
    /// Stream copy (lossless)
    Copy,
    /// Re-encode
    Reencode,
    /// Skip stream
    Skip,
}

/// Audio processing modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioProcessingMode {
    /// Stream copy (lossless)
    Copy,
    /// Re-encode
    Reencode,
    /// Resample and re-encode
    Resample,
    /// Skip stream
    Skip,
}

/// Subtitle processing modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubtitleProcessingMode {
    /// Stream copy (lossless)
    Copy,
    /// Re-time and copy
    Retime,
    /// Skip stream
    Skip,
}

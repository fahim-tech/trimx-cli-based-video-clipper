//! Media file inspection and validation module

use serde::{Deserialize, Serialize};

pub mod inspector;
pub mod validator;

/// Media file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    /// File path
    pub path: String,
    /// Duration in seconds
    pub duration: f64,
    /// Video streams information
    pub video_streams: Vec<VideoStreamInfo>,
    /// Audio streams information
    pub audio_streams: Vec<AudioStreamInfo>,
    /// Subtitle streams information
    pub subtitle_streams: Vec<SubtitleStreamInfo>,
    /// Container format
    pub container: String,
    /// File size in bytes
    pub file_size: u64,
}

impl MediaInfo {
    /// Create new media info
    pub fn new(
        path: String,
        container: String,
        file_size: u64,
        video_streams: Vec<VideoStreamInfo>,
        audio_streams: Vec<AudioStreamInfo>,
        subtitle_streams: Vec<SubtitleStreamInfo>,
    ) -> Self {
        Self {
            path,
            duration: 0.0, // Placeholder
            video_streams,
            audio_streams,
            subtitle_streams,
            container,
            file_size,
        }
    }

    /// Get total number of streams
    pub fn total_streams(&self) -> usize {
        self.video_streams.len() + self.audio_streams.len() + self.subtitle_streams.len()
    }
}

/// Video stream information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStreamInfo {
    /// Stream index
    pub index: usize,
    /// Codec name
    pub codec: String,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Frame rate
    pub frame_rate: f64,
    /// Bit rate
    pub bit_rate: Option<u64>,
    /// Time base
    pub time_base: (i32, i32),
    /// Keyframe interval (GOP size)
    pub keyframe_interval: Option<f64>,
    /// Rotation metadata
    pub rotation: Option<i32>,
}

/// Audio stream information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStreamInfo {
    /// Stream index
    pub index: usize,
    /// Codec name
    pub codec: String,
    /// Sample rate
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u16,
    /// Bit rate
    pub bit_rate: Option<u64>,
    /// Time base
    pub time_base: (i32, i32),
}

/// Subtitle stream information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleStreamInfo {
    /// Stream index
    pub index: usize,
    /// Codec name
    pub codec: String,
    /// Language code
    pub language: Option<String>,
    /// Time base
    pub time_base: (i32, i32),
}

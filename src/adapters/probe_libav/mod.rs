//! Mock probe adapter for testing and demonstration
//!
//! This module provides a mock implementation of media file inspection
//! for testing and demonstration purposes.

use async_trait::async_trait;
use std::collections::HashMap;

use crate::domain::errors::DomainError;
use crate::domain::model::*;
use crate::ports::*;

/// Mock probe adapter for testing
pub struct MockProbeAdapter {
    debug_mode: bool,
}

impl MockProbeAdapter {
    pub fn new() -> Result<Self, DomainError> {
        Ok(Self { debug_mode: false })
    }

    pub fn with_debug(mut self) -> Self {
        self.debug_mode = true;
        self
    }
}

#[async_trait]
impl ProbePort for MockProbeAdapter {
    async fn probe_media(&self, file_path: &str) -> Result<MediaInfo, DomainError> {
        if self.debug_mode {
            println!("Probing media file: {}", file_path);
        }

        // Check if file exists
        if !std::path::Path::new(file_path).exists() {
            return Err(DomainError::ProcessingError(format!(
                "File not found: {}",
                file_path
            )));
        }

        // Get file size
        let file_size = std::fs::metadata(file_path)
            .map_err(|e| {
                DomainError::ProcessingError(format!("Failed to get file metadata: {}", e))
            })?
            .len();

        // Create mock video stream
        let video_stream = VideoStreamInfo {
            index: 0,
            codec: "h264".to_string(),
            width: 1920,
            height: 1080,
            frame_rate: 29.97,
            timebase: Timebase::new(1, 30000).unwrap(),
            bit_rate: Some(5000000),
            rotation: Some(0.0),
            duration: Some(TimeSpec::from_seconds(120.0)),
            keyframe_interval: Some(2.0),
            color_space: Some("bt709".to_string()),
            pixel_format: Some("yuv420p".to_string()),
        };

        // Create mock audio stream
        let audio_stream = AudioStreamInfo {
            index: 1,
            codec: "aac".to_string(),
            sample_rate: 48000,
            channels: 2,
            timebase: Timebase::new(1, 48000).unwrap(),
            bit_rate: Some(128000),
            sample_format: Some("fltp".to_string()),
            channel_layout: Some("stereo".to_string()),
            duration: Some(TimeSpec::from_seconds(120.0)),
            language: Some("eng".to_string()),
        };

        // Create metadata
        let mut metadata = HashMap::new();
        metadata.insert("format".to_string(), "mp4".to_string());
        metadata.insert("duration".to_string(), "120.0".to_string());
        metadata.insert("bit_rate".to_string(), "5128000".to_string());
        metadata.insert("file_size".to_string(), file_size.to_string());

        Ok(MediaInfo {
            path: file_path.to_string(),
            container: "mp4".to_string(),
            duration: TimeSpec::from_seconds(120.0),
            video_streams: vec![video_stream],
            audio_streams: vec![audio_stream],
            subtitle_streams: vec![],
            bit_rate: Some(5128000),
            file_size,
            metadata,
        })
    }

    async fn get_video_stream_info(
        &self,
        file_path: &str,
        stream_index: usize,
    ) -> Result<VideoStreamInfo, DomainError> {
        if self.debug_mode {
            println!(
                "Getting video stream info for stream {} in {}",
                stream_index, file_path
            );
        }

        Ok(VideoStreamInfo {
            index: stream_index,
            codec: "h264".to_string(),
            width: 1920,
            height: 1080,
            frame_rate: 29.97,
            timebase: Timebase::new(1, 30000).unwrap(),
            bit_rate: Some(5000000),
            rotation: Some(0.0),
            duration: Some(TimeSpec::from_seconds(120.0)),
            keyframe_interval: Some(2.0),
            color_space: Some("bt709".to_string()),
            pixel_format: Some("yuv420p".to_string()),
        })
    }

    async fn get_audio_stream_info(
        &self,
        file_path: &str,
        stream_index: usize,
    ) -> Result<AudioStreamInfo, DomainError> {
        if self.debug_mode {
            println!(
                "Getting audio stream info for stream {} in {}",
                stream_index, file_path
            );
        }

        Ok(AudioStreamInfo {
            index: stream_index,
            codec: "aac".to_string(),
            sample_rate: 48000,
            channels: 2,
            timebase: Timebase::new(1, 48000).unwrap(),
            bit_rate: Some(128000),
            sample_format: Some("fltp".to_string()),
            channel_layout: Some("stereo".to_string()),
            duration: Some(TimeSpec::from_seconds(120.0)),
            language: Some("eng".to_string()),
        })
    }

    async fn get_subtitle_stream_info(
        &self,
        file_path: &str,
        stream_index: usize,
    ) -> Result<SubtitleStreamInfo, DomainError> {
        if self.debug_mode {
            println!(
                "Getting subtitle stream info for stream {} in {}",
                stream_index, file_path
            );
        }

        Ok(SubtitleStreamInfo {
            index: stream_index,
            codec: "srt".to_string(),
            language: Some("eng".to_string()),
            duration: Some(TimeSpec::from_seconds(120.0)),
            forced: false,
            default: false,
            timebase: Timebase::new(1, 1000).unwrap(),
        })
    }

    async fn is_format_supported(&self, file_path: &str) -> Result<bool, DomainError> {
        // Check if file exists and has a supported extension
        if !std::path::Path::new(file_path).exists() {
            return Ok(false);
        }

        let extension = std::path::Path::new(file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        let supported_extensions = ["mp4", "mkv", "avi", "mov", "ts", "m4v"];
        Ok(supported_extensions.contains(&extension))
    }

    async fn get_stream_counts(
        &self,
        file_path: &str,
    ) -> Result<(usize, usize, usize), DomainError> {
        if self.debug_mode {
            println!("Getting stream counts for {}", file_path);
        }

        // Mock stream counts
        Ok((1, 1, 0)) // 1 video, 1 audio, 0 subtitles
    }

    async fn probe_keyframes(
        &self,
        file_path: &str,
        stream_index: usize,
    ) -> Result<Vec<KeyframeInfo>, DomainError> {
        if self.debug_mode {
            println!(
                "Probing keyframes for stream {} in {}",
                stream_index, file_path
            );
        }

        // Generate mock keyframes every 2 seconds
        let mut keyframes = Vec::new();
        for i in 0..60 {
            let timestamp = i as f64 * 2.0;
            keyframes.push(KeyframeInfo {
                time_seconds: timestamp,
                position: i as u64,
                pts: (timestamp * 30000.0) as i64, // Convert to PTS
            });
        }

        Ok(keyframes)
    }

    async fn validate_file(&self, file_path: &str) -> Result<bool, DomainError> {
        // Check if file exists and has reasonable size
        if !std::path::Path::new(file_path).exists() {
            return Ok(false);
        }

        let metadata = std::fs::metadata(file_path).map_err(|e| {
            DomainError::ProcessingError(format!("Failed to get file metadata: {}", e))
        })?;

        // File should be at least 1KB
        Ok(metadata.len() > 1024)
    }
}

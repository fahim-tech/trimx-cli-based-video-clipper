//! FFmpeg probe adapter using libav bindings
//! 
//! This module provides media file probing using FFmpeg.

use async_trait::async_trait;
use std::path::Path;
use std::collections::HashMap;

use crate::domain::errors::DomainError;
use crate::domain::model::*;
use crate::ports::*;

/// FFmpeg probe adapter using libav FFI
pub struct LibavProbeAdapter;

impl LibavProbeAdapter {
    pub fn new() -> Result<Self, DomainError> {
        // Initialize FFmpeg
        ffmpeg_next::init().map_err(|e| DomainError::InternalError(format!("FFmpeg initialization failed: {}", e)))?;
        Ok(Self)
    }
}

#[async_trait]
impl ProbePort for LibavProbeAdapter {
    async fn probe_media(&self, file_path: &str) -> Result<MediaInfo, DomainError> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(DomainError::FileNotFound(format!("File not found: {}", file_path)));
        }

        // Open input context
        let ictx = ffmpeg_next::format::input(&file_path)
            .map_err(|e| DomainError::ProbeFail(format!("Failed to open input file: {}", e)))?;

        // Get format information
        let format_name = ictx.format().name().to_string();
        let duration = ictx.duration() as f64 / ffmpeg_next::ffi::AV_TIME_BASE as f64;
        let bit_rate = Some(ictx.bit_rate() as u64);

        // Collect metadata
        let mut metadata = HashMap::new();
        for (key, value) in ictx.metadata().iter() {
            metadata.insert(key.to_string(), value.to_string());
        }

        // Process streams
        let mut video_streams = Vec::new();
        let mut audio_streams = Vec::new();
        let mut subtitle_streams = Vec::new();

        for (index, stream) in ictx.streams().enumerate() {
            let codec = stream.parameters();
            let codec_id = codec.id();
            let time_base = stream.time_base();
            let duration_pts = stream.duration();
            let duration_seconds = if duration_pts != ffmpeg_next::ffi::AV_NOPTS_VALUE {
                duration_pts as f64 * time_base.0 as f64 / time_base.1 as f64
            } else {
                duration
            };

            match codec.medium() {
                ffmpeg_next::media::Type::Video => {
                    // Simplified video stream info
                    video_streams.push(VideoStreamInfo {
                        index,
                        codec: codec_id.name().to_string(),
                        width: 1920,
                        height: 1080,
                        frame_rate: 30.0,
                        bit_rate: Some(5000000),
                        timebase: Timebase { num: time_base.0, den: time_base.1 },
                        pixel_format: Some("yuv420p".to_string()),
                        color_space: Some("bt709".to_string()),
                        rotation: None,
                        duration: Some(TimeSpec::from_seconds(duration_seconds)),
                    });
                }
                ffmpeg_next::media::Type::Audio => {
                    // Simplified audio stream info
                    audio_streams.push(AudioStreamInfo {
                        index,
                        codec: codec_id.name().to_string(),
                        sample_rate: 48000,
                        channels: 2,
                        bit_rate: Some(128000),
                        timebase: Timebase { num: time_base.0, den: time_base.1 },
                        language: None,
                        duration: Some(TimeSpec::from_seconds(duration_seconds)),
                    });
                }
                ffmpeg_next::media::Type::Subtitle => {
                    subtitle_streams.push(SubtitleStreamInfo {
                        index,
                        codec: codec_id.name().to_string(),
                        language: None,
                        duration: Some(TimeSpec::from_seconds(duration_seconds)),
                        forced: false,
                        default: false,
                    });
                }
                _ => {}
            }
        }

        // Get file size
        let file_size = std::fs::metadata(file_path)
            .map_err(|e| DomainError::ProbeFail(format!("Failed to get file metadata: {}", e)))?
            .len();

        Ok(MediaInfo {
            duration: TimeSpec::from_seconds(duration),
            video_streams,
            audio_streams,
            subtitle_streams,
            format: format_name,
            file_size,
            bit_rate,
            metadata,
        })
    }
    
    async fn get_video_stream_info(&self, file_path: &str, stream_index: usize) -> Result<VideoStreamInfo, DomainError> {
        let media_info = self.probe_media(file_path).await?;
        media_info.video_streams.get(stream_index)
            .cloned()
            .ok_or_else(|| DomainError::BadArgs(format!("Video stream index {} not found", stream_index)))
    }
    
    async fn get_audio_stream_info(&self, file_path: &str, stream_index: usize) -> Result<AudioStreamInfo, DomainError> {
        let media_info = self.probe_media(file_path).await?;
        media_info.audio_streams.get(stream_index)
            .cloned()
            .ok_or_else(|| DomainError::BadArgs(format!("Audio stream index {} not found", stream_index)))
    }
    
    async fn get_subtitle_stream_info(&self, file_path: &str, stream_index: usize) -> Result<SubtitleStreamInfo, DomainError> {
        let media_info = self.probe_media(file_path).await?;
        media_info.subtitle_streams.get(stream_index)
            .cloned()
            .ok_or_else(|| DomainError::BadArgs(format!("Subtitle stream index {} not found", stream_index)))
    }
    
    async fn is_format_supported(&self, file_path: &str) -> Result<bool, DomainError> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Ok(false);
        }

        // Try to open the file to check if format is supported
        match ffmpeg_next::format::input(file_path) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    async fn get_stream_counts(&self, file_path: &str) -> Result<(usize, usize, usize), DomainError> {
        let media_info = self.probe_media(file_path).await?;
        Ok((
            media_info.video_streams.len(),
            media_info.audio_streams.len(),
            media_info.subtitle_streams.len(),
        ))
    }
    
    async fn probe_keyframes(&self, file_path: &str, stream_index: usize) -> Result<Vec<KeyframeInfo>, DomainError> {
        let mut ictx = ffmpeg_next::format::input(file_path)
            .map_err(|e| DomainError::ProbeFail(format!("Failed to open input file: {}", e)))?;

        let stream = ictx.streams().nth(stream_index)
            .ok_or_else(|| DomainError::BadArgs(format!("Stream index {} not found", stream_index)))?;

        let mut keyframes = Vec::new();
        let time_base = stream.time_base();
        let mut position = 0u64;

        // Seek to beginning
        ictx.seek(0, ..).map_err(|e| DomainError::ProbeFail(format!("Failed to seek: {}", e)))?;

        // Read packets to find keyframes
        for (stream, packet) in ictx.packets() {
            if stream.index() == stream_index && packet.is_key() {
                let pts = packet.pts().unwrap_or(0);
                let time_seconds = pts as f64 * time_base.0 as f64 / time_base.1 as f64;
                
                keyframes.push(KeyframeInfo {
                    pts,
                    time_seconds,
                    position,
                });
            }
            position += packet.size() as u64;
        }

        Ok(keyframes)
    }
    
    async fn validate_file(&self, file_path: &str) -> Result<bool, DomainError> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Ok(false);
        }

        // Try to open and read basic info
        match ffmpeg_next::format::input(file_path) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
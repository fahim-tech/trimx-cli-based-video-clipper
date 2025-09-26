//! Video file inspector implementation

use std::path::Path;
use std::collections::HashMap;
use anyhow::Result;
use tracing::{info, warn, error};
use ffmpeg_next as ffmpeg;

use crate::probe::{MediaInfo, VideoStreamInfo, AudioStreamInfo, SubtitleStreamInfo};
use crate::error::{TrimXError, TrimXResult};

/// Video file inspector
pub struct VideoInspector;

impl VideoInspector {
    /// Create a new video inspector
    pub fn new() -> Self {
        Self
    }

    /// Inspect a video file and return media information
    pub fn inspect(&self, path: &str) -> TrimXResult<MediaInfo> {
        info!("Inspecting video file: {}", path);

        // Validate file first
        self.validate_file(path)?;

        // Initialize FFmpeg
        ffmpeg::init().map_err(|e| TrimXError::ClippingError {
            message: format!("FFmpeg initialization failed: {}", e),
        })?;

        // Open input context
        let mut ictx = ffmpeg::format::input(path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e),
            })?;

        // Get basic file information
        let file_size = std::fs::metadata(path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to get file metadata: {}", e),
            })?
            .len();

        let format_name = ictx.format().name().to_string();
        let duration = ictx.duration() as f64 / ffmpeg::ffi::AV_TIME_BASE as f64;
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

        for (i, stream) in ictx.streams().enumerate() {
            let codec = stream.codec();
            let codec_type = codec.medium();

            match codec_type {
                ffmpeg::media::Type::Video => {
                    let video_info = VideoStreamInfo {
                        index: i,
                        codec: codec.name().to_string(),
                        width: codec.width(),
                        height: codec.height(),
                        frame_rate: codec.frame_rate(),
                        bit_rate: codec.bit_rate(),
                        pixel_format: codec.format().name().to_string(),
                        aspect_ratio: codec.aspect_ratio(),
                        time_base: stream.time_base(),
                    };
                    video_streams.push(video_info);
                }
                ffmpeg::media::Type::Audio => {
                    let audio_info = AudioStreamInfo {
                        index: i,
                        codec: codec.name().to_string(),
                        sample_rate: codec.sample_rate(),
                        channels: codec.channels(),
                        bit_rate: codec.bit_rate(),
                        sample_format: codec.format().name().to_string(),
                        channel_layout: codec.channel_layout(),
                        time_base: stream.time_base(),
                    };
                    audio_streams.push(audio_info);
                }
                ffmpeg::media::Type::Subtitle => {
                    let subtitle_info = SubtitleStreamInfo {
                        index: i,
                        codec: codec.name().to_string(),
                        language: stream.metadata().get("language").map(|s| s.to_string()),
                    };
                    subtitle_streams.push(subtitle_info);
                }
                _ => {
                    warn!("Unsupported stream type: {:?}", codec_type);
                }
            }
        }

        info!("Found {} video, {} audio, {} subtitle streams", 
            video_streams.len(), audio_streams.len(), subtitle_streams.len());

        Ok(MediaInfo {
            file_path: path.to_string(),
            format: format_name,
            duration,
            file_size,
            bit_rate,
            metadata,
            video_streams,
            audio_streams,
            subtitle_streams,
        })
    }

    /// Validate that a file exists and is readable
    pub fn validate_file(&self, path: &str) -> TrimXResult<()> {
        let path_obj = Path::new(path);
        
        if !path_obj.exists() {
            return Err(TrimXError::ClippingError {
                message: format!("File does not exist: {}", path),
            });
        }

        if !path_obj.is_file() {
            return Err(TrimXError::ClippingError {
                message: format!("Path is not a file: {}", path),
            });
        }

        // Try to open the file to ensure it's readable
        std::fs::File::open(path).map_err(|e| TrimXError::ClippingError {
            message: format!("Cannot read file {}: {}", path, e),
        })?;

        Ok(())
    }

    /// Get detailed stream information
    pub fn get_stream_details(&self, path: &str, stream_index: usize) -> TrimXResult<StreamDetails> {
        info!("Getting details for stream {} in: {}", stream_index, path);

        // Initialize FFmpeg
        ffmpeg::init().map_err(|e| TrimXError::ClippingError {
            message: format!("FFmpeg initialization failed: {}", e),
        })?;

        let mut ictx = ffmpeg::format::input(path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e),
            })?;

        // Find the specified stream
        let stream = ictx.streams()
            .find(|s| s.index() == stream_index)
            .ok_or_else(|| TrimXError::ClippingError {
                message: format!("Stream {} not found", stream_index),
            })?;

        let codec = stream.codec();
        let codec_type = codec.medium();

        let details = match codec_type {
            ffmpeg::media::Type::Video => {
                StreamDetails::Video(VideoStreamDetails {
                    index: stream_index,
                    codec: codec.name().to_string(),
                    width: codec.width(),
                    height: codec.height(),
                    frame_rate: codec.frame_rate(),
                    bit_rate: codec.bit_rate(),
                    pixel_format: codec.format().name().to_string(),
                    aspect_ratio: codec.aspect_ratio(),
                    time_base: stream.time_base(),
                    profile: codec.profile().to_string(),
                    level: codec.level(),
                    has_b_frames: codec.has_b_frames(),
                    max_b_frames: codec.max_b_frames(),
                })
            }
            ffmpeg::media::Type::Audio => {
                StreamDetails::Audio(AudioStreamDetails {
                    index: stream_index,
                    codec: codec.name().to_string(),
                    sample_rate: codec.sample_rate(),
                    channels: codec.channels(),
                    bit_rate: codec.bit_rate(),
                    sample_format: codec.format().name().to_string(),
                    channel_layout: codec.channel_layout(),
                    time_base: stream.time_base(),
                    profile: codec.profile().to_string(),
                })
            }
            ffmpeg::media::Type::Subtitle => {
                StreamDetails::Subtitle(SubtitleStreamDetails {
                    index: stream_index,
                    codec: codec.name().to_string(),
                    language: stream.metadata().get("language").map(|s| s.to_string()),
                })
            }
            _ => {
                return Err(TrimXError::ClippingError {
                    message: format!("Unsupported stream type: {:?}", codec_type),
                });
            }
        };

        Ok(details)
    }

    /// Check if a file is a valid video file
    pub fn is_video_file(&self, path: &str) -> bool {
        if let Err(_) = self.validate_file(path) {
            return false;
        }

        // Try to open with FFmpeg
        if let Err(_) = ffmpeg::init() {
            return false;
        }

        if let Err(_) = ffmpeg::format::input(path) {
            return false;
        }

        true
    }

    /// Get file format information
    pub fn get_format_info(&self, path: &str) -> TrimXResult<FormatInfo> {
        info!("Getting format information for: {}", path);

        // Initialize FFmpeg
        ffmpeg::init().map_err(|e| TrimXError::ClippingError {
            message: format!("FFmpeg initialization failed: {}", e),
        })?;

        let ictx = ffmpeg::format::input(path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e),
            })?;

        let format = ictx.format();
        
        Ok(FormatInfo {
            name: format.name().to_string(),
            long_name: format.long_name().to_string(),
            extensions: format.extensions().split(',').map(|s| s.trim().to_string()).collect(),
            mime_type: format.mime_type().to_string(),
            bit_rate: ictx.bit_rate() as u64,
            duration: ictx.duration() as f64 / ffmpeg::ffi::AV_TIME_BASE as f64,
        })
    }
}

/// Detailed stream information
#[derive(Debug, Clone)]
pub enum StreamDetails {
    Video(VideoStreamDetails),
    Audio(AudioStreamDetails),
    Subtitle(SubtitleStreamDetails),
}

/// Detailed video stream information
#[derive(Debug, Clone)]
pub struct VideoStreamDetails {
    pub index: usize,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
    pub bit_rate: Option<u64>,
    pub pixel_format: String,
    pub aspect_ratio: ffmpeg::Rational,
    pub time_base: ffmpeg::Rational,
    pub profile: String,
    pub level: i32,
    pub has_b_frames: bool,
    pub max_b_frames: u32,
}

/// Detailed audio stream information
#[derive(Debug, Clone)]
pub struct AudioStreamDetails {
    pub index: usize,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_rate: Option<u64>,
    pub sample_format: String,
    pub channel_layout: ffmpeg::ChannelLayout,
    pub time_base: ffmpeg::Rational,
    pub profile: String,
}

/// Detailed subtitle stream information
#[derive(Debug, Clone)]
pub struct SubtitleStreamDetails {
    pub index: usize,
    pub codec: String,
    pub language: Option<String>,
}

/// Format information
#[derive(Debug, Clone)]
pub struct FormatInfo {
    pub name: String,
    pub long_name: String,
    pub extensions: Vec<String>,
    pub mime_type: String,
    pub bit_rate: u64,
    pub duration: f64,
}

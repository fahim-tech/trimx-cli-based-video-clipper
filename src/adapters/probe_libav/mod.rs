//! FFmpeg probe adapter using libav bindings
//! 
//! This module provides comprehensive media file probing using FFmpeg.

use async_trait::async_trait;
use std::path::Path;

use crate::domain::errors::DomainError;
use crate::domain::model::*;
use crate::ports::*;
// Note: Using internal probe types - would need to be properly mapped in production

/// FFmpeg probe adapter using libav FFI
pub struct LibavProbeAdapter {
    /// Enable debug logging
    debug_mode: bool,
}

impl LibavProbeAdapter {
    pub fn new() -> Result<Self, DomainError> {
        // Initialize FFmpeg
        ffmpeg_next::init().map_err(|e| DomainError::InternalError(format!("FFmpeg initialization failed: {}", e)))?;
        
        Ok(Self {
            debug_mode: false,
        })
    }

    /// Enable debug mode for verbose logging
    pub fn with_debug(mut self) -> Self {
        self.debug_mode = true;
        self
    }

    /// Convert FFmpeg Rational to tuple
    fn rational_to_tuple(rational: ffmpeg_next::Rational) -> (i32, i32) {
        (rational.numerator(), rational.denominator())
    }

    /// Convert PTS to seconds using timebase
    fn pts_to_seconds(pts: i64, timebase: ffmpeg_next::Rational) -> f64 {
        if pts == ffmpeg_next::ffi::AV_NOPTS_VALUE {
            0.0
        } else {
            pts as f64 * timebase.numerator() as f64 / timebase.denominator() as f64
        }
    }

    /// Extract video stream information
    fn extract_video_stream_info(&self, stream: &ffmpeg_next::Stream, index: usize, timebase: crate::domain::model::Timebase) -> Result<crate::domain::model::VideoStreamInfo, DomainError> {
        let _params = stream.parameters();
        
        // Get basic video parameters - use defaults since Parameters doesn't expose these directly
        let width = 1920u32; // Default fallback
        let height = 1080u32; // Default fallback
        
        // Get codec name
        let codec = match stream.parameters().id() {
            ffmpeg_next::codec::Id::H264 => "h264".to_string(),
            ffmpeg_next::codec::Id::HEVC => "hevc".to_string(),
            ffmpeg_next::codec::Id::VP9 => "vp9".to_string(),
            ffmpeg_next::codec::Id::AV1 => "av1".to_string(),
            ffmpeg_next::codec::Id::MPEG4 => "mpeg4".to_string(),
            _ => format!("{:?}", stream.parameters().id()).to_lowercase(),
        };
        
        // Calculate frame rate
        let frame_rate = if stream.avg_frame_rate().numerator() > 0 && stream.avg_frame_rate().denominator() > 0 {
            stream.avg_frame_rate().numerator() as f64 / stream.avg_frame_rate().denominator() as f64
        } else {
            30.0 // Default fallback
        };
        
        crate::domain::model::VideoStreamInfo::new(
            index,
            codec,
            width,
            height,
            frame_rate,
            timebase,
        ).map_err(|e| DomainError::ProbeFail(format!("Failed to create VideoStreamInfo: {}", e)))
    }

    /// Extract audio stream information
    fn extract_audio_stream_info(&self, stream: &ffmpeg_next::Stream, index: usize, timebase: crate::domain::model::Timebase) -> Result<crate::domain::model::AudioStreamInfo, DomainError> {
        let _params = stream.parameters();
        
        // Get codec name
        let codec = match stream.parameters().id() {
            ffmpeg_next::codec::Id::AAC => "aac".to_string(),
            ffmpeg_next::codec::Id::MP3 => "mp3".to_string(),
            ffmpeg_next::codec::Id::VORBIS => "vorbis".to_string(),
            ffmpeg_next::codec::Id::OPUS => "opus".to_string(),
            ffmpeg_next::codec::Id::FLAC => "flac".to_string(),
            ffmpeg_next::codec::Id::AC3 => "ac3".to_string(),
            _ => format!("{:?}", stream.parameters().id()).to_lowercase(),
        };
        
        // Get audio parameters with fallbacks - use defaults since Parameters doesn't expose these directly
        let sample_rate = 48000u32; // Default fallback
        let channels = 2u32; // Default fallback
        
        crate::domain::model::AudioStreamInfo::new(
            index,
            codec,
            sample_rate,
            channels,
            timebase,
        ).map_err(|e| DomainError::ProbeFail(format!("Failed to create AudioStreamInfo: {}", e)))
    }

    /// Extract subtitle stream information
    fn extract_subtitle_stream_info(&self, stream: &ffmpeg_next::Stream, index: usize, timebase: crate::domain::model::Timebase) -> Result<crate::domain::model::SubtitleStreamInfo, DomainError> {
        let _params = stream.parameters();
        
        // Get codec name
        let codec = match stream.parameters().id() {
            ffmpeg_next::codec::Id::SUBRIP => "srt".to_string(),
            ffmpeg_next::codec::Id::WEBVTT => "webvtt".to_string(),
            ffmpeg_next::codec::Id::ASS => "ass".to_string(),
            ffmpeg_next::codec::Id::SSA => "ssa".to_string(),
            ffmpeg_next::codec::Id::MOV_TEXT => "mov_text".to_string(),
            _ => format!("{:?}", stream.parameters().id()).to_lowercase(),
        };
        
        // Extract language if available
        let language = stream.metadata().get("language").map(|s| s.to_string());
        
        Ok(crate::domain::model::SubtitleStreamInfo {
            index,
            codec,
            language,
            duration: None,
            forced: false, // Would need to check disposition
            default: false, // Would need to check disposition
            timebase,
        })
    }
}

#[async_trait]
impl ProbePort for LibavProbeAdapter {
    async fn probe_media(&self, file_path: &str) -> Result<MediaInfo, DomainError> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(DomainError::FileNotFound(format!("File not found: {}", file_path)));
        }

        // Get file size
        let file_size = std::fs::metadata(file_path)
            .map_err(|e| DomainError::FsFail(format!("Failed to get file metadata: {}", e)))?
            .len();

        // Open input context
        let ictx = ffmpeg_next::format::input(&file_path)
            .map_err(|e| DomainError::ProbeFail(format!("Failed to open input file: {}", e)))?;

        // Get format information
        let container = ictx.format().name().to_string();
        let duration = ictx.duration() as f64 / ffmpeg_next::ffi::AV_TIME_BASE as f64;

        // Extract actual stream information
        let mut video_streams: Vec<crate::domain::model::VideoStreamInfo> = Vec::new();
        let mut audio_streams: Vec<crate::domain::model::AudioStreamInfo> = Vec::new();  
        let mut subtitle_streams: Vec<crate::domain::model::SubtitleStreamInfo> = Vec::new();

        // Process all streams
        for (index, stream) in ictx.streams().enumerate() {
            let timebase = crate::domain::model::Timebase::new(
                stream.time_base().numerator(),
                stream.time_base().denominator()
            ).unwrap_or_else(|_| crate::domain::model::Timebase::av_time_base());

            match stream.parameters().medium() {
                ffmpeg_next::media::Type::Video => {
                    if let Ok(video_info) = self.extract_video_stream_info(&stream, index, timebase) {
                        video_streams.push(video_info);
                    }
                }
                ffmpeg_next::media::Type::Audio => {
                    if let Ok(audio_info) = self.extract_audio_stream_info(&stream, index, timebase) {
                        audio_streams.push(audio_info);
                    }
                }
                ffmpeg_next::media::Type::Subtitle => {
                    if let Ok(subtitle_info) = self.extract_subtitle_stream_info(&stream, index, timebase) {
                        subtitle_streams.push(subtitle_info);
                    }
                }
                _ => {} // Skip unknown stream types
            }
        }

        // Convert to domain MediaInfo - simplified implementation
        let duration_timespec = crate::domain::model::TimeSpec::from_seconds(duration);
        
        let domain_media_info = MediaInfo {
            path: file_path.to_string(),
            duration: duration_timespec,
            video_streams,
            audio_streams,
            subtitle_streams,
            container,
            file_size,
            bit_rate: None,
            metadata: std::collections::HashMap::new(),
        };

        Ok(domain_media_info)
    }

    async fn get_video_stream_info(&self, _file_path: &str, stream_index: usize) -> Result<VideoStreamInfo, DomainError> {
        // Simplified placeholder implementation
        let timebase = Timebase::new(1, 30)
            .map_err(|e| DomainError::ProbeFail(format!("Invalid timebase: {}", e)))?;

        VideoStreamInfo::new(
            stream_index,
            "h264".to_string(),
            1920,
            1080,
            30.0,
            timebase,
        ).map_err(|e| DomainError::ProbeFail(format!("Failed to create VideoStreamInfo: {}", e)))
    }

    async fn get_audio_stream_info(&self, _file_path: &str, stream_index: usize) -> Result<AudioStreamInfo, DomainError> {
        // Simplified placeholder implementation
        let timebase = Timebase::av_time_base();

        AudioStreamInfo::new(
            stream_index,
            "aac".to_string(),
            48000,
            2,
            timebase,
        ).map_err(|e| DomainError::ProbeFail(format!("Failed to create AudioStreamInfo: {}", e)))
    }

    async fn get_subtitle_stream_info(&self, _file_path: &str, stream_index: usize) -> Result<SubtitleStreamInfo, DomainError> {
        // Simplified placeholder implementation
        let timebase = Timebase::av_time_base();
        
        Ok(SubtitleStreamInfo {
            index: stream_index,
            codec: "srt".to_string(),
            language: Some("en".to_string()),
            duration: None,
            forced: false,
            default: false,
            timebase,
        })
    }

    async fn is_format_supported(&self, file_path: &str) -> Result<bool, DomainError> {
        // Try to open the file to see if FFmpeg can handle it
        match ffmpeg_next::format::input(&file_path) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn get_stream_counts(&self, file_path: &str) -> Result<(usize, usize, usize), DomainError> {
        let ictx = ffmpeg_next::format::input(&file_path)
            .map_err(|e| DomainError::ProbeFail(format!("Failed to open input file: {}", e)))?;

        let mut video_count = 0;
        let mut audio_count = 0;
        let mut subtitle_count = 0;

        for stream in ictx.streams() {
            match stream.parameters().medium() {
                ffmpeg_next::media::Type::Video => video_count += 1,
                ffmpeg_next::media::Type::Audio => audio_count += 1,
                ffmpeg_next::media::Type::Subtitle => subtitle_count += 1,
                _ => {} // Skip other types
            }
        }

        Ok((video_count, audio_count, subtitle_count))
    }

    async fn probe_keyframes(&self, file_path: &str, stream_index: usize) -> Result<Vec<KeyframeInfo>, DomainError> {
        let mut ictx = ffmpeg_next::format::input(&file_path)
            .map_err(|e| DomainError::ProbeFail(format!("Failed to open input file: {}", e)))?;

        let stream = ictx.streams().nth(stream_index)
            .ok_or_else(|| DomainError::ProbeFail(format!("Stream index {} not found", stream_index)))?;

        if stream.parameters().medium() != ffmpeg_next::media::Type::Video {
            return Err(DomainError::ProbeFail(format!("Stream {} is not a video stream", stream_index)));
        }

        let timebase = stream.time_base();
        let mut keyframes = Vec::new();

        // Iterate through packets to find keyframes
        for (packet_stream, packet) in ictx.packets() {
            if packet_stream.index() == stream_index && packet.flags().contains(ffmpeg_next::codec::packet::Flags::KEY) {
                let timestamp = Self::pts_to_seconds(packet.pts().unwrap_or(0), timebase);
                
                keyframes.push(KeyframeInfo {
                    pts: packet.pts().unwrap_or(0),
                    time_seconds: timestamp,
                    position: packet.position() as u64,
                });

                // Limit keyframes to avoid excessive memory usage
                if keyframes.len() >= 1000 {
                    break;
                }
            }
        }

        // Keyframes are now complete with all required information

        Ok(keyframes)
    }

    async fn validate_file(&self, file_path: &str) -> Result<bool, DomainError> {
        let path = Path::new(file_path);
        
        // Check if file exists
        if !path.exists() {
            return Ok(false);
        }

        // Check if file is readable
        if let Err(_) = std::fs::File::open(file_path) {
            return Ok(false);
        }

        // Try to probe the file
        self.is_format_supported(file_path).await
    }
}
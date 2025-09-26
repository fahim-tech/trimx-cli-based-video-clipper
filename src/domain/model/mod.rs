// Domain models - Core types and data structures

use std::time::Duration;
use std::fmt;

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    pub used_memory: u64,
    pub available_memory: u64,
    pub peak_memory: u64,
}
use std::ops::Add;
use crate::domain::errors::DomainError;

/// Time specification with precision - represents time in seconds with fractional precision
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct TimeSpec {
    pub seconds: f64,
}

impl std::ops::Sub for TimeSpec {
    type Output = TimeSpec;
    
    fn sub(self, other: TimeSpec) -> TimeSpec {
        TimeSpec {
            seconds: self.seconds - other.seconds,
        }
    }
}

impl TimeSpec {
    /// Create a new TimeSpec from seconds
    pub fn from_seconds(seconds: f64) -> Self {
        Self { seconds }
    }
    
    /// Convert to seconds
    pub fn to_seconds(&self) -> f64 {
        self.seconds
    }
    
    /// Create a new TimeSpec from hours, minutes, seconds, milliseconds
    pub fn from_components(hours: u32, minutes: u32, seconds: u32, milliseconds: u32) -> Self {
        let total_seconds = hours as f64 * 3600.0 + 
                           minutes as f64 * 60.0 + 
                           seconds as f64 + 
                           milliseconds as f64 / 1000.0;
        Self { seconds: total_seconds }
    }
    
    /// Convert to Duration
    pub fn to_duration(&self) -> Duration {
        Duration::from_secs_f64(self.seconds)
    }
    
    /// Convert from Duration
    pub fn from_duration(duration: Duration) -> Self {
        Self { seconds: duration.as_secs_f64() }
    }
    
    /// Parse time string in various formats
    pub fn parse(time_str: &str) -> Result<Self, DomainError> {
        let trimmed = time_str.trim();
        
        // Try parsing as seconds (float)
        if let Ok(seconds) = trimmed.parse::<f64>() {
            if seconds < 0.0 {
                return Err(DomainError::BadArgs("Time cannot be negative".to_string()));
            }
            return Ok(Self::from_seconds(seconds));
        }
        
        // Try parsing as HH:MM:SS.ms or MM:SS.ms
        let parts: Vec<&str> = trimmed.split(':').collect();
        if parts.len() == 2 {
            // MM:SS.ms format
            let minutes = parts[0].parse::<u32>()
                .map_err(|_| DomainError::BadArgs("Invalid minutes format".to_string()))?;
            let seconds_part = parts[1].parse::<f64>()
                .map_err(|_| DomainError::BadArgs("Invalid seconds format".to_string()))?;
            
            if seconds_part >= 60.0 {
                return Err(DomainError::BadArgs("Seconds must be less than 60".to_string()));
            }
            
            Ok(Self::from_seconds(minutes as f64 * 60.0 + seconds_part))
        } else if parts.len() == 3 {
            // HH:MM:SS.ms format
            let hours = parts[0].parse::<u32>()
                .map_err(|_| DomainError::BadArgs("Invalid hours format".to_string()))?;
            let minutes = parts[1].parse::<u32>()
                .map_err(|_| DomainError::BadArgs("Invalid minutes format".to_string()))?;
            let seconds_part = parts[2].parse::<f64>()
                .map_err(|_| DomainError::BadArgs("Invalid seconds format".to_string()))?;
            
            if minutes >= 60 {
                return Err(DomainError::BadArgs("Minutes must be less than 60".to_string()));
            }
            if seconds_part >= 60.0 {
                return Err(DomainError::BadArgs("Seconds must be less than 60".to_string()));
            }
            
            Ok(Self::from_seconds(hours as f64 * 3600.0 + minutes as f64 * 60.0 + seconds_part))
        } else {
            Err(DomainError::BadArgs(
                "Invalid time format. Supported formats: seconds (e.g., 123.45), MM:SS.ms (e.g., 2:30.5), HH:MM:SS.ms (e.g., 1:02:30.5)".to_string()
            ))
        }
    }
    
    /// Format as HH:MM:SS.ms
    pub fn format_hms(&self) -> String {
        let hours = (self.seconds / 3600.0) as u32;
        let minutes = ((self.seconds % 3600.0) / 60.0) as u32;
        let seconds = (self.seconds % 60.0) as u32;
        let milliseconds = ((self.seconds % 1.0) * 1000.0) as u32;
        
        if hours > 0 {
            format!("{}:{:02}:{:02}.{:03}", hours, minutes, seconds, milliseconds)
        } else {
            format!("{}:{:02}.{:03}", minutes, seconds, milliseconds)
        }
    }
}

impl fmt::Display for TimeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_hms())
    }
}

impl Add for TimeSpec {
    type Output = TimeSpec;
    
    fn add(self, other: TimeSpec) -> TimeSpec {
        TimeSpec {
            seconds: self.seconds + other.seconds,
        }
    }
}

/// Timebase for timestamp calculations - represents rational number for timestamp conversion
#[derive(Debug, Clone, PartialEq)]
pub struct Timebase {
    pub num: i32,
    pub den: i32,
}

impl Timebase {
    /// Create a new timebase
    pub fn new(num: i32, den: i32) -> Result<Self, DomainError> {
        if den == 0 {
            return Err(DomainError::BadArgs("Timebase denominator cannot be zero".to_string()));
        }
        Ok(Self { num, den })
    }
    
    /// Convert to floating point seconds
    pub fn to_seconds(&self) -> f64 {
        self.num as f64 / self.den as f64
    }
    
    /// Rescale PTS from this timebase to target timebase
    pub fn rescale_pts(&self, pts: i64, target: &Timebase) -> i64 {
        if self.den == target.den && self.num == target.num {
            return pts;
        }
        
        // Convert to seconds and back to target timebase
        let seconds = pts as f64 * self.to_seconds();
        (seconds / target.to_seconds()) as i64
    }
    
    /// Convert PTS to seconds
    pub fn pts_to_seconds(&self, pts: i64) -> f64 {
        pts as f64 * self.to_seconds()
    }
    
    /// Convert seconds to PTS
    pub fn seconds_to_pts(&self, seconds: f64) -> i64 {
        (seconds / self.to_seconds()) as i64
    }
    
    /// Common timebases
    pub fn av_time_base() -> Self {
        Self { num: 1, den: 1000000 } // microseconds
    }
    
    pub fn frame_rate_30() -> Self {
        Self { num: 1, den: 30 }
    }
    
    pub fn frame_rate_25() -> Self {
        Self { num: 1, den: 25 }
    }
    
    pub fn frame_rate_24() -> Self {
        Self { num: 1001, den: 24000 } // 23.976 fps
    }
}

/// Video stream information
#[derive(Debug, Clone)]
pub struct VideoStreamInfo {
    pub index: usize,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
    pub bit_rate: Option<u64>,
    pub timebase: Timebase,
    pub pixel_format: Option<String>,
    pub color_space: Option<String>,
    pub rotation: Option<f32>,
    pub duration: Option<TimeSpec>,
    pub keyframe_interval: Option<f64>,
}

impl VideoStreamInfo {
    /// Create new video stream info with validation
    pub fn new(
        index: usize,
        codec: String,
        width: u32,
        height: u32,
        frame_rate: f64,
        timebase: Timebase,
    ) -> Result<Self, DomainError> {
        if width == 0 || height == 0 {
            return Err(DomainError::BadArgs("Video dimensions cannot be zero".to_string()));
        }
        if frame_rate <= 0.0 {
            return Err(DomainError::BadArgs("Frame rate must be positive".to_string()));
        }
        
        Ok(Self {
            index,
            codec,
            width,
            height,
            frame_rate,
            bit_rate: None,
            timebase,
            pixel_format: None,
            color_space: None,
            rotation: None,
            duration: None,
            keyframe_interval: None,
        })
    }
    
    /// Get aspect ratio
    pub fn aspect_ratio(&self) -> f64 {
        self.width as f64 / self.height as f64
    }
    
    /// Check if codec supports copy mode
    pub fn supports_copy(&self) -> bool {
        matches!(self.codec.as_str(), "h264" | "hevc" | "vp9" | "av1")
    }
    
    /// Get frame duration in seconds
    pub fn frame_duration(&self) -> f64 {
        1.0 / self.frame_rate
    }
}

/// Audio stream information
#[derive(Debug, Clone)]
pub struct AudioStreamInfo {
    pub index: usize,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub bit_rate: Option<u64>,
    pub timebase: Timebase,
    pub language: Option<String>,
    pub duration: Option<TimeSpec>,
    pub sample_format: Option<String>,
    pub channel_layout: Option<String>,
}

impl AudioStreamInfo {
    /// Create new audio stream info with validation
    pub fn new(
        index: usize,
        codec: String,
        sample_rate: u32,
        channels: u32,
        timebase: Timebase,
    ) -> Result<Self, DomainError> {
        if sample_rate == 0 {
            return Err(DomainError::BadArgs("Sample rate cannot be zero".to_string()));
        }
        if channels == 0 {
            return Err(DomainError::BadArgs("Channel count cannot be zero".to_string()));
        }
        
        Ok(Self {
            index,
            codec,
            sample_rate,
            channels,
            bit_rate: None,
            timebase,
            language: None,
            duration: None,
            sample_format: None,
            channel_layout: None,
        })
    }
    
    /// Check if codec supports copy mode
    pub fn supports_copy(&self) -> bool {
        matches!(self.codec.as_str(), "aac" | "mp3" | "ac3" | "eac3" | "pcm")
    }
    
    /// Get bytes per sample
    pub fn bytes_per_sample(&self) -> usize {
        match self.codec.as_str() {
            "pcm_s16le" | "pcm_s16be" => 2,
            "pcm_s24le" | "pcm_s24be" => 3,
            "pcm_s32le" | "pcm_s32be" => 4,
            "pcm_f32le" | "pcm_f32be" => 4,
            "pcm_f64le" | "pcm_f64be" => 8,
            _ => 2, // Default assumption
        }
    }
}

/// Subtitle stream information
#[derive(Debug, Clone)]
pub struct SubtitleStreamInfo {
    pub index: usize,
    pub codec: String,
    pub language: Option<String>,
    pub duration: Option<TimeSpec>,
    pub forced: bool,
    pub default: bool,
    pub timebase: Timebase,
}

impl SubtitleStreamInfo {
    /// Create new subtitle stream info
    pub fn new(index: usize, codec: String) -> Self {
        Self {
            index,
            codec,
            language: None,
            duration: None,
            forced: false,
            default: false,
            timebase: Timebase::av_time_base(),
        }
    }
    
    /// Check if subtitle codec supports copy mode
    pub fn supports_copy(&self) -> bool {
        matches!(self.codec.as_str(), "mov_text" | "srt" | "ass" | "ssa" | "subrip")
    }
}

/// Complete media file information
#[derive(Debug, Clone)]
pub struct MediaInfo {
    pub path: String,
    pub duration: TimeSpec,
    pub video_streams: Vec<VideoStreamInfo>,
    pub audio_streams: Vec<AudioStreamInfo>,
    pub subtitle_streams: Vec<SubtitleStreamInfo>,
    pub container: String,
    pub file_size: u64,
    pub bit_rate: Option<u64>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl MediaInfo {
    /// Create new media info with validation
    pub fn new(
        path: String,
        container: String,
        file_size: u64,
        video_streams: Vec<VideoStreamInfo>,
        audio_streams: Vec<AudioStreamInfo>,
        subtitle_streams: Vec<SubtitleStreamInfo>,
    ) -> Result<Self, DomainError> {
        if file_size == 0 {
            return Err(DomainError::BadArgs("File size cannot be zero".to_string()));
        }
        
        // Calculate duration from streams
        let duration = Self::calculate_duration(&video_streams, &audio_streams)?;
        
        Ok(Self {
            path,
            duration,
            video_streams,
            audio_streams,
            subtitle_streams,
            container,
            file_size,
            bit_rate: None,
            metadata: std::collections::HashMap::new(),
        })
    }
    
    /// Calculate duration from stream information
    fn calculate_duration(
        video_streams: &[VideoStreamInfo],
        audio_streams: &[AudioStreamInfo],
    ) -> Result<TimeSpec, DomainError> {
        let mut max_duration = TimeSpec::from_seconds(0.0);
        
        // Use video stream duration if available
        for stream in video_streams {
            if let Some(duration) = &stream.duration {
                if duration.seconds > max_duration.seconds {
                    max_duration = duration.clone();
                }
            }
        }
        
        // Fall back to audio stream duration
        if max_duration.seconds == 0.0 {
            for stream in audio_streams {
                if let Some(duration) = &stream.duration {
                    if duration.seconds > max_duration.seconds {
                        max_duration = duration.clone();
                    }
                }
            }
        }
        
        if max_duration.seconds == 0.0 {
            return Err(DomainError::ProbeFail("Could not determine media duration".to_string()));
        }
        
        Ok(max_duration)
    }
    
    /// Get primary video stream (usually the first one)
    pub fn primary_video_stream(&self) -> Option<&VideoStreamInfo> {
        self.video_streams.first()
    }
    
    /// Get primary audio stream
    pub fn primary_audio_stream(&self) -> Option<&AudioStreamInfo> {
        self.audio_streams.first()
    }
    
    /// Check if all streams support copy mode
    pub fn all_streams_support_copy(&self) -> bool {
        self.video_streams.iter().all(|s| s.supports_copy()) &&
        self.audio_streams.iter().all(|s| s.supports_copy()) &&
        self.subtitle_streams.iter().all(|s| s.supports_copy())
    }
    
    /// Get total number of streams
    pub fn total_streams(&self) -> usize {
        self.video_streams.len() + self.audio_streams.len() + self.subtitle_streams.len()
    }
}

/// Cut range specification
#[derive(Debug, Clone, PartialEq)]
pub struct CutRange {
    pub start: TimeSpec,
    pub end: TimeSpec,
}

impl CutRange {
    /// Create new cut range with validation
    pub fn new(start: TimeSpec, end: TimeSpec) -> Result<Self, DomainError> {
        if start.seconds < 0.0 {
            return Err(DomainError::OutOfRange("Start time cannot be negative".to_string()));
        }
        if end.seconds <= start.seconds {
            return Err(DomainError::OutOfRange("End time must be after start time".to_string()));
        }
        
        Ok(Self { start, end })
    }
    
    /// Get duration of the cut range
    pub fn duration(&self) -> TimeSpec {
        TimeSpec::from_seconds(self.end.seconds - self.start.seconds)
    }
    
    /// Validate against media duration
    pub fn validate_against_duration(&self, media_duration: &TimeSpec) -> Result<(), DomainError> {
        if self.start.seconds >= media_duration.seconds {
            return Err(DomainError::OutOfRange(
                format!("Start time {} exceeds media duration {}", self.start, media_duration)
            ));
        }
        if self.end.seconds > media_duration.seconds {
            return Err(DomainError::OutOfRange(
                format!("End time {} exceeds media duration {}", self.end, media_duration)
            ));
        }
        Ok(())
    }
    
    /// Check if range is valid for copy mode (keyframe aligned)
    pub fn is_keyframe_aligned(&self, frame_duration: f64, tolerance: f64) -> bool {
        let start_aligned = (self.start.seconds / frame_duration).fract().abs() < tolerance;
        let end_aligned = (self.end.seconds / frame_duration).fract().abs() < tolerance;
        start_aligned && end_aligned
    }
}

/// Clipping mode enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum ClippingMode {
    Auto,
    Copy,
    Reencode,
    Hybrid,
}

impl ClippingMode {
    /// Parse mode from string
    pub fn parse(mode_str: &str) -> Result<Self, DomainError> {
        match mode_str.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "copy" => Ok(Self::Copy),
            "reencode" => Ok(Self::Reencode),
            "hybrid" => Ok(Self::Hybrid),
            _ => Err(DomainError::BadArgs(
                format!("Invalid clipping mode: {}. Valid modes: auto, copy, reencode, hybrid", mode_str)
            )),
        }
    }
    
    /// Get description of the mode
    pub fn description(&self) -> &'static str {
        match self {
            Self::Auto => "Automatically select best mode based on content",
            Self::Copy => "Fast lossless stream copy (approximate cuts)",
            Self::Reencode => "Precise frame-accurate cuts with re-encoding",
            Self::Hybrid => "GOP-spanning method (re-encode head/tail, copy middle)",
        }
    }
}

/// Stream mapping for output
#[derive(Debug, Clone)]
pub struct StreamMapping {
    pub input_index: usize,
    pub output_index: usize,
    pub copy: bool,
    pub stream_type: StreamType,
}

/// Stream type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum StreamType {
    Video,
    Audio,
    Subtitle,
}

impl StreamMapping {
    /// Create new stream mapping
    pub fn new(input_index: usize, output_index: usize, copy: bool, stream_type: StreamType) -> Self {
        Self {
            input_index,
            output_index,
            copy,
            stream_type,
        }
    }
}

/// Quality settings for encoding
#[derive(Debug, Clone)]
pub struct QualitySettings {
    pub preset: String,
    pub crf: Option<u8>,
    pub bitrate: Option<u64>,
    pub hardware_acceleration: bool,
}

impl QualitySettings {
    /// Create new quality settings with validation
    pub fn new(
        preset: String,
        crf: Option<u8>,
        bitrate: Option<u64>,
        hardware_acceleration: bool,
    ) -> Result<Self, DomainError> {
        if let Some(crf_value) = crf {
            if crf_value > 51 {
                return Err(DomainError::BadArgs("CRF value cannot exceed 51".to_string()));
            }
        }
        
        Ok(Self {
            preset,
            crf,
            bitrate,
            hardware_acceleration,
        })
    }
    
    /// Get default quality settings
    pub fn default() -> Self {
        Self {
            preset: "medium".to_string(),
            crf: Some(18),
            bitrate: None,
            hardware_acceleration: false,
        }
    }
}

/// Execution plan for video clipping
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub mode: ClippingMode,
    pub input_file: String,
    pub output_file: String,
    pub cut_range: CutRange,
    pub streams: Vec<StreamMapping>,
    pub quality_settings: QualitySettings,
    pub container_format: String,
}

impl ExecutionPlan {
    /// Create new execution plan with validation
    pub fn new(
        mode: ClippingMode,
        input_file: String,
        output_file: String,
        cut_range: CutRange,
        streams: Vec<StreamMapping>,
        quality_settings: QualitySettings,
        container_format: String,
    ) -> Result<Self, DomainError> {
        if input_file.is_empty() {
            return Err(DomainError::BadArgs("Input file cannot be empty".to_string()));
        }
        if output_file.is_empty() {
            return Err(DomainError::BadArgs("Output file cannot be empty".to_string()));
        }
        if streams.is_empty() {
            return Err(DomainError::BadArgs("At least one stream must be mapped".to_string()));
        }
        
        Ok(Self {
            mode,
            input_file,
            output_file,
            cut_range,
            streams,
            quality_settings,
            container_format,
        })
    }
}

/// Output report after clipping
#[derive(Debug, Clone)]
pub struct OutputReport {
    pub success: bool,
    pub duration: TimeSpec,
    pub file_size: u64,
    pub processing_time: Duration,
    pub mode_used: ClippingMode,
    pub warnings: Vec<String>,
    pub first_pts: Option<i64>,
    pub last_pts: Option<i64>,
}

impl OutputReport {
    /// Create successful output report
    pub fn success(
        duration: TimeSpec,
        file_size: u64,
        processing_time: Duration,
        mode_used: ClippingMode,
    ) -> Self {
        Self {
            success: true,
            duration,
            file_size,
            processing_time,
            mode_used,
            warnings: Vec::new(),
            first_pts: None,
            last_pts: None,
        }
    }
    
    /// Create failed output report
    pub fn failure(mode_used: ClippingMode, error_message: String) -> Self {
        Self {
            success: false,
            duration: TimeSpec::from_seconds(0.0),
            file_size: 0,
            processing_time: Duration::from_secs(0),
            mode_used,
            warnings: vec![error_message],
            first_pts: None,
            last_pts: None,
        }
    }
}

/// Request for media file inspection
#[derive(Debug, Clone)]
pub struct InspectRequest {
    pub input_file: String,
    pub include_streams: bool,
    pub include_metadata: bool,
}

impl InspectRequest {
    /// Create new inspect request
    pub fn new(input_file: String) -> Self {
        Self {
            input_file,
            include_streams: true,
            include_metadata: true,
        }
    }
    
    /// Create new inspect request with specific options
    pub fn with_options(
        input_file: String,
        include_streams: bool,
        include_metadata: bool,
    ) -> Self {
        Self {
            input_file,
            include_streams,
            include_metadata,
        }
    }
}

/// Response from media file inspection
#[derive(Debug, Clone)]
pub struct InspectResponse {
    pub success: bool,
    pub media_info: MediaInfo,
    pub error_message: Option<String>,
}

impl InspectResponse {
    /// Create successful inspect response
    pub fn success(media_info: MediaInfo) -> Self {
        Self {
            success: true,
            media_info,
            error_message: None,
        }
    }
    
    /// Create failed inspect response
    pub fn failure(error_message: String) -> Self {
        Self {
            success: false,
            media_info: MediaInfo::new(
                "unknown".to_string(),
                "unknown".to_string(),
                0,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ).unwrap_or_else(|_| panic!("Failed to create default MediaInfo")),
            error_message: Some(error_message),
        }
    }
}


#[cfg(test)]
mod tests;
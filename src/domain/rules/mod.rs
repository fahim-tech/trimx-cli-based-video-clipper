// Domain rules - Business logic and policies

use crate::domain::model::*;
use crate::domain::errors::*;

/// Business rules for clipping mode selection
pub struct ClippingModeSelector;

impl ClippingModeSelector {
    /// Select the optimal clipping mode based on content analysis
    pub fn select_mode(
        media_info: &MediaInfo,
        cut_range: &CutRange,
        requested_mode: ClippingMode,
    ) -> Result<ClippingMode, DomainError> {
        match requested_mode {
            ClippingMode::Auto => Self::auto_select_mode(media_info, cut_range),
            mode => Ok(mode), // Use requested mode if not auto
        }
    }
    
    /// Automatically select the best clipping mode
    fn auto_select_mode(
        media_info: &MediaInfo,
        cut_range: &CutRange,
    ) -> Result<ClippingMode, DomainError> {
        // Check if copy mode is viable
        if Self::is_copy_mode_viable(media_info, cut_range) {
            return Ok(ClippingMode::Copy);
        }
        
        // Check if hybrid mode is viable
        if Self::is_hybrid_mode_viable(media_info, cut_range) {
            return Ok(ClippingMode::Hybrid);
        }
        
        // Fall back to re-encode mode
        Ok(ClippingMode::Reencode)
    }
    
    /// Check if copy mode is viable for the given content
    fn is_copy_mode_viable(
        media_info: &MediaInfo,
        cut_range: &CutRange,
    ) -> bool {
        // All streams must support copy
        if !media_info.all_streams_support_copy() {
            return false;
        }
        
        // Check keyframe alignment for video streams
        if let Some(video_stream) = media_info.primary_video_stream() {
            let frame_duration = video_stream.frame_duration();
            let tolerance = frame_duration * 0.1; // 10% tolerance
            
            if !cut_range.is_keyframe_aligned(frame_duration, tolerance) {
                return false;
            }
        }
        
        // Container format should support copy operations
        Self::container_supports_copy(&media_info.container)
    }
    
    /// Check if hybrid mode is viable
    fn is_hybrid_mode_viable(
        media_info: &MediaInfo,
        _cut_range: &CutRange,
    ) -> bool {
        // At least video stream should support copy
        if let Some(video_stream) = media_info.primary_video_stream() {
            video_stream.supports_copy()
        } else {
            false
        }
    }
    
    /// Check if container format supports copy operations
    fn container_supports_copy(format: &str) -> bool {
        matches!(format.to_lowercase().as_str(), 
            "mp4" | "mkv" | "mov" | "ts" | "mts" | "m2ts"
        )
    }
}

// Keyframe analysis is now handled by the planner module's KeyframeAnalyzer
// which performs actual GOP analysis on the video file. The domain rules
// now use the planner's more comprehensive implementation.

/// Keyframe proximity analysis result
#[derive(Debug, Clone)]
pub struct KeyframeProximity {
    pub start_distance: f64,
    pub end_distance: f64,
    pub is_copy_viable: bool,
}

/// Business rules for stream mapping
pub struct StreamMapper;

impl StreamMapper {
    /// Create optimal stream mappings for the execution plan
    pub fn create_stream_mappings(
        media_info: &MediaInfo,
        mode: &ClippingMode,
    ) -> Result<Vec<StreamMapping>, DomainError> {
        let mut mappings = Vec::new();
        let mut output_index = 0;
        
        // Map video streams
        for (input_index, video_stream) in media_info.video_streams.iter().enumerate() {
            let copy = Self::should_copy_stream(video_stream, mode);
            mappings.push(StreamMapping::new(
                input_index,
                output_index,
                copy,
                StreamType::Video,
            ));
            output_index += 1;
        }
        
        // Map audio streams
        for (input_index, audio_stream) in media_info.audio_streams.iter().enumerate() {
            let copy = Self::should_copy_stream(audio_stream, mode);
            mappings.push(StreamMapping::new(
                input_index,
                output_index,
                copy,
                StreamType::Audio,
            ));
            output_index += 1;
        }
        
        // Map subtitle streams
        for (input_index, subtitle_stream) in media_info.subtitle_streams.iter().enumerate() {
            let copy = Self::should_copy_stream(subtitle_stream, mode);
            mappings.push(StreamMapping::new(
                input_index,
                output_index,
                copy,
                StreamType::Subtitle,
            ));
            output_index += 1;
        }
        
        if mappings.is_empty() {
            return Err(DomainError::BadArgs("No streams found to map".to_string()));
        }
        
        Ok(mappings)
    }
    
    /// Determine if a stream should be copied based on mode and codec support
    fn should_copy_stream<T>(stream: &T, mode: &ClippingMode) -> bool 
    where
        T: StreamCopySupport,
    {
        match mode {
            ClippingMode::Copy => stream.supports_copy(),
            ClippingMode::Hybrid => stream.supports_copy(),
            ClippingMode::Reencode => false, // Always re-encode in re-encode mode
            ClippingMode::Auto => stream.supports_copy(), // Should be resolved before this
        }
    }
}

/// Trait for streams that support copy mode
pub trait StreamCopySupport {
    fn supports_copy(&self) -> bool;
}

impl StreamCopySupport for VideoStreamInfo {
    fn supports_copy(&self) -> bool {
        matches!(self.codec.as_str(), "h264" | "hevc" | "vp9" | "av1")
    }
}

impl StreamCopySupport for AudioStreamInfo {
    fn supports_copy(&self) -> bool {
        matches!(self.codec.as_str(), "aac" | "mp3" | "ac3" | "eac3" | "pcm")
    }
}

impl StreamCopySupport for SubtitleStreamInfo {
    fn supports_copy(&self) -> bool {
        matches!(self.codec.as_str(), "mov_text" | "srt" | "ass" | "ssa" | "subrip")
    }
}

/// Business rules for output validation
pub struct OutputValidator;

impl OutputValidator {
    /// Validate output against expected parameters
    pub fn validate_output(
        output_report: &OutputReport,
        expected_duration: &TimeSpec,
        tolerance_ms: u32,
    ) -> ValidationResult {
        let duration_diff = (output_report.duration.seconds - expected_duration.seconds).abs();
        let tolerance_seconds = tolerance_ms as f64 / 1000.0;
        
        let duration_valid = duration_diff <= tolerance_seconds;
        let success_valid = output_report.success;
        let size_valid = output_report.file_size > 0;
        
        ValidationResult {
            duration_valid,
            success_valid,
            size_valid,
            duration_difference_ms: (duration_diff * 1000.0) as u32,
            overall_valid: duration_valid && success_valid && size_valid,
        }
    }
}

/// Output validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub duration_valid: bool,
    pub success_valid: bool,
    pub size_valid: bool,
    pub duration_difference_ms: u32,
    pub overall_valid: bool,
}

#[cfg(test)]
mod tests;
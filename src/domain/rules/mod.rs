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
        Self::container_supports_copy(&media_info.format)
    }
    
    /// Check if hybrid mode is viable
    fn is_hybrid_mode_viable(
        media_info: &MediaInfo,
        cut_range: &CutRange,
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

#[cfg(test)]
mod tests;
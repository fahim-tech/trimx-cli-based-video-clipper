//! Media file validation utilities

use crate::error::{TrimXError, TrimXResult};
use crate::probe::MediaInfo;

/// Media file validator
pub struct MediaValidator;

impl MediaValidator {
    /// Validate that the media file can be processed
    pub fn validate(&self, media_info: &MediaInfo) -> TrimXResult<()> {
        // Check if file has video streams
        if media_info.video_streams.is_empty() {
            return Err(TrimXError::ProbeError {
                message: "No video streams found in file".to_string(),
            });
        }

        // Check if duration is valid
        if media_info.duration <= 0.0 {
            return Err(TrimXError::ProbeError {
                message: "Invalid or zero duration".to_string(),
            });
        }

        // Check if file size is reasonable
        if media_info.file_size == 0 {
            return Err(TrimXError::ProbeError {
                message: "File appears to be empty".to_string(),
            });
        }

        Ok(())
    }

    /// Validate time range against media duration
    pub fn validate_time_range(&self, start: f64, end: f64, duration: f64) -> TrimXResult<()> {
        if start < 0.0 {
            return Err(TrimXError::InvalidTimeRange {
                start: start.to_string(),
                end: end.to_string(),
            });
        }

        if end > duration {
            return Err(TrimXError::InvalidTimeRange {
                start: start.to_string(),
                end: end.to_string(),
            });
        }

        if start >= end {
            return Err(TrimXError::InvalidTimeRange {
                start: start.to_string(),
                end: end.to_string(),
            });
        }

        Ok(())
    }
}

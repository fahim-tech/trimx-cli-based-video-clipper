//! Output verification implementation

use tracing::{info, warn};

use crate::output::{VerificationResult, OutputConfig};
use crate::probe::{MediaInfo, inspector::VideoInspector};
use crate::error::{TrimXError, TrimXResult};

/// Output file verifier
pub struct ClipVerifier;

impl ClipVerifier {
    /// Create a new clip verifier
    pub fn new() -> Self {
        Self
    }

    /// Verify a clipped segment
    pub fn verify(
        &self,
        output_path: &str,
        expected_start: f64,
        expected_end: f64,
    ) -> TrimXResult<VerificationResult> {
        info!("Verifying clipped segment: {}", output_path);
        info!("Expected range: {} - {}", expected_start, expected_end);

        // Inspect the output file
        let inspector = VideoInspector::new();
        let media_info = inspector.inspect(output_path)?;

        // Calculate verification metrics
        let actual_start = self.get_actual_start_time(&media_info)?;
        let actual_end = self.get_actual_end_time(&media_info)?;
        let duration_accuracy = self.calculate_duration_accuracy(
            expected_start, expected_end,
            actual_start, actual_end,
        );

        // Check if verification passed
        let success = self.is_verification_successful(
            expected_start, expected_end,
            actual_start, actual_end,
            duration_accuracy,
        );

        let result = VerificationResult {
            success,
            expected_start,
            actual_start,
            expected_end,
            actual_end,
            duration_accuracy,
            stream_count_match: true, // TODO: Implement stream count verification
            error: if success { None } else { Some("Verification failed".to_string()) },
        };

        if success {
            info!("Verification passed");
        } else {
            warn!("Verification failed: {:?}", result);
        }

        Ok(result)
    }

    /// Get actual start time from media info
    fn get_actual_start_time(&self, media_info: &MediaInfo) -> TrimXResult<f64> {
        // TODO: Implement actual start time extraction
        // For now, return 0.0 as placeholder
        Ok(0.0)
    }

    /// Get actual end time from media info
    fn get_actual_end_time(&self, media_info: &MediaInfo) -> TrimXResult<f64> {
        // TODO: Implement actual end time extraction
        // For now, return duration as placeholder
        Ok(media_info.duration)
    }

    /// Calculate duration accuracy
    fn calculate_duration_accuracy(
        &self,
        expected_start: f64,
        expected_end: f64,
        actual_start: f64,
        actual_end: f64,
    ) -> f64 {
        let expected_duration = expected_end - expected_start;
        let actual_duration = actual_end - actual_start;
        
        if expected_duration == 0.0 {
            return 1.0; // Perfect accuracy for zero duration
        }

        1.0 - (actual_duration - expected_duration).abs() / expected_duration
    }

    /// Check if verification is successful
    fn is_verification_successful(
        &self,
        expected_start: f64,
        expected_end: f64,
        actual_start: f64,
        actual_end: f64,
        duration_accuracy: f64,
    ) -> bool {
        // Check timing accuracy (within 1 frame tolerance)
        let start_diff = (actual_start - expected_start).abs();
        let end_diff = (actual_end - expected_end).abs();
        let frame_tolerance = 1.0 / 30.0; // Assume 30fps for tolerance

        start_diff <= frame_tolerance
            && end_diff <= frame_tolerance
            && duration_accuracy >= 0.995 // 99.5% accuracy threshold
    }
}

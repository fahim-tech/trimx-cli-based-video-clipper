//! Output verification implementation

use std::path::Path;
use tracing::{info, warn, error};
use ffmpeg_next as ffmpeg;

use crate::output::{VerificationResult, VerificationCheck};
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
        info!("Expected range: {:.3}s - {:.3}s", expected_start, expected_end);

        // Validate file exists
        if !Path::new(output_path).exists() {
            return Err(TrimXError::ClippingError {
                message: format!("Output file does not exist: {}", output_path),
            });
        }

        // Inspect the output file
        let inspector = VideoInspector::new();
        let media_info = inspector.inspect(output_path)?;

        // Perform comprehensive verification
        let mut checks = Vec::new();
        let mut overall_score = 0.0;
        let mut total_weight = 0.0;

        // Check 1: File existence and readability
        let file_check = self.check_file_validity(output_path)?;
        checks.push(file_check.clone());
        overall_score += file_check.score * file_check.weight;
        total_weight += file_check.weight;

        // Check 2: Duration accuracy
        let duration_check = self.check_duration_accuracy(&media_info, expected_start, expected_end)?;
        checks.push(duration_check.clone());
        overall_score += duration_check.score * duration_check.weight;
        total_weight += duration_check.weight;

        // Check 3: Stream integrity
        let stream_check = self.check_stream_integrity(&media_info)?;
        checks.push(stream_check.clone());
        overall_score += stream_check.score * stream_check.weight;
        total_weight += stream_check.weight;

        // Check 4: Codec consistency
        let codec_check = self.check_codec_consistency(&media_info)?;
        checks.push(codec_check.clone());
        overall_score += codec_check.score * codec_check.weight;
        total_weight += codec_check.weight;

        // Check 5: Timing accuracy
        let timing_check = self.check_timing_accuracy(output_path, expected_start, expected_end)?;
        checks.push(timing_check.clone());
        overall_score += timing_check.score * timing_check.weight;
        total_weight += timing_check.weight;

        // Calculate final score
        let final_score = if total_weight > 0.0 {
            overall_score / total_weight
        } else {
            0.0
        };

        // Determine overall success
        let success = final_score >= 0.95 && checks.iter().all(|c| c.success);

        let result = VerificationResult {
            success,
            overall_score: final_score,
            checks,
            error_message: if success { None } else { Some("Verification failed".to_string()) },
        };

        if success {
            info!("Verification passed with score: {:.1}%", final_score * 100.0);
        } else {
            warn!("Verification failed with score: {:.1}%", final_score * 100.0);
        }

        Ok(result)
    }

    /// Check file validity
    fn check_file_validity(&self, output_path: &str) -> TrimXResult<VerificationCheck> {
        let file_exists = Path::new(output_path).exists();
        let file_readable = std::fs::File::open(output_path).is_ok();
        let file_size = std::fs::metadata(output_path)
            .map(|m| m.len())
            .unwrap_or(0);

        let success = file_exists && file_readable && file_size > 0;
        let score = if success { 1.0 } else { 0.0 };

        Ok(VerificationCheck {
            check_type: "File Validity".to_string(),
            success,
            score,
            weight: 0.2,
            details: format!("File exists: {}, readable: {}, size: {} bytes", 
                file_exists, file_readable, file_size),
            error_message: if success { None } else { Some("File validation failed".to_string()) },
        })
    }

    /// Check duration accuracy
    fn check_duration_accuracy(&self, media_info: &MediaInfo, expected_start: f64, expected_end: f64) -> TrimXResult<VerificationCheck> {
        let expected_duration = expected_end - expected_start;
        let actual_duration = media_info.duration;
        
        let duration_diff = (actual_duration - expected_duration).abs();
        let accuracy = if expected_duration > 0.0 {
            1.0 - (duration_diff / expected_duration)
        } else {
            1.0
        };

        let success = accuracy >= 0.99; // 99% accuracy threshold
        let score = accuracy.max(0.0).min(1.0);

        Ok(VerificationCheck {
            check_type: "Duration Accuracy".to_string(),
            success,
            score,
            weight: 0.3,
            details: format!("Expected: {:.3}s, Actual: {:.3}s, Accuracy: {:.1}%", 
                expected_duration, actual_duration, accuracy * 100.0),
            error_message: if success { None } else { Some("Duration accuracy below threshold".to_string()) },
        })
    }

    /// Check stream integrity
    fn check_stream_integrity(&self, media_info: &MediaInfo) -> TrimXResult<VerificationCheck> {
        let has_video = !media_info.video_streams.is_empty();
        let has_audio = !media_info.audio_streams.is_empty();
        let stream_count = media_info.video_streams.len() + media_info.audio_streams.len() + media_info.subtitle_streams.len();

        let success = has_video && stream_count > 0;
        let score = if success { 1.0 } else { 0.0 };

        Ok(VerificationCheck {
            check_type: "Stream Integrity".to_string(),
            success,
            score,
            weight: 0.2,
            details: format!("Video: {}, Audio: {}, Total streams: {}", 
                has_video, has_audio, stream_count),
            error_message: if success { None } else { Some("Stream integrity check failed".to_string()) },
        })
    }

    /// Check codec consistency
    fn check_codec_consistency(&self, media_info: &MediaInfo) -> TrimXResult<VerificationCheck> {
        let mut score = 0.0;
        let mut total_streams = 0;

        // Check video codecs
        for video_stream in &media_info.video_streams {
            if !video_stream.codec.is_empty() {
                score += 1.0;
            }
            total_streams += 1;
        }

        // Check audio codecs
        for audio_stream in &media_info.audio_streams {
            if !audio_stream.codec.is_empty() {
                score += 1.0;
            }
            total_streams += 1;
        }

        let final_score = if total_streams > 0 { score / total_streams as f64 } else { 0.0 };
        let success = final_score >= 0.8; // 80% of streams should have valid codecs

        Ok(VerificationCheck {
            check_type: "Codec Consistency".to_string(),
            success,
            score: final_score,
            weight: 0.15,
            details: format!("Valid codecs: {}/{} streams", score as usize, total_streams),
            error_message: if success { None } else { Some("Codec consistency check failed".to_string()) },
        })
    }

    /// Check timing accuracy using FFmpeg analysis
    fn check_timing_accuracy(&self, output_path: &str, expected_start: f64, expected_end: f64) -> TrimXResult<VerificationCheck> {
        // Initialize FFmpeg
        ffmpeg::init().map_err(|e| TrimXError::ClippingError {
            message: format!("FFmpeg initialization failed: {}", e),
        })?;

        let mut ictx = ffmpeg::format::input(output_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open output file: {}", e),
            })?;

        // Find video stream
        let video_stream = ictx.streams()
            .find(|s| s.parameters().medium() == ffmpeg::media::Type::Video)
            .ok_or_else(|| TrimXError::ClippingError {
                message: "No video stream found".to_string(),
            })?;

        // Analyze first and last frames
        let mut first_frame_time = None;
        let mut last_frame_time = None;
        let mut frame_count = 0;

        for (stream, packet) in ictx.packets() {
            if stream.index() == video_stream.index() {
                let packet_pts = packet.pts().unwrap_or(0);
                let packet_time = packet_pts as f64 * stream.time_base().denominator() as f64 / stream.time_base().numerator() as f64;
                
                if first_frame_time.is_none() {
                    first_frame_time = Some(packet_time);
                }
                last_frame_time = Some(packet_time);
                frame_count += 1;
            }
        }

        let actual_start = first_frame_time.unwrap_or(0.0);
        let actual_end = last_frame_time.unwrap_or(0.0);

        // Calculate timing accuracy
        let start_diff = (actual_start - expected_start).abs();
        let end_diff = (actual_end - expected_end).abs();
        let frame_tolerance = 1.0 / 30.0; // 1 frame at 30fps

        let start_accuracy = if start_diff <= frame_tolerance { 1.0 } else { 0.0 };
        let end_accuracy = if end_diff <= frame_tolerance { 1.0 } else { 0.0 };
        let overall_accuracy = (start_accuracy + end_accuracy) / 2.0;

        let success = overall_accuracy >= 0.9; // 90% accuracy threshold

        Ok(VerificationCheck {
            check_type: "Timing Accuracy".to_string(),
            success,
            score: overall_accuracy,
            weight: 0.15,
            details: format!("Start diff: {:.3}s, End diff: {:.3}s, Frames: {}", 
                start_diff, end_diff, frame_count),
            error_message: if success { None } else { Some("Timing accuracy below threshold".to_string()) },
        })
    }

    /// Verify output against original file
    pub fn verify_against_original(
        &self,
        output_path: &str,
        original_path: &str,
        expected_start: f64,
        expected_end: f64,
    ) -> TrimXResult<VerificationResult> {
        info!("Verifying output against original file");
        info!("Output: {}", output_path);
        info!("Original: {}", original_path);

        // First verify the output file
        let mut result = self.verify(output_path, expected_start, expected_end)?;

        // Then compare with original
        let inspector = VideoInspector::new();
        let original_info = inspector.inspect(original_path)?;
        let output_info = inspector.inspect(output_path)?;

        // Check if output has same format and codecs
        let format_match = original_info.format == output_info.format;
        let codec_match = !output_info.video_streams.is_empty() && 
                        !original_info.video_streams.is_empty() &&
                        output_info.video_streams[0].codec == original_info.video_streams[0].codec;

        let comparison_check = VerificationCheck {
            check_type: "Original Comparison".to_string(),
            success: format_match && codec_match,
            score: if format_match && codec_match { 1.0 } else { 0.0 },
            weight: 0.1,
            details: format!("Format match: {}, Codec match: {}", format_match, codec_match),
            error_message: if format_match && codec_match { None } else { Some("Comparison with original failed".to_string()) },
        };

        result.checks.push(comparison_check.clone());
        result.overall_score = (result.overall_score * 0.9) + (comparison_check.score * comparison_check.weight);
        result.success = result.success && comparison_check.success;

        Ok(result)
    }
}

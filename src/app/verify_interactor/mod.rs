// Verify interactor - Orchestrates output verification use case

use crate::domain::model::*;
use crate::domain::errors::*;
use crate::domain::rules::*;
use crate::ports::*;

/// Interactor for output verification use case
pub struct VerifyInteractor {
    probe_port: Box<dyn ProbePort>,
    fs_port: Box<dyn FsPort>,
    log_port: Box<dyn LogPort>,
}

impl VerifyInteractor {
    /// Create new verify interactor with injected ports
    pub fn new(
        probe_port: Box<dyn ProbePort>,
        fs_port: Box<dyn FsPort>,
        log_port: Box<dyn LogPort>,
    ) -> Self {
        Self {
            probe_port,
            fs_port,
            log_port,
        }
    }
    
    /// Execute output verification
    pub async fn execute(&self, request: VerifyRequest) -> Result<VerifyResponse, DomainError> {
        // Log start of operation
        self.log_port.info(&format!("Starting output verification for: {}", request.output_file));
        
        // Validate output file exists
        if !self.fs_port.file_exists(&request.output_file).await? {
            return Err(DomainError::FsFail(format!("Output file does not exist: {}", request.output_file)));
        }
        
        // Probe output file
        let output_media_info = self.probe_port.probe_media(&request.output_file).await?;
        self.log_port.info(&format!("Output file probed: duration {}, {} streams", 
            output_media_info.duration, output_media_info.total_streams()));
        
        // Get file metadata
        let file_metadata = self.fs_port.get_file_metadata(&request.output_file).await?;
        
        // Perform verification checks
        let verification_result = self.perform_verification_checks(
            &request,
            &output_media_info,
            &file_metadata,
        ).await?;
        
        // Log completion
        if verification_result.success {
            self.log_port.info("Output verification completed successfully");
        } else {
            self.log_port.warn(&format!("Output verification failed: {}", verification_result.error_message));
        }
        
        Ok(VerifyResponse {
            output_file: request.output_file,
            verification_result,
            output_media_info,
            file_metadata,
        })
    }
    
    /// Perform comprehensive verification checks
    async fn perform_verification_checks(
        &self,
        request: &VerifyRequest,
        output_media_info: &MediaInfo,
        file_metadata: &crate::ports::FileMetadata,
    ) -> Result<VerificationResult, DomainError> {
        let mut checks = Vec::new();
        let mut success = true;
        let mut error_message = String::new();
        
        // Check 1: Duration verification
        let duration_check = self.verify_duration(
            &request.expected_range,
            &output_media_info.duration,
            request.tolerance_ms,
        );
        checks.push(duration_check.clone());
        if !duration_check.success {
            success = false;
            error_message = duration_check.error_message;
        }
        
        // Check 2: File size verification
        let size_check = self.verify_file_size(&file_metadata.size);
        checks.push(size_check.clone());
        if !size_check.success && success {
            success = false;
            error_message = size_check.error_message;
        }
        
        // Check 3: Stream verification
        let stream_check = self.verify_streams(output_media_info, &request.expected_mode);
        checks.push(stream_check.clone());
        if !stream_check.success && success {
            success = false;
            error_message = stream_check.error_message;
        }
        
        // Check 4: Format verification
        let format_check = self.verify_format(output_media_info);
        checks.push(format_check.clone());
        if !format_check.success && success {
            success = false;
            error_message = format_check.error_message;
        }
        
        // Check 5: Quality verification (if applicable)
        let quality_check = self.verify_quality(output_media_info, &request.expected_mode);
        checks.push(quality_check.clone());
        if !quality_check.success && success {
            success = false;
            error_message = quality_check.error_message;
        }
        
        Ok(VerificationResult {
            success,
            error_message,
            checks,
            overall_score: self.calculate_overall_score(&checks),
        })
    }
    
    /// Verify output duration matches expected range
    fn verify_duration(
        &self,
        expected_range: &CutRange,
        actual_duration: &TimeSpec,
        tolerance_ms: u32,
    ) -> VerificationCheck {
        let expected_duration = expected_range.duration();
        let duration_diff = (actual_duration.seconds - expected_duration.seconds).abs();
        let tolerance_seconds = tolerance_ms as f64 / 1000.0;
        
        if duration_diff <= tolerance_seconds {
            VerificationCheck {
                check_type: "duration".to_string(),
                success: true,
                error_message: String::new(),
                details: format!("Duration matches expected (diff: {:.3}s)", duration_diff),
            }
        } else {
            VerificationCheck {
                check_type: "duration".to_string(),
                success: false,
                error_message: format!(
                    "Duration mismatch: expected {:.3}s, got {:.3}s (diff: {:.3}s, tolerance: {:.3}s)",
                    expected_duration.seconds, actual_duration.seconds, duration_diff, tolerance_seconds
                ),
                details: String::new(),
            }
        }
    }
    
    /// Verify file size is reasonable
    fn verify_file_size(&self, file_size: &u64) -> VerificationCheck {
        if *file_size == 0 {
            VerificationCheck {
                check_type: "file_size".to_string(),
                success: false,
                error_message: "Output file is empty".to_string(),
                details: String::new(),
            }
        } else if *file_size < 1024 {
            VerificationCheck {
                check_type: "file_size".to_string(),
                success: false,
                error_message: format!("Output file too small: {} bytes", file_size),
                details: String::new(),
            }
        } else {
            VerificationCheck {
                check_type: "file_size".to_string(),
                success: true,
                error_message: String::new(),
                details: format!("File size: {} bytes ({:.2} MB)", file_size, *file_size as f64 / 1_048_576.0),
            }
        }
    }
    
    /// Verify streams are present and valid
    fn verify_streams(&self, media_info: &MediaInfo, expected_mode: &ClippingMode) -> VerificationCheck {
        if media_info.total_streams() == 0 {
            return VerificationCheck {
                check_type: "streams".to_string(),
                success: false,
                error_message: "No streams found in output file".to_string(),
                details: String::new(),
            };
        }
        
        // Check if video stream is present
        if media_info.video_streams.is_empty() {
            return VerificationCheck {
                check_type: "streams".to_string(),
                success: false,
                error_message: "No video stream found in output file".to_string(),
                details: String::new(),
            };
        }
        
        // Verify stream compatibility with expected mode
        let mut details = format!("Streams: {} video, {} audio, {} subtitle", 
            media_info.video_streams.len(), 
            media_info.audio_streams.len(), 
            media_info.subtitle_streams.len());
        
        // Check if streams support the expected mode
        let all_support_copy = media_info.all_streams_support_copy();
        if matches!(expected_mode, ClippingMode::Copy) && !all_support_copy {
            details.push_str(" (Warning: Some streams may not support copy mode)");
        }
        
        VerificationCheck {
            check_type: "streams".to_string(),
            success: true,
            error_message: String::new(),
            details,
        }
    }
    
    /// Verify format is supported and valid
    fn verify_format(&self, media_info: &MediaInfo) -> VerificationCheck {
        let supported_formats = ["mp4", "mkv", "mov", "avi", "ts"];
        
        if supported_formats.contains(&media_info.format.to_lowercase().as_str()) {
            VerificationCheck {
                check_type: "format".to_string(),
                success: true,
                error_message: String::new(),
                details: format!("Format: {}", media_info.format),
            }
        } else {
            VerificationCheck {
                check_type: "format".to_string(),
                success: false,
                error_message: format!("Unsupported output format: {}", media_info.format),
                details: String::new(),
            }
        }
    }
    
    /// Verify quality meets expectations
    fn verify_quality(&self, media_info: &MediaInfo, expected_mode: &ClippingMode) -> VerificationCheck {
        let mut details = String::new();
        let mut warnings = Vec::new();
        
        // Check video stream quality
        if let Some(video_stream) = media_info.primary_video_stream() {
            if video_stream.width == 0 || video_stream.height == 0 {
                warnings.push("Invalid video dimensions".to_string());
            }
            
            if video_stream.frame_rate <= 0.0 {
                warnings.push("Invalid frame rate".to_string());
            }
            
            details.push_str(&format!("Video: {}x{} @ {:.2}fps", 
                video_stream.width, video_stream.height, video_stream.frame_rate));
        }
        
        // Check audio stream quality
        if let Some(audio_stream) = media_info.primary_audio_stream() {
            if audio_stream.sample_rate == 0 {
                warnings.push("Invalid audio sample rate".to_string());
            }
            
            if audio_stream.channels == 0 {
                warnings.push("Invalid audio channel count".to_string());
            }
            
            details.push_str(&format!(", Audio: {}Hz {}ch", 
                audio_stream.sample_rate, audio_stream.channels));
        }
        
        let success = warnings.is_empty();
        let error_message = if success { 
            String::new() 
        } else { 
            warnings.join(", ") 
        };
        
        VerificationCheck {
            check_type: "quality".to_string(),
            success,
            error_message,
            details,
        }
    }
    
    /// Calculate overall verification score
    fn calculate_overall_score(&self, checks: &[VerificationCheck]) -> f32 {
        let total_checks = checks.len();
        let passed_checks = checks.iter().filter(|check| check.success).count();
        
        if total_checks == 0 {
            0.0
        } else {
            (passed_checks as f32 / total_checks as f32) * 100.0
        }
    }
}

/// Request for output verification
#[derive(Debug, Clone)]
pub struct VerifyRequest {
    pub output_file: String,
    pub expected_range: CutRange,
    pub expected_mode: ClippingMode,
    pub tolerance_ms: u32,
}

impl VerifyRequest {
    /// Create new verify request
    pub fn new(
        output_file: String,
        expected_range: CutRange,
        expected_mode: ClippingMode,
    ) -> Self {
        Self {
            output_file,
            expected_range,
            expected_mode,
            tolerance_ms: 100, // Default 100ms tolerance
        }
    }
    
    /// Create new verify request with custom tolerance
    pub fn with_tolerance(
        output_file: String,
        expected_range: CutRange,
        expected_mode: ClippingMode,
        tolerance_ms: u32,
    ) -> Self {
        Self {
            output_file,
            expected_range,
            expected_mode,
            tolerance_ms,
        }
    }
}

/// Response from output verification
#[derive(Debug, Clone)]
pub struct VerifyResponse {
    pub output_file: String,
    pub verification_result: VerificationResult,
    pub output_media_info: MediaInfo,
    pub file_metadata: crate::ports::FileMetadata,
}

/// Verification result with detailed checks
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub success: bool,
    pub error_message: String,
    pub checks: Vec<VerificationCheck>,
    pub overall_score: f32,
}

/// Individual verification check
#[derive(Debug, Clone)]
pub struct VerificationCheck {
    pub check_type: String,
    pub success: bool,
    pub error_message: String,
    pub details: String,
}
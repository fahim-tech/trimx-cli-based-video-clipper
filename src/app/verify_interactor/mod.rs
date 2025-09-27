// Verify interactor - Orchestrates output verification use case

use std::sync::Arc;

use crate::domain::model::*;
use crate::domain::errors::*;
use crate::ports::*;

/// Interactor for output verification use case
pub struct VerifyInteractor {
    probe_port: Arc<dyn ProbePort>,
    fs_port: Arc<dyn FsPort>,
    log_port: Arc<dyn LogPort>,
}

impl VerifyInteractor {
    /// Create new verify interactor with injected ports
    pub fn new(
        probe_port: Arc<dyn ProbePort>,
        fs_port: Arc<dyn FsPort>,
        log_port: Arc<dyn LogPort>,
    ) -> Self {
        Self {
            probe_port,
            fs_port,
            log_port,
        }
    }
    
    /// Execute output verification
    pub async fn execute(&self, request: VerifyRequest) -> Result<VerifyResponse, DomainError> {
        self.verify_output(request).await
    }
    
    /// Execute output verification (alias for compatibility)
    pub async fn verify_output(&self, request: VerifyRequest) -> Result<VerifyResponse, DomainError> {
        // Parse time strings if expected_range is not properly set
        let expected_range = if !request.expected_start.is_empty() && !request.expected_end.is_empty() {
            let start = TimeSpec::parse(&request.expected_start)
                .map_err(|e| DomainError::BadArgs(format!("Invalid start time: {}", e)))?;
            let end = TimeSpec::parse(&request.expected_end)
                .map_err(|e| DomainError::BadArgs(format!("Invalid end time: {}", e)))?;
            CutRange::new(start, end)
                .map_err(|e| DomainError::BadArgs(format!("Invalid cut range: {}", e)))?
        } else {
            request.expected_range.clone()
        };

        // Use output_path as primary, fall back to output_file for compatibility
        let output_file = if !request.output_path.is_empty() {
            request.output_path.clone()
        } else {
            request.output_file.clone()
        };
        
        // Create a processed request for the existing logic
        let processed_request = VerifyRequest {
            output_file: output_file.clone(),
            output_path: output_file.clone(),
            expected_range,
            expected_mode: request.mode.clone(),
            mode: request.mode.clone(),
            tolerance_ms: request.tolerance,
            tolerance: request.tolerance,
            expected_start: request.expected_start,
            expected_end: request.expected_end,
        };
        
        // Log start of operation
        self.log_port.info(&format!("Starting output verification for: {}", processed_request.output_file)).await;
        
        // Validate output file exists
        if !self.fs_port.file_exists(&processed_request.output_file).await? {
            return Err(DomainError::FsFail(format!("Output file does not exist: {}", processed_request.output_file)));
        }
        
        // Probe output file
        let output_media_info = self.probe_port.probe_media(&processed_request.output_file).await?;
        self.log_port
            .info(&format!(
                "Output file probed: duration {}, {} streams",
                output_media_info.duration,
                output_media_info.total_streams()
            ))
            .await;
        
        // Perform verification checks
        let file_metadata = self.fs_port.get_file_metadata(&processed_request.output_file).await?;

        let verification_result = self
            .perform_verification_checks(&processed_request, &output_media_info, &file_metadata)
            .await?;
        
        // Log completion
        if verification_result.success {
            self.log_port.info("Output verification completed successfully").await;
        } else {
            self.log_port
                .warn(&format!("Output verification failed: {}", verification_result.error_message))
                .await;
        }
        
        Ok(VerifyResponse {
            output_file: processed_request.output_file,
            verification_result: verification_result.clone(),
            output_media_info,
            file_metadata,
            is_valid: verification_result.success,
            message: if verification_result.success { None } else { Some(verification_result.error_message) },
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
        
        let overall_score = self.calculate_overall_score(&checks);
        Ok(VerificationResult {
            success,
            error_message,
            checks,
            overall_score,
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
                error_message: format!("Duration mismatch: expected {:.3}s, got {:.3}s (diff: {:.3}s)", 
                    expected_duration.seconds, actual_duration.seconds, duration_diff),
                details: format!("Duration difference exceeds tolerance of {}ms", tolerance_ms),
            }
        }
    }
    
    /// Verify file size is reasonable
    fn verify_file_size(&self, file_size: &u64) -> VerificationCheck {
        if *file_size > 0 {
            VerificationCheck {
                check_type: "file_size".to_string(),
                success: true,
                error_message: String::new(),
                details: format!("File size: {} bytes ({:.2} MB)", file_size, *file_size as f64 / 1_048_576.0),
            }
        } else {
            VerificationCheck {
                check_type: "file_size".to_string(),
                success: false,
                error_message: "Output file is empty".to_string(),
                details: "File size is 0 bytes".to_string(),
            }
        }
    }
    
    /// Verify streams are present and valid
    fn verify_streams(&self, media_info: &MediaInfo, _expected_mode: &ClippingMode) -> VerificationCheck {
        if media_info.total_streams() == 0 {
            VerificationCheck {
                check_type: "streams".to_string(),
                success: false,
                error_message: "No streams found in output file".to_string(),
                details: "Output file contains no video, audio, or subtitle streams".to_string(),
            }
        } else {
            VerificationCheck {
                check_type: "streams".to_string(),
                success: true,
                error_message: String::new(),
                details: format!("Found {} streams ({} video, {} audio, {} subtitle)", 
                    media_info.total_streams(),
                    media_info.video_streams.len(),
                    media_info.audio_streams.len(),
                    media_info.subtitle_streams.len()),
            }
        }
    }
    
    /// Verify format is valid
    fn verify_format(&self, media_info: &MediaInfo) -> VerificationCheck {
        if media_info.container.is_empty() {
            VerificationCheck {
                check_type: "format".to_string(),
                success: false,
                error_message: "Invalid or unknown format".to_string(),
                details: "Format information is missing".to_string(),
            }
        } else {
            VerificationCheck {
                check_type: "format".to_string(),
                success: true,
                error_message: String::new(),
                details: format!("Format: {}", media_info.container),
            }
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
    pub output_path: String,
    pub output_file: String,  // For backward compatibility
    pub expected_start: String,
    pub expected_end: String,
    pub expected_range: CutRange,
    pub mode: ClippingMode,
    pub expected_mode: ClippingMode,  // For backward compatibility
    pub tolerance: u32,
    pub tolerance_ms: u32,  // For backward compatibility
}

impl VerifyRequest {
    /// Create new verify request
    pub fn new(
        output_file: String,
        expected_range: CutRange,
        expected_mode: ClippingMode,
    ) -> Self {
        Self {
            output_path: output_file.clone(),
            output_file,
            expected_start: String::new(),
            expected_end: String::new(),
            expected_range,
            mode: expected_mode.clone(),
            expected_mode,
            tolerance: 100,
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
            output_path: output_file.clone(),
            output_file,
            expected_start: String::new(),
            expected_end: String::new(),
            expected_range,
            mode: expected_mode.clone(),
            expected_mode,
            tolerance: tolerance_ms,
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
    pub is_valid: bool,
    pub message: Option<String>,
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
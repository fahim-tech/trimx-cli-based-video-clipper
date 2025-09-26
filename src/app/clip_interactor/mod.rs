// Clip interactor - Orchestrates video clipping use case

use crate::domain::model::*;
use crate::domain::errors::*;
use crate::domain::rules::*;
use crate::ports::*;

/// Interactor for video clipping use case
pub struct ClipInteractor {
    probe_port: Box<dyn ProbePort>,
    execute_port: Box<dyn ExecutePort>,
    fs_port: Box<dyn FsPort>,
    config_port: Box<dyn ConfigPort>,
    log_port: Box<dyn LogPort>,
}

impl ClipInteractor {
    /// Create new clip interactor with injected ports
    pub fn new(
        probe_port: Box<dyn ProbePort>,
        execute_port: Box<dyn ExecutePort>,
        fs_port: Box<dyn FsPort>,
        config_port: Box<dyn ConfigPort>,
        log_port: Box<dyn LogPort>,
    ) -> Self {
        Self {
            probe_port,
            execute_port,
            fs_port,
            config_port,
            log_port,
        }
    }
    
    /// Execute video clipping
    pub async fn execute(&self, request: ClipRequest) -> Result<ClipResponse, DomainError> {
        // Log start of operation
        let _ = self.log_port.info(&format!("Starting video clipping operation for file: {}", request.input_file));
        
        // Validate input file
        if !self.fs_port.file_exists(&request.input_file).await? {
            return Err(DomainError::FsFail(format!("Input file does not exist: {}", request.input_file)));
        }
        
        // Probe media file
        let media_info = self.probe_port.probe_media(&request.input_file).await?;
        let _ = self.log_port.info(&format!("Media file probed: {} streams, duration: {}", 
            media_info.total_streams(), media_info.duration));
        
        // Validate cut range
        self.validate_cut_range(&request.cut_range, &media_info)?;
        
        // Select optimal clipping mode
        let mode = request.mode.clone();
        let selected_mode = ClippingModeSelector::select_mode(&media_info, &request.cut_range, mode)?;
        let _ = self.log_port.info(&format!("Selected clipping mode: {:?}", selected_mode));
        
        // Create execution plan
        let plan = self.create_execution_plan(request, &media_info, selected_mode).await?;
        
        // Execute clipping
        let result = self.execute_port.execute_plan(&plan).await?;
        
        // Log completion
        if result.success {
            let _ = self.log_port.info(&format!("Video clipping completed successfully. Output: {}", plan.output_file));
        } else {
            let _ = self.log_port.error("Video clipping failed");
        }
        
        Ok(ClipResponse {
            success: result.success,
            output_file: plan.output_file,
            duration: result.duration,
            processing_time: result.processing_time,
            mode_used: result.mode_used,
            warnings: result.warnings,
        })
    }
    
    /// Validate cut range against media duration
    fn validate_cut_range(&self, cut_range: &CutRange, media_info: &MediaInfo) -> Result<(), DomainError> {
        cut_range.validate_against_duration(&media_info.duration)?;
        
        // Additional validation: check if cut range is too short
        let duration = cut_range.duration();
        if duration.seconds < 0.1 {
            return Err(DomainError::OutOfRange("Cut range too short (minimum 0.1 seconds)".to_string()));
        }
        
        // Check if cut range is too long (more than 90% of media duration)
        let max_allowed = media_info.duration.seconds * 0.9;
        if duration.seconds > max_allowed {
            return Err(DomainError::OutOfRange(
                format!("Cut range too long (maximum {}% of media duration)", 
                    (max_allowed / media_info.duration.seconds * 100.0) as u32)
            ));
        }
        
        Ok(())
    }
    
    /// Create execution plan
    async fn create_execution_plan(
        &self, 
        request: ClipRequest, 
        media_info: &MediaInfo, 
        mode: ClippingMode
    ) -> Result<ExecutionPlan, DomainError> {
        // Generate output file path if not provided
        let output_file = if let Some(output) = request.output_file {
            output
        } else {
            self.generate_output_filename(&request.input_file, &request.cut_range)?
        };
        
        // Check if output file already exists
        if self.fs_port.file_exists(&output_file).await? {
            let overwrite_policy = self.config_port.get_config_or_default("overwrite_policy", "prompt").await?;
            if overwrite_policy == "never" {
                return Err(DomainError::FsFail(format!("Output file already exists: {}", output_file)));
            }
        }
        
        // Create stream mappings
        let stream_mappings = StreamMapper::create_stream_mappings(media_info, &mode)?;
        
        // Get quality settings
        let quality_settings = if let Some(settings) = request.quality_settings {
            settings
        } else {
            let hardware_acceleration = self.execute_port.is_hardware_acceleration_available().await?;
            QualitySettings::default() // Simplified for now
        };
        
        // Determine container format
        let container_format = self.determine_container_format(&request.input_file, media_info)?;
        
        // Create execution plan
        let plan = ExecutionPlan::new(
            mode,
            request.input_file,
            output_file,
            request.cut_range,
            stream_mappings,
            quality_settings,
            container_format,
        )?;
        
        let _ = self.log_port.debug(&format!("Created execution plan: {:?}", plan));
        Ok(plan)
    }
    
    /// Generate output filename based on input file and cut range
    fn generate_output_filename(&self, input_file: &str, cut_range: &CutRange) -> Result<String, DomainError> {
        let path = std::path::Path::new(input_file);
        let stem = path.file_stem()
            .ok_or_else(|| DomainError::BadArgs("Invalid input file path".to_string()))?
            .to_string_lossy();
        let extension = path.extension()
            .map(|ext| format!(".{}", ext.to_string_lossy()))
            .unwrap_or_default();
        
        // Format time as user-friendly format (0:15, 0:30, etc.)
        let start_str = self.format_time_for_filename(cut_range.start.seconds);
        let end_str = self.format_time_for_filename(cut_range.end.seconds);
        
        Ok(format!("{}_clip_{}_to_{}{}", stem, start_str, end_str, extension))
    }
    
    /// Format time for filename (e.g., 0:15, 1:30, 1:05:30)
    fn format_time_for_filename(&self, seconds: f64) -> String {
        let total_seconds = seconds as u64;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let secs = total_seconds % 60;
        
        if hours > 0 {
            format!("{}:{:02}:{:02}", hours, minutes, secs)
        } else {
            format!("{}:{:02}", minutes, secs)
        }
    }
    
    /// Determine container format for output
    fn determine_container_format(&self, input_file: &str, media_info: &MediaInfo) -> Result<String, DomainError> {
        // Use same format as input for now
        Ok(media_info.container.clone())
    }
}

/// Request for video clipping
#[derive(Debug, Clone)]
pub struct ClipRequest {
    pub input_file: String,
    pub output_file: Option<String>,
    pub cut_range: CutRange,
    pub mode: ClippingMode,
    pub quality_settings: Option<QualitySettings>,
}

impl ClipRequest {
    /// Create new clip request with validation
    pub fn new(
        input_file: String,
        output_file: Option<String>,
        cut_range: CutRange,
        mode: ClippingMode,
    ) -> Result<Self, DomainError> {
        if input_file.is_empty() {
            return Err(DomainError::BadArgs("Input file cannot be empty".to_string()));
        }
        
        Ok(Self {
            input_file,
            output_file,
            cut_range,
            mode,
            quality_settings: None,
        })
    }
    
    /// Create new clip request with quality settings
    pub fn with_quality_settings(
        input_file: String,
        output_file: Option<String>,
        cut_range: CutRange,
        mode: ClippingMode,
        quality_settings: QualitySettings,
    ) -> Result<Self, DomainError> {
        if input_file.is_empty() {
            return Err(DomainError::BadArgs("Input file cannot be empty".to_string()));
        }
        
        Ok(Self {
            input_file,
            output_file,
            cut_range,
            mode,
            quality_settings: Some(quality_settings),
        })
    }
}

/// Response from video clipping
#[derive(Debug, Clone)]
pub struct ClipResponse {
    pub success: bool,
    pub output_file: String,
    pub duration: TimeSpec,
    pub processing_time: std::time::Duration,
    pub mode_used: ClippingMode,
    pub warnings: Vec<String>,
}

impl ClipResponse {
    /// Create successful clip response
    pub fn success(
        output_file: String,
        duration: TimeSpec,
        processing_time: std::time::Duration,
        mode_used: ClippingMode,
    ) -> Self {
        Self {
            success: true,
            output_file,
            duration,
            processing_time,
            mode_used,
            warnings: Vec::new(),
        }
    }
    
    /// Create failed clip response
    pub fn failure(output_file: String, error_message: String) -> Self {
        Self {
            success: false,
            output_file,
            duration: TimeSpec::from_seconds(0.0),
            processing_time: std::time::Duration::from_secs(0),
            mode_used: ClippingMode::Auto,
            warnings: vec![error_message],
        }
    }
}
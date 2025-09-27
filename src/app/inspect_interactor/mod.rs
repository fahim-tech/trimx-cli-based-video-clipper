// Inspect interactor - Orchestrates media file inspection use case

use std::sync::Arc;

use crate::domain::model::*;
use crate::domain::errors::*;
use crate::ports::*;

/// Interactor for media file inspection use case
pub struct InspectInteractor {
    probe_port: Arc<dyn ProbePort>,
    fs_port: Arc<dyn FsPort>,
    log_port: Arc<dyn LogPort>,
}

impl InspectInteractor {
    /// Create new inspect interactor with injected ports
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
    
    /// Execute media file inspection
    pub async fn execute(&self, request: InspectRequest) -> Result<InspectResponse, DomainError> {
        self.inspect_file(request).await
    }
    
    /// Execute media file inspection (alias for compatibility)
    pub async fn inspect_file(&self, request: InspectRequest) -> Result<InspectResponse, DomainError> {
        // Use input_path as primary, fall back to input for compatibility
        let input_file = if !request.input_path.is_empty() {
            request.input_path.clone()
        } else {
            request.input.clone()
        };
        
        // Log start of operation
        self.log_port.info(&format!("Starting media file inspection for: {}", input_file)).await;
        
        // Validate input file
        if !self.fs_port.file_exists(&input_file).await? {
            return Err(DomainError::FsFail(format!("Input file does not exist: {}", input_file)));
        }
        
        // Probe media file
        let media_info = self.probe_port.probe_media(&input_file).await?;
        self.log_port
            .info(&format!("Media file probed successfully: {} streams", media_info.total_streams()))
            .await;
        
        // Generate summary based on format requested
        let summary = match request.format.as_str() {
            "json" => self.format_as_json(&media_info)?,
            "yaml" => self.format_as_yaml(&media_info)?,
            _ => self.format_as_text(&media_info, &request)?,
        };
        
        // Log completion
        self.log_port.info("Media file inspection completed successfully").await;
        
        Ok(InspectResponse {
            success: true,
            media_info,
            summary,
            error_message: None,
        })
    }
    
    /// Format media info as JSON
    fn format_as_json(&self, media_info: &MediaInfo) -> Result<String, DomainError> {
        serde_json::to_string_pretty(media_info)
            .map_err(|e| DomainError::InternalError(format!("JSON serialization failed: {}", e)))
    }
    
    /// Format media info as YAML
    fn format_as_yaml(&self, media_info: &MediaInfo) -> Result<String, DomainError> {
        serde_yaml::to_string(media_info)
            .map_err(|e| DomainError::InternalError(format!("YAML serialization failed: {}", e)))
    }
    
    /// Format media info as human-readable text
    fn format_as_text(&self, media_info: &MediaInfo, request: &InspectRequest) -> Result<String, DomainError> {
        let mut output = String::new();
        
        output.push_str(&format!("Media File Information:\n"));
        output.push_str(&format!("  File: {}\n", if !request.input_path.is_empty() { &request.input_path } else { &request.input }));
        output.push_str(&format!("  Container: {}\n", media_info.container));
        output.push_str(&format!("  Duration: {:.3}s\n", media_info.duration.seconds));
        output.push_str(&format!("  File Size: {:.2} MB\n", media_info.file_size as f64 / 1_048_576.0));
        output.push_str(&format!("  Total Streams: {}\n", media_info.total_streams()));
        
        if !media_info.video_streams.is_empty() {
            output.push_str(&format!("\nVideo Streams ({}):\n", media_info.video_streams.len()));
            for (i, stream) in media_info.video_streams.iter().enumerate() {
                output.push_str(&format!("  Stream #{}: {}x{} @ {:.2}fps, {}\n", 
                    i, stream.width, stream.height, stream.frame_rate, stream.codec));
            }
        }
        
        if !media_info.audio_streams.is_empty() {
            output.push_str(&format!("\nAudio Streams ({}):\n", media_info.audio_streams.len()));
            for (i, stream) in media_info.audio_streams.iter().enumerate() {
                output.push_str(&format!("  Stream #{}: {} channels, {} Hz, {}\n", 
                    i, stream.channels, stream.sample_rate, stream.codec));
            }
        }
        
        if !media_info.subtitle_streams.is_empty() {
            output.push_str(&format!("\nSubtitle Streams ({}):\n", media_info.subtitle_streams.len()));
            for (i, stream) in media_info.subtitle_streams.iter().enumerate() {
                output.push_str(&format!("  Stream #{}: {} ({})\n", 
                    i, stream.codec, stream.language.as_deref().unwrap_or("unknown")));
            }
        }
        
        if request.show_keyframes {
            output.push_str(&format!("\n[Keyframe analysis would be shown here]\n"));
        }
        
        Ok(output)
    }
}

/// Request for media file inspection
#[derive(Debug, Clone)]
pub struct InspectRequest {
    pub input_path: String,
    pub input: String,  // For backward compatibility
    pub format: String,
    pub show_streams: bool,
    pub show_keyframes: bool,
}

impl InspectRequest {
    /// Create new inspect request
    pub fn new(input_path: String) -> Self {
        Self {
            input_path: input_path.clone(),
            input: input_path,
            format: "text".to_string(),
            show_streams: true,
            show_keyframes: false,
        }
    }
    
    /// Create new inspect request with format
    pub fn with_format(input_path: String, format: String) -> Self {
        Self {
            input_path: input_path.clone(),
            input: input_path,
            format,
            show_streams: true,
            show_keyframes: false,
        }
    }
}

/// Response from media file inspection
#[derive(Debug, Clone)]
pub struct InspectResponse {
    pub success: bool,
    pub media_info: MediaInfo,
    pub summary: String,
    pub error_message: Option<String>,
}

impl InspectResponse {
    /// Create successful inspect response
    pub fn success(media_info: MediaInfo, summary: String) -> Self {
        Self {
            success: true,
            media_info,
            summary,
            error_message: None,
        }
    }
    
    /// Create failed inspect response
    pub fn failure(error_message: String) -> Self {
        Self {
            success: false,
            media_info: MediaInfo::default(),
            summary: String::new(),
            error_message: Some(error_message),
        }
    }
}

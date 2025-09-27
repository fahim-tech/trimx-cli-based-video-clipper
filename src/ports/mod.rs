// Ports - Interface definitions (contracts)

use crate::domain::errors::*;
use crate::domain::model::*;
use async_trait::async_trait;

/// Port for media file probing and analysis
#[async_trait]
pub trait ProbePort: Send + Sync {
    /// Probe media file and return complete information
    async fn probe_media(&self, file_path: &str) -> Result<MediaInfo, DomainError>;

    /// Get detailed video stream information
    async fn get_video_stream_info(
        &self,
        file_path: &str,
        stream_index: usize,
    ) -> Result<VideoStreamInfo, DomainError>;

    /// Get detailed audio stream information
    async fn get_audio_stream_info(
        &self,
        file_path: &str,
        stream_index: usize,
    ) -> Result<AudioStreamInfo, DomainError>;

    /// Get detailed subtitle stream information
    async fn get_subtitle_stream_info(
        &self,
        file_path: &str,
        stream_index: usize,
    ) -> Result<SubtitleStreamInfo, DomainError>;

    /// Check if file format is supported
    async fn is_format_supported(&self, file_path: &str) -> Result<bool, DomainError>;

    /// Get available streams count by type
    async fn get_stream_counts(
        &self,
        file_path: &str,
    ) -> Result<(usize, usize, usize), DomainError>; // (video, audio, subtitle)

    /// Probe keyframe positions for a video stream
    async fn probe_keyframes(
        &self,
        file_path: &str,
        stream_index: usize,
    ) -> Result<Vec<KeyframeInfo>, DomainError>;

    /// Validate file integrity
    async fn validate_file(&self, file_path: &str) -> Result<bool, DomainError>;
}

/// Keyframe information
#[derive(Debug, Clone)]
pub struct KeyframeInfo {
    pub pts: i64,
    pub time_seconds: f64,
    pub position: u64, // byte position in file
}

/// Port for video execution and processing
#[async_trait]
pub trait ExecutePort: Send + Sync {
    /// Execute clipping plan
    async fn execute_plan(&self, plan: &ExecutionPlan) -> Result<OutputReport, DomainError>;

    /// Execute copy mode clipping (fast, lossless)
    async fn execute_copy_mode(&self, plan: &ExecutionPlan) -> Result<OutputReport, DomainError>;

    /// Execute re-encode mode clipping (precise, slower)
    async fn execute_reencode_mode(
        &self,
        plan: &ExecutionPlan,
    ) -> Result<OutputReport, DomainError>;

    /// Execute hybrid mode clipping (GOP-spanning method)
    async fn execute_hybrid_mode(&self, plan: &ExecutionPlan) -> Result<OutputReport, DomainError>;

    /// Check if hardware acceleration is available
    async fn is_hardware_acceleration_available(&self) -> Result<bool, DomainError>;

    /// Get available hardware acceleration types
    async fn get_available_hardware_acceleration(
        &self,
    ) -> Result<Vec<HardwareAccelerationType>, DomainError>;

    /// Get available video codecs
    async fn get_available_video_codecs(&self) -> Result<Vec<CodecInfo>, DomainError>;

    /// Get available audio codecs
    async fn get_available_audio_codecs(&self) -> Result<Vec<CodecInfo>, DomainError>;

    /// Test execution capabilities
    async fn test_execution_capabilities(&self) -> Result<ExecutionCapabilities, DomainError>;

    /// Cancel ongoing execution
    async fn cancel_execution(&self) -> Result<(), DomainError>;

    /// Get execution progress
    async fn get_execution_progress(&self) -> Result<ExecutionProgress, DomainError>;
}

/// Execution phase for progress tracking
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionPhase {
    Initializing,
    Processing,
    Finalizing,
    Complete,
}

/// Hardware acceleration type
#[derive(Debug, Clone, PartialEq)]
pub enum HardwareAccelerationType {
    None,  // No hardware acceleration
    Nvenc, // NVIDIA
    Qsv,   // Intel Quick Sync
    Amf,   // AMD
    Vaapi, // Video Acceleration API
}

/// Codec information
#[derive(Debug, Clone)]
pub struct CodecInfo {
    pub name: String,
    pub long_name: String,
    pub is_encoder: bool,
    pub is_decoder: bool,
    pub is_hardware_accelerated: bool,
}

/// Execution capabilities
#[derive(Debug, Clone)]
pub struct ExecutionCapabilities {
    pub supports_copy_mode: bool,
    pub supports_reencode_mode: bool,
    pub supports_hybrid_mode: bool,
    pub hardware_acceleration_available: bool,
    pub max_concurrent_operations: usize,
}

/// Execution progress information
#[derive(Debug, Clone)]
pub struct ExecutionProgress {
    pub percentage: f32,
    pub current_operation: String,
    pub bytes_processed: u64,
    pub estimated_time_remaining: Option<std::time::Duration>,
}

/// Port for file system operations
#[async_trait]
pub trait FsPort: Send + Sync {
    /// Check if file exists
    async fn file_exists(&self, file_path: &str) -> Result<bool, DomainError>;

    /// Check if directory exists
    async fn directory_exists(&self, dir_path: &str) -> Result<bool, DomainError>;

    /// Get file size
    async fn get_file_size(&self, file_path: &str) -> Result<u64, DomainError>;

    /// Get file metadata
    async fn get_file_metadata(&self, file_path: &str) -> Result<FileMetadata, DomainError>;

    /// Create directory (including parent directories)
    async fn create_directory(&self, dir_path: &str) -> Result<(), DomainError>;

    /// Create output file with atomic write
    async fn create_output_file(&self, file_path: &str) -> Result<(), DomainError>;

    /// Create temporary file
    async fn create_temp_file(&self, prefix: &str, suffix: &str) -> Result<String, DomainError>;

    /// Delete file
    async fn delete_file(&self, file_path: &str) -> Result<(), DomainError>;

    /// Delete directory recursively
    async fn delete_directory(&self, dir_path: &str) -> Result<(), DomainError>;

    /// Move file atomically
    async fn move_file(&self, from: &str, to: &str) -> Result<(), DomainError>;

    /// Copy file
    async fn copy_file(&self, from: &str, to: &str) -> Result<(), DomainError>;

    /// Get available disk space for directory
    async fn get_available_space(&self, dir_path: &str) -> Result<u64, DomainError>;

    /// Validate file path (security check)
    async fn validate_path(&self, file_path: &str) -> Result<bool, DomainError>;

    /// Resolve relative path to absolute path
    async fn resolve_path(&self, file_path: &str) -> Result<String, DomainError>;

    /// Check write permissions for directory
    async fn can_write_to_directory(&self, dir_path: &str) -> Result<bool, DomainError>;
}

/// File metadata
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub created: Option<std::time::SystemTime>,
    pub modified: Option<std::time::SystemTime>,
    pub accessed: Option<std::time::SystemTime>,
    pub is_readonly: bool,
    pub is_hidden: bool,
}

/// Port for configuration management
#[async_trait]
pub trait ConfigPort: Send + Sync {
    /// Get configuration value
    async fn get_config(&self, key: &str) -> Result<Option<String>, DomainError>;

    /// Get configuration value with default
    async fn get_config_or_default(&self, key: &str, default: &str) -> Result<String, DomainError>;

    /// Set configuration value
    async fn set_config(&self, key: &str, value: &str) -> Result<(), DomainError>;

    /// Load configuration from file
    async fn load_config(&self, file_path: &str) -> Result<(), DomainError>;

    /// Save configuration to file
    async fn save_config(&self, file_path: &str) -> Result<(), DomainError>;

    /// Load default configuration
    async fn load_default_config(&self) -> Result<(), DomainError>;

    /// Validate configuration
    async fn validate_config(&self) -> Result<(), DomainError>;

    /// Get all configuration keys
    async fn get_all_config_keys(&self) -> Result<Vec<String>, DomainError>;

    /// Clear configuration
    async fn clear_config(&self) -> Result<(), DomainError>;

    /// Get configuration file path
    async fn get_config_file_path(&self) -> Result<String, DomainError>;
}

/// Port for logging and observability
#[async_trait]
pub trait LogPort: Send + Sync {
    /// Log info message
    async fn info(&self, message: &str);

    /// Log warning message
    async fn warn(&self, message: &str);

    /// Log error message
    async fn error(&self, message: &str);

    /// Log debug message
    async fn debug(&self, message: &str);

    /// Log trace message
    async fn trace(&self, message: &str);

    /// Log structured event
    async fn log_event(&self, event: &LogEvent);

    /// Set log level
    async fn set_log_level(&self, level: LogLevel);

    /// Get current log level
    async fn get_log_level(&self) -> LogLevel;

    /// Enable/disable JSON output
    async fn set_json_output(&self, enabled: bool);

    /// Flush log buffer
    async fn flush(&self);
}

/// Log event with structured data
#[derive(Debug, Clone)]
pub struct LogEvent {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: std::time::SystemTime,
    pub context: std::collections::HashMap<String, String>,
}

/// Log level enumeration
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Parse log level from string
    pub fn parse(level_str: &str) -> Result<Self, DomainError> {
        match level_str.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(DomainError::BadArgs(format!(
                "Invalid log level: {}. Valid levels: trace, debug, info, warn, error",
                level_str
            ))),
        }
    }
}

// Port interfaces are now complete with comprehensive functionality

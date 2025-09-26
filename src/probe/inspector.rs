//! Video inspection implementation

use std::path::Path;
use tracing::info;
use crate::probe::MediaInfo;
use crate::error::{TrimXError, TrimXResult};

/// Video inspector for analyzing media files
pub struct VideoInspector;

impl VideoInspector {
    /// Create a new video inspector
    pub fn new() -> TrimXResult<Self> {
        Ok(Self)
    }

    /// Inspect a video file
    pub fn inspect(&self, path: &str) -> TrimXResult<MediaInfo> {
        info!("Inspecting video file: {}", path);

        // Check if file exists
        if !Path::new(path).exists() {
            return Err(TrimXError::InputFileNotFound { path: path.to_string() });
        }

        // Get file size
        let file_size = std::fs::metadata(path)
            .map_err(|e| TrimXError::IoError(e))?
            .len();

        // Create placeholder media info
        let media_info = MediaInfo::new(
            path.to_string(),
            "mp4".to_string(),
            file_size,
            vec![],
            vec![],
            vec![],
        );

        info!("Video inspection completed");
        Ok(media_info)
    }

    /// Validate a file
    pub fn validate_file(&self, path: &str) -> TrimXResult<bool> {
        Ok(Path::new(path).exists())
    }

    /// Generate output filename
    pub fn generate_filename(&self, input: &str, start: f64, end: f64) -> TrimXResult<String> {
        let path = Path::new(input);
        let stem = path.file_stem()
            .ok_or_else(|| TrimXError::ClippingError { 
                message: "Invalid input file path".to_string() 
            })?
            .to_string_lossy();
        let extension = path.extension()
            .map(|ext| format!(".{}", ext.to_string_lossy()))
            .unwrap_or_else(|| ".mp4".to_string());

        Ok(format!("{}_clip_{:.1}_{:.1}{}", stem, start, end, extension))
    }
}
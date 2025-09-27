//! Output writer implementation

use crate::error::{TrimXError, TrimXResult};
use std::path::Path;

/// Output writer for writing processed video files
pub struct OutputWriter;

impl OutputWriter {
    /// Create a new output writer
    pub fn new() -> Self {
        Self
    }
}

impl Default for OutputWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputWriter {
    /// Generate output filename
    pub fn generate_filename(&self, input: &str, start: f64, end: f64) -> TrimXResult<String> {
        let path = Path::new(input);
        let stem = path
            .file_stem()
            .ok_or_else(|| TrimXError::ClippingError {
                message: "Invalid input file path".to_string(),
            })?
            .to_string_lossy();
        let extension = path
            .extension()
            .map(|ext| format!(".{}", ext.to_string_lossy()))
            .unwrap_or_else(|| ".mp4".to_string());

        Ok(format!(
            "{}_clip_{:.1}_{:.1}{}",
            stem, start, end, extension
        ))
    }

    /// Check if file exists
    pub fn file_exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }
}

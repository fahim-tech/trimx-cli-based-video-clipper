//! Output file writer implementation

use std::path::Path;
use tracing::{info, warn};

use crate::output::{OutputConfig, OverwritePolicy};
use crate::error::{TrimXError, TrimXResult};

/// Output file writer
pub struct OutputWriter;

impl OutputWriter {
    /// Create a new output writer
    pub fn new() -> Self {
        Self
    }

    /// Write output file with safety checks
    pub fn write_output(&self, config: OutputConfig) -> TrimXResult<()> {
        info!("Writing output file: {}", config.path);

        // Check if file exists and handle overwrite policy
        self.check_overwrite_policy(&config)?;

        // Create output directory if needed
        self.ensure_output_directory(&config.path)?;

        // TODO: Implement actual file writing
        // 1. Create temporary file
        // 2. Write data to temporary file
        // 3. Atomic rename to final location
        // 4. Set file permissions

        warn!("Output writing not yet implemented");
        Ok(())
    }

    /// Check overwrite policy and handle existing files
    fn check_overwrite_policy(&self, config: &OutputConfig) -> TrimXResult<()> {
        let path = Path::new(&config.path);
        
        if path.exists() {
            match config.overwrite {
                OverwritePolicy::Never => {
                    return Err(TrimXError::OutputError {
                        message: "Output file exists and overwrite is disabled".to_string(),
                    });
                }
                OverwritePolicy::Prompt => {
                    // TODO: Implement user prompt
                    warn!("File exists, should prompt user for overwrite");
                }
                OverwritePolicy::Always => {
                    info!("File exists, will overwrite");
                }
            }
        }

        Ok(())
    }

    /// Ensure output directory exists
    fn ensure_output_directory(&self, path: &str) -> TrimXResult<()> {
        let path = Path::new(path);
        
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| TrimXError::OutputError {
                message: format!("Failed to create output directory: {}", e),
            })?;
        }

        Ok(())
    }

    /// Generate output filename based on input and time range
    pub fn generate_filename(
        &self,
        input_path: &str,
        start_time: f64,
        end_time: f64,
    ) -> TrimXResult<String> {
        let input_path = Path::new(input_path);
        
        // Extract base name and extension
        let base_name = input_path.file_stem()
            .ok_or_else(|| TrimXError::OutputError {
                message: "Invalid input file name".to_string(),
            })?
            .to_string_lossy();

        let extension = input_path.extension()
            .map(|ext| ext.to_string_lossy())
            .unwrap_or_else(|| "mp4".into());

        // Format time range
        let start_str = self.format_time_for_filename(start_time);
        let end_str = self.format_time_for_filename(end_time);

        // Generate output filename
        let output_name = format!("{}_clip_{}_{}.{}", base_name, start_str, end_str, extension);
        
        Ok(output_name)
    }

    /// Format time for use in filename
    fn format_time_for_filename(&self, time: f64) -> String {
        let hours = (time / 3600.0) as u32;
        let minutes = ((time % 3600.0) / 60.0) as u32;
        let seconds = (time % 60.0) as u32;
        let milliseconds = ((time % 1.0) * 1000.0) as u32;

        format!("{:02}-{:02}-{:02}.{:03}", hours, minutes, seconds, milliseconds)
    }
}

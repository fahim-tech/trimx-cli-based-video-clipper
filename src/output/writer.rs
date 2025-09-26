//! Output file writer implementation

use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};
use std::io::{Write, BufWriter};
use std::time::Instant;
use tracing::{info, warn, error};
use ffmpeg_next as ffmpeg;

use crate::output::{OutputConfig, OverwritePolicy};
use crate::error::{TrimXError, TrimXResult};

/// Output file writer
pub struct OutputWriter {
    temp_dir: Option<PathBuf>,
    atomic_writes: bool,
}

impl OutputWriter {
    /// Create a new output writer
    pub fn new() -> Self {
        Self {
            temp_dir: None,
            atomic_writes: true,
        }
    }

    /// Create output writer with custom temp directory
    pub fn with_temp_dir(temp_dir: PathBuf) -> Self {
        Self {
            temp_dir: Some(temp_dir),
            atomic_writes: true,
        }
    }

    /// Write output file with safety checks
    pub fn write_output(&self, config: OutputConfig) -> TrimXResult<()> {
        info!("Writing output file: {}", config.path);

        // Check if file exists and handle overwrite policy
        self.check_overwrite_policy(&config)?;

        // Create output directory if needed
        self.ensure_output_directory(&config.path)?;

        // Write file atomically
        if self.atomic_writes {
            self.write_atomic(&config)?;
        } else {
            self.write_direct(&config)?;
        }

        // Set file permissions if specified
        if let Some(permissions) = config.permissions {
            self.set_file_permissions(&config.path, permissions)?;
        }

        info!("Output file written successfully: {}", config.path);
        Ok(())
    }

    /// Write file atomically using temporary file
    fn write_atomic(&self, config: &OutputConfig) -> TrimXResult<()> {
        let temp_path = self.create_temp_path(&config.path)?;
        
        info!("Writing to temporary file: {}", temp_path.display());

        // Write to temporary file
        self.write_to_file(&temp_path, config)?;

        // Atomic rename
        std::fs::rename(&temp_path, &config.path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to rename temporary file: {}", e),
            })?;

        info!("Atomic write completed successfully");
        Ok(())
    }

    /// Write file directly
    fn write_direct(&self, config: &OutputConfig) -> TrimXResult<()> {
        self.write_to_file(&config.path, config)
    }

    /// Write data to file
    fn write_to_file(&self, path: &Path, config: &OutputConfig) -> TrimXResult<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open file for writing: {}", e),
            })?;

        // Write data if provided
        if let Some(data) = &config.data {
            let mut writer = BufWriter::with_capacity(config.buffer_size.unwrap_or(8192), &mut file);
            writer.write_all(data)
                .map_err(|e| TrimXError::ClippingError {
                    message: format!("Failed to write data: {}", e),
                })?;
            writer.flush()
                .map_err(|e| TrimXError::ClippingError {
                    message: format!("Failed to flush data: {}", e),
                })?;
        }

        // Sync to disk
        file.sync_all()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to sync file to disk: {}", e),
            })?;

        Ok(())
    }

    /// Create temporary file path
    fn create_temp_path(&self, final_path: &str) -> TrimXResult<PathBuf> {
        let final_path = Path::new(final_path);
        let temp_dir = self.get_temp_directory()?;
        
        let filename = final_path.file_name()
            .ok_or_else(|| TrimXError::ClippingError {
                message: "Invalid output file path".to_string(),
            })?;

        let temp_filename = format!(".tmp_{}", filename.to_string_lossy());
        Ok(temp_dir.join(temp_filename))
    }

    /// Get temporary directory
    fn get_temp_directory(&self) -> TrimXResult<PathBuf> {
        if let Some(ref temp_dir) = self.temp_dir {
            Ok(temp_dir.clone())
        } else {
            // Use system temp directory
            let temp_dir = std::env::temp_dir();
            Ok(temp_dir)
        }
    }

    /// Check overwrite policy and handle existing files
    fn check_overwrite_policy(&self, config: &OutputConfig) -> TrimXResult<()> {
        let path = Path::new(&config.path);
        
        if path.exists() {
            match config.overwrite {
                OverwritePolicy::Never => {
                    return Err(TrimXError::ClippingError {
                        message: "Output file exists and overwrite is disabled".to_string(),
                    });
                }
                OverwritePolicy::Prompt => {
                    // For CLI tool, default to overwrite
                    warn!("File exists, proceeding with overwrite (non-interactive mode)");
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
            std::fs::create_dir_all(parent).map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to create output directory: {}", e),
            })?;
        }

        Ok(())
    }

    /// Set file permissions
    fn set_file_permissions(&self, path: &str, permissions: u32) -> TrimXResult<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(permissions);
            std::fs::set_permissions(path, perms)
                .map_err(|e| TrimXError::ClippingError {
                    message: format!("Failed to set file permissions: {}", e),
                })?;
        }
        
        #[cfg(windows)]
        {
            // Windows permissions are handled differently
            // For now, just log that we would set permissions
            info!("File permissions would be set to {:o} on Windows", permissions);
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
            .ok_or_else(|| TrimXError::ClippingError {
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

    /// Write video data using FFmpeg
    pub fn write_video_data(
        &self,
        input_path: &str,
        output_path: &str,
        start_time: f64,
        end_time: f64,
    ) -> TrimXResult<()> {
        info!("Writing video data from {} to {}", input_path, output_path);
        info!("Time range: {:.3}s - {:.3}s", start_time, end_time);

        let start = Instant::now();

        // Initialize FFmpeg
        ffmpeg::init().map_err(|e| TrimXError::ClippingError {
            message: format!("FFmpeg initialization failed: {}", e),
        })?;

        // Open input context
        let mut ictx = ffmpeg::format::input(input_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e),
            })?;

        // Create output context
        let mut octx = ffmpeg::format::output(output_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to create output file: {}", e),
            })?;

        // Copy streams
        for (i, stream) in ictx.streams().enumerate() {
            let codec = stream.codec();
            let mut output_stream = octx.add_stream(codec.id())
                .map_err(|e| TrimXError::ClippingError {
                    message: format!("Failed to add stream {}: {}", i, e),
                })?;

            output_stream.set_parameters(codec.parameters());
            output_stream.set_time_base(stream.time_base());
        }

        // Write header
        octx.write_header()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to write output header: {}", e),
            })?;

        // Convert time to AV_TIME_BASE
        let start_pts = (start_time * ffmpeg::ffi::AV_TIME_BASE as f64) as i64;
        let end_pts = (end_time * ffmpeg::ffi::AV_TIME_BASE as f64) as i64;

        // Seek to start time
        ictx.seek(start_pts, start_pts..)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to seek to start time: {}", e),
            })?;

        // Process packets
        let mut packet_count = 0;
        for (stream, packet) in ictx.packets() {
            let packet_pts = packet.pts().unwrap_or(0);
            
            if packet_pts >= start_pts && packet_pts <= end_pts {
                // Adjust timestamps
                let mut output_packet = packet.clone();
                output_packet.set_stream(stream.index());
                
                if let Some(pts) = output_packet.pts() {
                    output_packet.set_pts(Some(pts - start_pts));
                }
                if let Some(dts) = output_packet.dts() {
                    output_packet.set_dts(Some(dts - start_pts));
                }

                // Write packet
                octx.write_packet(&output_packet)
                    .map_err(|e| TrimXError::ClippingError {
                        message: format!("Failed to write packet: {}", e),
                    })?;

                packet_count += 1;
            }
        }

        // Write trailer
        octx.write_trailer()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to write output trailer: {}", e),
            })?;

        let duration = start.elapsed();
        info!("Video data written successfully");
        info!("Processed {} packets in {:.2}s", packet_count, duration.as_secs_f64());

        Ok(())
    }

    /// Get file size
    pub fn get_file_size(&self, path: &str) -> TrimXResult<u64> {
        let metadata = std::fs::metadata(path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to get file metadata: {}", e),
            })?;

        Ok(metadata.len())
    }

    /// Check if file exists
    pub fn file_exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }

    /// Delete file
    pub fn delete_file(&self, path: &str) -> TrimXResult<()> {
        std::fs::remove_file(path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to delete file: {}", e),
            })?;

        info!("File deleted: {}", path);
        Ok(())
    }

    /// Move file
    pub fn move_file(&self, from: &str, to: &str) -> TrimXResult<()> {
        std::fs::rename(from, to)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to move file from {} to {}: {}", from, to, e),
            })?;

        info!("File moved from {} to {}", from, to);
        Ok(())
    }

    /// Copy file
    pub fn copy_file(&self, from: &str, to: &str) -> TrimXResult<()> {
        std::fs::copy(from, to)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to copy file from {} to {}: {}", from, to, e),
            })?;

        info!("File copied from {} to {}", from, to);
        Ok(())
    }
}

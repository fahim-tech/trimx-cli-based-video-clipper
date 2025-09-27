//! Stream copy clipping implementation for lossless video clipping

use crate::engine::{ClippingPhase, ClippingProgress, EngineConfig};
use crate::error::{TrimXError, TrimXResult};
use std::time::Instant;
use tracing::{debug, info, warn};

/// Stream copy clipper for lossless clipping when cuts align with keyframes
pub struct StreamCopyClipper {
    /// Enable debug logging
    debug: bool,
}

impl StreamCopyClipper {
    /// Create a new stream copy clipper
    pub fn new() -> Self {
        Self { debug: false }
    }
}

impl Default for StreamCopyClipper {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamCopyClipper {
    /// Enable debug logging
    pub fn with_debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// Execute stream copy clipping
    pub fn clip(&self, config: EngineConfig) -> TrimXResult<ClippingProgress> {
        let start_time = Instant::now();
        info!("Starting stream copy clipping operation");
        info!("Input: {}", config.input_path);
        info!("Output: {}", config.output_path);
        info!(
            "Time range: {:.3}s - {:.3}s",
            config.start_time, config.end_time
        );

        // Validate configuration
        self.validate_config(&config)?;

        match self.execute_stream_copy(&config) {
            Ok(progress) => {
                let elapsed = start_time.elapsed();
                info!(
                    "Stream copy clipping completed successfully in {:.2}s",
                    elapsed.as_secs_f64()
                );
                Ok(progress)
            }
            Err(e) => {
                warn!("Stream copy clipping failed: {}", e);
                Err(e)
            }
        }
    }

    /// Validate the configuration for stream copy
    fn validate_config(&self, config: &EngineConfig) -> TrimXResult<()> {
        // Check that input file exists
        if !std::path::Path::new(&config.input_path).exists() {
            return Err(TrimXError::InputFileNotFound {
                path: config.input_path.clone(),
            });
        }

        // Check that output directory exists
        if let Some(parent) = std::path::Path::new(&config.output_path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(TrimXError::IoError)?;
            }
        }

        // Validate time range
        if config.start_time >= config.end_time {
            return Err(TrimXError::ClippingError {
                message: format!(
                    "Invalid time range: start ({:.3}s) must be before end ({:.3}s)",
                    config.start_time, config.end_time
                ),
            });
        }

        if config.start_time < 0.0 {
            return Err(TrimXError::ClippingError {
                message: "Start time cannot be negative".to_string(),
            });
        }

        Ok(())
    }

    /// Execute the actual stream copy operation using FFmpeg
    fn execute_stream_copy(&self, config: &EngineConfig) -> TrimXResult<ClippingProgress> {
        info!("Opening input file for stream copy");

        // Initialize FFmpeg
        ffmpeg_next::init().map_err(|e| TrimXError::ClippingError {
            message: format!("Failed to initialize FFmpeg: {}", e),
        })?;

        // Open input context
        let mut input_ctx = ffmpeg_next::format::input(&config.input_path).map_err(|e| {
            TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e),
            }
        })?;

        // Create output context
        let mut output_ctx = ffmpeg_next::format::output(&config.output_path).map_err(|e| {
            TrimXError::ClippingError {
                message: format!("Failed to create output file: {}", e),
            }
        })?;

        // Copy stream information from input to output
        self.copy_stream_info(&input_ctx, &mut output_ctx)?;

        // Write output header
        output_ctx
            .write_header()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to write output header: {}", e),
            })?;

        // Perform the actual packet copying
        let progress = self.copy_packets(&mut input_ctx, &mut output_ctx, config)?;

        // Write output trailer
        output_ctx
            .write_trailer()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to write output trailer: {}", e),
            })?;

        info!("Stream copy operation completed successfully");
        Ok(progress)
    }

    /// Copy stream information from input to output
    fn copy_stream_info(
        &self,
        input_ctx: &ffmpeg_next::format::context::Input,
        output_ctx: &mut ffmpeg_next::format::context::Output,
    ) -> TrimXResult<()> {
        for input_stream in input_ctx.streams() {
            // Find the encoder for the input codec
            let codec_id = input_stream.parameters().id();
            let encoder = ffmpeg_next::codec::encoder::find(codec_id).ok_or_else(|| {
                TrimXError::ClippingError {
                    message: format!("No encoder found for codec: {:?}", codec_id),
                }
            })?;

            let mut output_stream =
                output_ctx
                    .add_stream(encoder)
                    .map_err(|e| TrimXError::ClippingError {
                        message: format!("Failed to add output stream: {}", e),
                    })?;

            // Copy parameters from input to output
            output_stream.set_parameters(input_stream.parameters());

            // Copy timebase
            output_stream.set_time_base(input_stream.time_base());

            if self.debug {
                debug!(
                    "Copied stream {}: codec={:?}, time_base={:?}",
                    input_stream.index(),
                    input_stream.parameters().id().name(),
                    input_stream.time_base()
                );
            }
        }

        Ok(())
    }

    /// Copy packets from input to output within the specified time range
    fn copy_packets(
        &self,
        input_ctx: &mut ffmpeg_next::format::context::Input,
        output_ctx: &mut ffmpeg_next::format::context::Output,
        config: &EngineConfig,
    ) -> TrimXResult<ClippingProgress> {
        let start_time_av = (config.start_time * ffmpeg_next::ffi::AV_TIME_BASE as f64) as i64;
        let end_time_av = (config.end_time * ffmpeg_next::ffi::AV_TIME_BASE as f64) as i64;

        // Seek to start time
        if config.start_time > 0.0 {
            input_ctx
                .seek(start_time_av, start_time_av..)
                .map_err(|e| TrimXError::ClippingError {
                    message: format!("Failed to seek to start time: {}", e),
                })?;
        }

        let mut packets_copied = 0;
        let mut total_bytes = 0u64;

        info!("Starting packet copy loop");

        // Process packets
        for (input_stream, mut packet) in input_ctx.packets() {
            // Convert packet timestamp to AV_TIME_BASE
            let packet_time = if let Some(pts) = packet.pts() {
                // Convert from stream timebase to AV_TIME_BASE using proper rescaling
                let stream_tb = input_stream.time_base();
                // Proper timestamp rescaling: pts * stream_tb / AV_TIME_BASE
                (pts as f64 * stream_tb.numerator() as f64 / stream_tb.denominator() as f64
                    * ffmpeg_next::ffi::AV_TIME_BASE as f64) as i64
            } else {
                continue; // Skip packets without PTS
            };

            // Check if packet is within our time range
            if packet_time < start_time_av {
                continue; // Before start time, skip
            }

            if packet_time > end_time_av {
                break; // After end time, stop processing
            }

            // Find the corresponding output stream by matching stream index
            let output_stream_index = input_stream.index();
            let output_stream = &output_ctx.stream(output_stream_index).ok_or_else(|| {
                TrimXError::ClippingError {
                    message: format!("Output stream {} not found", output_stream_index),
                }
            })?;

            packet.rescale_ts(input_stream.time_base(), output_stream.time_base());

            // Write packet to output
            packet
                .write_interleaved(output_ctx)
                .map_err(|e| TrimXError::ClippingError {
                    message: format!("Failed to write packet: {}", e),
                })?;

            packets_copied += 1;
            total_bytes += packet.size() as u64;

            // Log progress periodically
            if self.debug && packets_copied % 1000 == 0 {
                debug!("Copied {} packets, {} bytes", packets_copied, total_bytes);
            }
        }

        info!(
            "Copied {} packets ({} bytes) in total",
            packets_copied, total_bytes
        );

        Ok(ClippingProgress {
            phase: ClippingPhase::Completed,
            progress: 100.0,
            description: format!(
                "Stream copy completed: {} packets, {:.2} MB",
                packets_copied,
                total_bytes as f64 / 1024.0 / 1024.0
            ),
            eta: None,
        })
    }

    /// Check if stream copy is possible for this configuration
    pub fn is_possible(&self, config: &EngineConfig) -> bool {
        // For stream copy to work well, cuts should be at keyframe boundaries
        // This is a simplified check - in a real implementation we'd analyze GOP structure

        // Basic validation
        if !std::path::Path::new(&config.input_path).exists() {
            return false;
        }

        if config.start_time >= config.end_time {
            return false;
        }

        if config.start_time < 0.0 {
            return false;
        }

        // In a full implementation, we would:
        // 1. Check if cuts align with keyframes
        // 2. Verify codec compatibility
        // 3. Check if the container supports stream copying

        true // Optimistic assumption for now
    }

    /// Get estimated file size after stream copy
    pub fn estimate_output_size(&self, config: &EngineConfig) -> TrimXResult<u64> {
        let input_size = std::fs::metadata(&config.input_path)
            .map_err(TrimXError::IoError)?
            .len();

        // For stream copy, estimate proportional to time range
        // This is approximate since bitrate may not be constant
        let duration_ratio =
            (config.end_time - config.start_time) / self.get_input_duration(config)?;
        let estimated_size = (input_size as f64 * duration_ratio) as u64;

        Ok(estimated_size)
    }

    /// Get the duration of the input file
    fn get_input_duration(&self, config: &EngineConfig) -> TrimXResult<f64> {
        ffmpeg_next::init().map_err(|e| TrimXError::ClippingError {
            message: format!("Failed to initialize FFmpeg: {}", e),
        })?;

        let input_ctx = ffmpeg_next::format::input(&config.input_path).map_err(|e| {
            TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e),
            }
        })?;

        let duration_seconds = input_ctx.duration() as f64 / ffmpeg_next::ffi::AV_TIME_BASE as f64;
        Ok(duration_seconds)
    }
}

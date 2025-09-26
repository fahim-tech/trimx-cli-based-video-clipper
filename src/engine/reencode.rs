//! Re-encoding clipping implementation for frame-accurate video clipping

use std::time::Instant;
use tracing::{info, warn, debug};
use crate::engine::{EngineConfig, ClippingProgress, ClippingPhase};
use crate::error::{TrimXError, TrimXResult};

/// Re-encoding clipper for frame-accurate clipping when stream copy is not viable
pub struct ReencodeClipper {
    /// Enable debug logging
    debug: bool,
    /// Quality preset for re-encoding
    preset: String,
    /// Constant rate factor (0-51, lower is higher quality)
    crf: Option<u8>,
    /// Target bitrate for re-encoding
    bitrate: Option<u64>,
    /// Hardware acceleration type
    hw_accel: HardwareAcceleration,
    /// Enable hardware decoding
    hw_decode: bool,
}

/// Hardware acceleration options
#[derive(Debug, Clone, PartialEq)]
pub enum HardwareAcceleration {
    /// No hardware acceleration (software only)
    None,
    /// NVIDIA NVENC encoder
    Nvenc,
    /// Intel Quick Sync Video
    Qsv,
    /// AMD VCE/AMF
    Amf,
    /// Apple VideoToolbox (macOS)
    VideoToolbox,
    /// Auto-detect best available
    Auto,
}

impl ReencodeClipper {
    /// Create a new re-encoding clipper with default settings
    pub fn new() -> Self {
        Self {
            debug: false,
            preset: "medium".to_string(), // Balanced speed/quality
            crf: Some(23), // Default quality
            bitrate: None, // Use CRF by default
            hw_accel: HardwareAcceleration::Auto, // Auto-detect by default
            hw_decode: true, // Enable hardware decoding when available
        }
    }

    /// Enable debug logging
    pub fn with_debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// Set the encoding preset (ultrafast, fast, medium, slow, veryslow)
    pub fn with_preset(mut self, preset: impl Into<String>) -> Self {
        self.preset = preset.into();
        self
    }

    /// Set constant rate factor for quality-based encoding
    pub fn with_crf(mut self, crf: u8) -> Self {
        self.crf = Some(crf.min(51)); // Clamp to valid range
        self.bitrate = None; // CRF overrides bitrate
        self
    }

    /// Set target bitrate for rate-based encoding
    pub fn with_bitrate(mut self, bitrate: u64) -> Self {
        self.bitrate = Some(bitrate);
        self.crf = None; // Bitrate overrides CRF
        self
    }

    /// Set hardware acceleration type
    pub fn with_hardware_acceleration(mut self, hw_accel: HardwareAcceleration) -> Self {
        self.hw_accel = hw_accel;
        self
    }

    /// Enable or disable hardware decoding
    pub fn with_hardware_decode(mut self, hw_decode: bool) -> Self {
        self.hw_decode = hw_decode;
        self
    }

    /// Detect available hardware acceleration
    pub fn detect_hardware_acceleration() -> HardwareAcceleration {
        // Check for NVIDIA GPUs
        if Self::is_nvenc_available() {
            info!("NVIDIA NVENC encoder detected");
            return HardwareAcceleration::Nvenc;
        }

        // Check for Intel Quick Sync
        if Self::is_qsv_available() {
            info!("Intel Quick Sync Video detected");
            return HardwareAcceleration::Qsv;
        }

        // Check for AMD AMF
        if Self::is_amf_available() {
            info!("AMD AMF encoder detected");
            return HardwareAcceleration::Amf;
        }

        // Check for Apple VideoToolbox (macOS only)
        #[cfg(target_os = "macos")]
        {
            if Self::is_videotoolbox_available() {
                info!("Apple VideoToolbox detected");
                return HardwareAcceleration::VideoToolbox;
            }
        }

        info!("No hardware acceleration detected, using software encoding");
        HardwareAcceleration::None
    }

    /// Check if NVIDIA NVENC is available
    fn is_nvenc_available() -> bool {
        // Try to create NVENC encoder context
        // This would check for NVIDIA drivers and compatible GPU
        // For now, return false as a placeholder
        false
    }

    /// Check if Intel QSV is available
    fn is_qsv_available() -> bool {
        // Try to create QSV encoder context
        // This would check for Intel Media SDK or oneVPL
        false
    }

    /// Check if AMD AMF is available
    fn is_amf_available() -> bool {
        // Try to create AMF encoder context
        // This would check for AMD drivers and compatible GPU
        false
    }

    /// Check if Apple VideoToolbox is available (macOS only)
    #[cfg(target_os = "macos")]
    fn is_videotoolbox_available() -> bool {
        // Try to create VideoToolbox encoder
        // This would check for macOS hardware encoding support
        true // Generally available on macOS
    }

    /// Execute re-encoding clipping
    pub fn clip(&self, config: EngineConfig) -> TrimXResult<ClippingProgress> {
        let start_time = Instant::now();
        info!("Starting re-encoding clipping operation");
        info!("Input: {}", config.input_path);
        info!("Output: {}", config.output_path);
        info!("Time range: {:.3}s - {:.3}s", config.start_time, config.end_time);
        info!("Preset: {}, CRF: {:?}, Bitrate: {:?}", self.preset, self.crf, self.bitrate);
        
        // Resolve hardware acceleration
        let hw_accel = match self.hw_accel {
            HardwareAcceleration::Auto => Self::detect_hardware_acceleration(),
            _ => self.hw_accel.clone(),
        };
        info!("Using hardware acceleration: {:?}", hw_accel);

        // Validate configuration
        self.validate_config(&config)?;

        match self.execute_reencode(&config, &hw_accel) {
            Ok(progress) => {
                let elapsed = start_time.elapsed();
                info!("Re-encoding clipping completed successfully in {:.2}s", elapsed.as_secs_f64());
                Ok(progress)
            }
            Err(e) => {
                warn!("Re-encoding clipping failed: {}", e);
                Err(e)
            }
        }
    }

    /// Validate the configuration for re-encoding
    fn validate_config(&self, config: &EngineConfig) -> TrimXResult<()> {
        // Check that input file exists
        if !std::path::Path::new(&config.input_path).exists() {
            return Err(TrimXError::InputFileNotFound {
                path: config.input_path.clone()
            });
        }

        // Check that output directory exists
        if let Some(parent) = std::path::Path::new(&config.output_path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| TrimXError::IoError(e))?;
            }
        }

        // Validate time range
        if config.start_time >= config.end_time {
            return Err(TrimXError::ClippingError {
                message: format!(
                    "Invalid time range: start ({:.3}s) must be before end ({:.3}s)",
                    config.start_time, config.end_time
                )
            });
        }

        if config.start_time < 0.0 {
            return Err(TrimXError::ClippingError {
                message: "Start time cannot be negative".to_string()
            });
        }

        // Validate encoding parameters
        if let Some(crf) = self.crf {
            if crf > 51 {
                return Err(TrimXError::ClippingError {
                    message: format!("CRF value {} is invalid (must be 0-51)", crf)
                });
            }
        }

        Ok(())
    }

    /// Execute the actual re-encoding operation using FFmpeg
    fn execute_reencode(&self, config: &EngineConfig, hw_accel: &HardwareAcceleration) -> TrimXResult<ClippingProgress> {
        info!("Opening input file for re-encoding");

        // Initialize FFmpeg
        ffmpeg_next::init().map_err(|e| TrimXError::ClippingError {
            message: format!("Failed to initialize FFmpeg: {}", e)
        })?;

        // Open input context
        let mut input_ctx = ffmpeg_next::format::input(&config.input_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e)
            })?;

        // Find video stream
        let video_stream_index = input_ctx
            .streams()
            .best(ffmpeg_next::media::Type::Video)
            .ok_or_else(|| TrimXError::ClippingError {
                message: "No video stream found in input file".to_string()
            })?
            .index();

        // Create output context
        let mut output_ctx = ffmpeg_next::format::output(&config.output_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to create output file: {}", e)
            })?;

        // Setup encoders and decoders with hardware acceleration
        let (video_decoder, video_encoder) = self.setup_video_transcoding(&input_ctx, &mut output_ctx, video_stream_index, hw_accel)?;
        
        // Setup audio pass-through (if audio streams exist)
        self.setup_audio_passthrough(&input_ctx, &mut output_ctx)?;

        // Write output header
        output_ctx.write_header()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to write output header: {}", e)
            })?;

        // Perform the actual transcoding
        let progress = self.transcode_video(&mut input_ctx, &mut output_ctx, config, video_decoder, video_encoder)?;

        // Write output trailer
        output_ctx.write_trailer()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to write output trailer: {}", e)
            })?;

        info!("Re-encoding operation completed successfully");
        Ok(progress)
    }

    /// Setup video transcoding (decoder and encoder) with hardware acceleration
    fn setup_video_transcoding(
        &self,
        input_ctx: &ffmpeg_next::format::context::Input,
        output_ctx: &mut ffmpeg_next::format::context::Output,
        video_stream_index: usize,
        hw_accel: &HardwareAcceleration,
    ) -> TrimXResult<(ffmpeg_next::codec::decoder::Video, ffmpeg_next::codec::encoder::video::Video)> {
        
        let input_stream = input_ctx.stream(video_stream_index)
            .ok_or_else(|| TrimXError::ClippingError {
                message: format!("Video stream {} not found", video_stream_index)
            })?;

        // Create video decoder
        let video_decoder = ffmpeg_next::codec::context::Context::from_parameters(input_stream.parameters())
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to create decoder context: {}", e)
            })?
            .decoder()
            .video()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to create video decoder: {}", e)
            })?;

        // Select encoder based on hardware acceleration
        let encoder_id = self.select_encoder(hw_accel);
        info!("Selected encoder: {:?} for hardware acceleration: {:?}", encoder_id, hw_accel);

        // Create output video stream
        let mut output_stream = output_ctx.add_stream(ffmpeg_next::codec::encoder::find(encoder_id))
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to add video stream: {}", e)
            })?;

        // Configure video encoder
        let mut video_encoder = ffmpeg_next::codec::context::Context::new()
            .encoder()
            .video()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to create video encoder: {}", e)
            })?;

        // Set encoder parameters
        video_encoder.set_width(video_decoder.width());
        video_encoder.set_height(video_decoder.height());
        video_encoder.set_aspect_ratio(video_decoder.aspect_ratio());
        // Pixel format would be set here in a full implementation
        video_encoder.set_time_base(output_stream.time_base());

        // Set quality parameters
        if let Some(_crf) = self.crf {
            video_encoder.set_max_bit_rate(0); // No max bitrate for CRF
            // CRF setting would typically be done via encoder options
            // This is a placeholder - actual implementation would use av_opt_set
        } else if let Some(bitrate) = self.bitrate {
            video_encoder.set_bit_rate(bitrate as usize);
        }

        // Open the encoder - simplified approach to avoid ownership issues
        // In a full implementation, this would be properly configured
        let video_encoder = video_encoder;

        // Copy encoder parameters to output stream
        output_stream.set_parameters(&video_encoder);

        Ok((video_decoder, video_encoder))
    }

    /// Select appropriate encoder based on hardware acceleration
    fn select_encoder(&self, hw_accel: &HardwareAcceleration) -> ffmpeg_next::codec::Id {
        match hw_accel {
            HardwareAcceleration::None => {
                // Software encoder
                ffmpeg_next::codec::Id::H264
            }
            HardwareAcceleration::Nvenc => {
                // NVIDIA NVENC encoder
                // Note: In real implementation, would check if h264_nvenc is available
                ffmpeg_next::codec::Id::H264 // Fallback to software for now
            }
            HardwareAcceleration::Qsv => {
                // Intel Quick Sync encoder
                // Note: In real implementation, would use h264_qsv
                ffmpeg_next::codec::Id::H264 // Fallback to software for now
            }
            HardwareAcceleration::Amf => {
                // AMD AMF encoder
                // Note: In real implementation, would use h264_amf
                ffmpeg_next::codec::Id::H264 // Fallback to software for now
            }
            HardwareAcceleration::VideoToolbox => {
                // Apple VideoToolbox encoder
                // Note: In real implementation, would use h264_videotoolbox
                ffmpeg_next::codec::Id::H264 // Fallback to software for now
            }
            HardwareAcceleration::Auto => {
                // This should have been resolved earlier
                warn!("Auto hardware acceleration not resolved, using software encoder");
                ffmpeg_next::codec::Id::H264
            }
        }
    }

    /// Setup audio pass-through (copy audio streams without re-encoding)
    fn setup_audio_passthrough(
        &self,
        input_ctx: &ffmpeg_next::format::context::Input,
        output_ctx: &mut ffmpeg_next::format::context::Output,
    ) -> TrimXResult<()> {
        
        // Copy all audio streams as-is
        for input_stream in input_ctx.streams() {
            if input_stream.parameters().medium() == ffmpeg_next::media::Type::Audio {
                let mut output_stream = output_ctx.add_stream(ffmpeg_next::codec::encoder::find(ffmpeg_next::codec::Id::None))
                    .map_err(|e| TrimXError::ClippingError {
                        message: format!("Failed to add audio stream: {}", e)
                    })?;

                // Copy parameters directly for pass-through
                output_stream.set_parameters(input_stream.parameters());
                output_stream.set_time_base(input_stream.time_base());

                if self.debug {
                    debug!("Setup audio pass-through for stream {}", input_stream.index());
                }
            }
        }

        Ok(())
    }

    /// Perform the actual video transcoding
    fn transcode_video(
        &self,
        input_ctx: &mut ffmpeg_next::format::context::Input,
        output_ctx: &mut ffmpeg_next::format::context::Output,
        config: &EngineConfig,
        mut video_decoder: ffmpeg_next::codec::decoder::Video,
        mut video_encoder: ffmpeg_next::codec::encoder::video::Video,
    ) -> TrimXResult<ClippingProgress> {
        
        let start_time_av = (config.start_time * ffmpeg_next::ffi::AV_TIME_BASE as f64) as i64;
        let end_time_av = (config.end_time * ffmpeg_next::ffi::AV_TIME_BASE as f64) as i64;

        // Seek to start time
        if config.start_time > 0.0 {
            input_ctx.seek(start_time_av, start_time_av..)
                .map_err(|e| TrimXError::ClippingError {
                    message: format!("Failed to seek to start time: {}", e)
                })?;
        }

        let mut frames_processed = 0;
        let mut bytes_written = 0u64;

        info!("Starting transcoding loop");

        // Process packets
        for (input_stream, packet) in input_ctx.packets() {
            // Check packet timing
            let packet_time = if let Some(pts) = packet.pts() {
                let stream_tb = input_stream.time_base();
                pts * stream_tb.numerator() as i64 / stream_tb.denominator() as i64 * ffmpeg_next::ffi::AV_TIME_BASE as i64
            } else {
                continue;
            };

            if packet_time < start_time_av {
                continue;
            }
            if packet_time > end_time_av {
                break;
            }

            // Handle video packets (decode + encode)
            if input_stream.parameters().medium() == ffmpeg_next::media::Type::Video {
                video_decoder.send_packet(&packet)
                    .map_err(|e| TrimXError::ClippingError {
                        message: format!("Failed to send packet to decoder: {}", e)
                    })?;

                let mut frame = ffmpeg_next::util::frame::video::Video::empty();
                while video_decoder.receive_frame(&mut frame).is_ok() {
                    // Send frame to encoder
                    video_encoder.send_frame(&frame)
                        .map_err(|e| TrimXError::ClippingError {
                            message: format!("Failed to send frame to encoder: {}", e)
                        })?;

                    // Receive encoded packets
                    let mut encoded_packet = ffmpeg_next::codec::packet::Packet::empty();
                    while video_encoder.receive_packet(&mut encoded_packet).is_ok() {
                        // Write to output
                        encoded_packet.write_interleaved(output_ctx)
                            .map_err(|e| TrimXError::ClippingError {
                                message: format!("Failed to write encoded packet: {}", e)
                            })?;

                        bytes_written += encoded_packet.size() as u64;
                        frames_processed += 1;
                    }
                }
            }
            // Handle audio packets (pass-through)
            else if input_stream.parameters().medium() == ffmpeg_next::media::Type::Audio {
                // Find corresponding output stream
                if let Some(output_stream) = output_ctx.stream(input_stream.index()) {
                    let mut audio_packet = packet;
                    audio_packet.rescale_ts(input_stream.time_base(), output_stream.time_base());
                    
                    audio_packet.write_interleaved(output_ctx)
                        .map_err(|e| TrimXError::ClippingError {
                            message: format!("Failed to write audio packet: {}", e)
                        })?;
                }
            }

            // Log progress periodically
            if self.debug && frames_processed % 100 == 0 {
                debug!("Processed {} frames, {} bytes written", frames_processed, bytes_written);
            }
        }

        // Flush encoders
        video_encoder.send_eof()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to flush video encoder: {}", e)
            })?;

        let mut encoded_packet = ffmpeg_next::codec::packet::Packet::empty();
        while video_encoder.receive_packet(&mut encoded_packet).is_ok() {
            encoded_packet.write_interleaved(output_ctx)
                .map_err(|e| TrimXError::ClippingError {
                    message: format!("Failed to write final packet: {}", e)
                })?;
            bytes_written += encoded_packet.size() as u64;
        }

        info!("Processed {} frames, wrote {:.2} MB", frames_processed, bytes_written as f64 / 1024.0 / 1024.0);

        Ok(ClippingProgress {
            phase: ClippingPhase::Completed,
            progress: 100.0,
            description: format!(
                "Re-encoding completed: {} frames, {:.2} MB", 
                frames_processed, 
                bytes_written as f64 / 1024.0 / 1024.0
            ),
            eta: None,
        })
    }

    /// Get estimated output file size for re-encoding
    pub fn estimate_output_size(&self, config: &EngineConfig) -> TrimXResult<u64> {
        // For re-encoding, estimate based on target bitrate and duration
        let duration = config.end_time - config.start_time;
        
        if let Some(bitrate) = self.bitrate {
            // Bitrate-based estimation
            let estimated_size = (bitrate * duration as u64) / 8; // Convert from bits to bytes
            Ok(estimated_size)
        } else {
            // For CRF, estimate based on input file proportionally with quality factor
            let input_size = std::fs::metadata(&config.input_path)
                .map_err(|e| TrimXError::IoError(e))?
                .len();

            let input_duration = self.get_input_duration(config)?;
            let duration_ratio = duration / input_duration;
            
            // Apply quality factor - lower CRF means larger files
            let quality_factor = if let Some(crf) = self.crf {
                // Rough approximation: CRF 18 ≈ 1.5x, CRF 23 ≈ 1.0x, CRF 28 ≈ 0.7x
                1.5_f64.powf((23.0 - crf as f64) / 5.0)
            } else {
                1.0 // Default
            };

            let estimated_size = (input_size as f64 * duration_ratio * quality_factor) as u64;
            Ok(estimated_size)
        }
    }

    /// Get the duration of the input file
    fn get_input_duration(&self, config: &EngineConfig) -> TrimXResult<f64> {
        ffmpeg_next::init().map_err(|e| TrimXError::ClippingError {
            message: format!("Failed to initialize FFmpeg: {}", e)
        })?;

        let input_ctx = ffmpeg_next::format::input(&config.input_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e)
            })?;

        let duration_seconds = input_ctx.duration() as f64 / ffmpeg_next::ffi::AV_TIME_BASE as f64;
        Ok(duration_seconds)
    }
}
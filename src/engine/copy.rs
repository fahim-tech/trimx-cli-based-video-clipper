//! Stream copy implementation

use std::path::Path;
use tracing::{info, warn, error};
use ffmpeg_next as ffmpeg;

use crate::engine::EngineConfig;
use crate::error::{TrimXError, TrimXResult};

/// Stream copy clipper for lossless operations
pub struct StreamCopyClipper;

impl StreamCopyClipper {
    /// Create a new stream copy clipper
    pub fn new() -> Self {
        Self
    }

    /// Execute stream copy clipping
    pub fn clip(&self, config: EngineConfig) -> TrimXResult<()> {
        info!("Starting stream copy clipping");
        info!("Input: {}", config.input_path);
        info!("Output: {}", config.output_path);
        info!("Time range: {:.3}s - {:.3}s", config.start_time, config.end_time);

        // Initialize FFmpeg
        ffmpeg::init().map_err(|e| TrimXError::ClippingError {
            message: format!("FFmpeg initialization failed: {}", e),
        })?;

        // Open input context
        let mut ictx = ffmpeg::format::input(&config.input_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e),
            })?;

        // Create output context
        let mut octx = ffmpeg::format::output(&config.output_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to create output file: {}", e),
            })?;

        // Set output format
        octx.set_metadata(ictx.metadata().to_owned());

        // Find streams and set up mapping
        let mut stream_mapping = Vec::new();
        let mut stream_index = 0;

        for (i, stream) in ictx.streams().enumerate() {
            let codec = stream.codec();
            let codec_id = codec.id();

            // Check if we should include this stream
            if self.should_include_stream(&config, &stream) {
                // Create output stream
                let mut out_stream = octx.add_stream(codec_id)
                    .map_err(|e| TrimXError::ClippingError {
                        message: format!("Failed to add output stream: {}", e),
                    })?;

                // Copy codec parameters
                out_stream.set_parameters(stream.parameters());
                out_stream.set_time_base(stream.time_base());

                stream_mapping.push((i, stream_index));
                stream_index += 1;
            }
        }

        // Write header
        octx.write_header()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to write output header: {}", e),
            })?;

        // Convert time to AV_TIME_BASE
        let start_pts = (config.start_time * ffmpeg::ffi::AV_TIME_BASE as f64) as i64;
        let end_pts = (config.end_time * ffmpeg::ffi::AV_TIME_BASE as f64) as i64;

        // Seek to start time
        ictx.seek(start_pts, start_pts..)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to seek to start time: {}", e),
            })?;

        // Process packets
        let mut packet_count = 0;
        for (stream, packet) in ictx.packets() {
            // Check if we should process this stream
            if let Some((_, out_stream_index)) = stream_mapping.iter().find(|(in_idx, _)| *in_idx == stream.index()) {
                // Check if packet is within time range
                let packet_pts = packet.pts().unwrap_or(0);
                let packet_time = packet_pts as f64 * stream.time_base().denominator() as f64 / stream.time_base().numerator() as f64;

                if packet_time >= config.start_time && packet_time <= config.end_time {
                    // Adjust packet timestamps
                    let mut out_packet = packet.clone();
                    out_packet.set_stream(*out_stream_index);
                    
                    // Adjust PTS/DTS for the new time base
                    if let Some(pts) = out_packet.pts() {
                        out_packet.set_pts(Some(pts - start_pts));
                    }
                    if let Some(dts) = out_packet.dts() {
                        out_packet.set_dts(Some(dts - start_pts));
                    }

                    // Write packet
                    octx.write_packet(&out_packet)
                        .map_err(|e| TrimXError::ClippingError {
                            message: format!("Failed to write packet: {}", e),
                        })?;

                    packet_count += 1;
                }
            }
        }

        // Write trailer
        octx.write_trailer()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to write output trailer: {}", e),
            })?;

        info!("Stream copy clipping completed successfully");
        info!("Processed {} packets", packet_count);

        Ok(())
    }

    /// Check if stream copy is possible for the given configuration
    pub fn is_possible(&self, config: &EngineConfig) -> bool {
        // Check if input file exists
        if !Path::new(&config.input_path).exists() {
            return false;
        }

        // Check if output directory exists and is writable
        if let Some(parent) = Path::new(&config.output_path).parent() {
            if !parent.exists() || !parent.is_dir() {
                return false;
            }
        }

        // For now, assume stream copy is possible for most formats
        // In a more sophisticated implementation, we would check:
        // 1. Container compatibility
        // 2. Codec support
        // 3. Timestamp precision
        // 4. Stream characteristics

        true
    }

    /// Determine if a stream should be included in the output
    fn should_include_stream(&self, config: &EngineConfig, stream: &ffmpeg::Stream) -> bool {
        let codec = stream.codec();
        let codec_type = codec.medium();

        match codec_type {
            ffmpeg::media::Type::Video => true, // Always include video
            ffmpeg::media::Type::Audio => !config.no_audio,
            ffmpeg::media::Type::Subtitle => !config.no_subs,
            _ => false, // Skip other stream types
        }
    }
}

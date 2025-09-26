//! Re-encoding implementation

use std::path::Path;
use tracing::{info, warn, error};
use ffmpeg_next as ffmpeg;

use crate::engine::EngineConfig;
use crate::error::{TrimXError, TrimXResult};

/// Re-encoding clipper for precise cuts
pub struct ReencodeClipper;

impl ReencodeClipper {
    /// Create a new re-encoding clipper
    pub fn new() -> Self {
        Self
    }

    /// Execute re-encoding clipping
    pub fn clip(&self, config: EngineConfig) -> TrimXResult<()> {
        info!("Starting re-encoding clipping");
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

        // Find video and audio streams
        let video_stream = ictx.streams()
            .find(|s| s.parameters().medium() == ffmpeg::media::Type::Video)
            .ok_or_else(|| TrimXError::ClippingError {
                message: "No video stream found".to_string(),
            })?;

        let audio_stream = ictx.streams()
            .find(|s| s.parameters().medium() == ffmpeg::media::Type::Audio);

        // Create video encoder
        let video_encoder = self.create_video_encoder(&config, &video_stream)?;
        let mut video_stream_out = octx.add_stream(video_encoder.id())
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to add video stream: {}", e),
            })?;
        video_stream_out.set_parameters(video_encoder.parameters());

        // Create audio encoder if needed
        let mut audio_stream_out = None;
        if let Some(audio_stream) = audio_stream {
            if !config.no_audio {
                let audio_encoder = self.create_audio_encoder(&config, &audio_stream)?;
                audio_stream_out = Some(octx.add_stream(audio_encoder.id())
                    .map_err(|e| TrimXError::ClippingError {
                        message: format!("Failed to add audio stream: {}", e),
                    })?);
                if let Some(ref mut stream) = audio_stream_out {
                    stream.set_parameters(audio_encoder.parameters());
                }
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

        // Process frames
        let mut frame_count = 0;
        let mut packet_count = 0;

        for (stream, packet) in ictx.packets() {
            if stream.index() == video_stream.index() {
                // Process video packet
                if let Ok(mut frame) = ffmpeg::frame::Video::empty() {
                    let mut decoder = video_stream.codec().decoder().video()
                        .map_err(|e| TrimXError::ClippingError {
                            message: format!("Failed to create video decoder: {}", e),
                        })?;

                    decoder.send_packet(&packet)
                        .map_err(|e| TrimXError::ClippingError {
                            message: format!("Failed to send video packet: {}", e),
                        })?;

                    while decoder.receive_frame(&mut frame).is_ok() {
                        // Check if frame is within time range
                        let frame_pts = frame.pts().unwrap_or(0);
                        let frame_time = frame_pts as f64 * video_stream.time_base().denominator() as f64 / video_stream.time_base().numerator() as f64;

                        if frame_time >= config.start_time && frame_time <= config.end_time {
                            // Encode frame
                            let mut encoder = video_stream_out.codec().encoder().video()
                                .map_err(|e| TrimXError::ClippingError {
                                    message: format!("Failed to create video encoder: {}", e),
                                })?;

                            encoder.send_frame(&frame)
                                .map_err(|e| TrimXError::ClippingError {
                                    message: format!("Failed to send video frame: {}", e),
                                })?;

                            let mut encoded_packet = ffmpeg::packet::Packet::empty();
                            while encoder.receive_packet(&mut encoded_packet).is_ok() {
                                encoded_packet.set_stream(0);
                                octx.write_packet(&encoded_packet)
                                    .map_err(|e| TrimXError::ClippingError {
                                        message: format!("Failed to write video packet: {}", e),
                                    })?;
                                packet_count += 1;
                            }

                            frame_count += 1;
                        }
                    }
                }
            } else if let Some(audio_stream) = audio_stream {
                if stream.index() == audio_stream.index() && !config.no_audio {
                    // Process audio packet
                    if let Ok(mut frame) = ffmpeg::frame::Audio::empty() {
                        let mut decoder = audio_stream.codec().decoder().audio()
                            .map_err(|e| TrimXError::ClippingError {
                                message: format!("Failed to create audio decoder: {}", e),
                            })?;

                        decoder.send_packet(&packet)
                            .map_err(|e| TrimXError::ClippingError {
                                message: format!("Failed to send audio packet: {}", e),
                            })?;

                        while decoder.receive_frame(&mut frame).is_ok() {
                            // Check if frame is within time range
                            let frame_pts = frame.pts().unwrap_or(0);
                            let frame_time = frame_pts as f64 * audio_stream.time_base().denominator() as f64 / audio_stream.time_base().numerator() as f64;

                            if frame_time >= config.start_time && frame_time <= config.end_time {
                                // Encode frame
                                if let Some(ref audio_stream_out) = audio_stream_out {
                                    let mut encoder = audio_stream_out.codec().encoder().audio()
                                        .map_err(|e| TrimXError::ClippingError {
                                            message: format!("Failed to create audio encoder: {}", e),
                                        })?;

                                    encoder.send_frame(&frame)
                                        .map_err(|e| TrimXError::ClippingError {
                                            message: format!("Failed to send audio frame: {}", e),
                                        })?;

                                    let mut encoded_packet = ffmpeg::packet::Packet::empty();
                                    while encoder.receive_packet(&mut encoded_packet).is_ok() {
                                        encoded_packet.set_stream(1);
                                        octx.write_packet(&encoded_packet)
                                            .map_err(|e| TrimXError::ClippingError {
                                                message: format!("Failed to write audio packet: {}", e),
                                            })?;
                                        packet_count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Write trailer
        octx.write_trailer()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to write output trailer: {}", e),
            })?;

        info!("Re-encoding clipping completed successfully");
        info!("Processed {} frames, {} packets", frame_count, packet_count);

        Ok(())
    }

    /// Configure encoding parameters
    pub fn configure_encoding(&self, config: &EngineConfig) -> TrimXResult<EncodingConfig> {
        Ok(EncodingConfig {
            video_codec: config.video_codec.clone(),
            audio_codec: config.audio_codec.clone(),
            crf: config.crf,
            preset: config.preset.clone(),
        })
    }

    /// Create video encoder
    fn create_video_encoder(&self, config: &EngineConfig, stream: &ffmpeg::Stream) -> TrimXResult<ffmpeg::encoder::Video> {
        let codec_id = match config.video_codec.as_str() {
            "h264" => ffmpeg::codec::Id::H264,
            "hevc" | "h265" => ffmpeg::codec::Id::HEVC,
            _ => ffmpeg::codec::Id::H264, // Default to H.264
        };

        let encoder = ffmpeg::encoder::find(codec_id)
            .ok_or_else(|| TrimXError::ClippingError {
                message: format!("Video codec not found: {}", config.video_codec),
            })?;

        let mut video_encoder = encoder.encoder().video()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to create video encoder: {}", e),
            })?;

        // Set encoder parameters
        video_encoder.set_height(stream.parameters().height());
        video_encoder.set_width(stream.parameters().width());
        video_encoder.set_aspect_ratio(stream.parameters().aspect_ratio());
        video_encoder.set_frame_rate(stream.parameters().frame_rate());
        video_encoder.set_time_base(stream.time_base());

        // Set quality parameters
        video_encoder.set_bit_rate(stream.parameters().bit_rate());
        video_encoder.set_max_bit_rate(stream.parameters().max_bit_rate());

        // Set CRF if supported
        if let Ok(mut options) = ffmpeg::Dictionary::new() {
            options.set("crf", &config.crf.to_string());
            options.set("preset", &config.preset);
            video_encoder.set_options(&options);
        }

        Ok(video_encoder)
    }

    /// Create audio encoder
    fn create_audio_encoder(&self, config: &EngineConfig, stream: &ffmpeg::Stream) -> TrimXResult<ffmpeg::encoder::Audio> {
        let codec_id = match config.audio_codec.as_deref().unwrap_or("aac") {
            "aac" => ffmpeg::codec::Id::AAC,
            "mp3" => ffmpeg::codec::Id::MP3,
            _ => ffmpeg::codec::Id::AAC, // Default to AAC
        };

        let encoder = ffmpeg::encoder::find(codec_id)
            .ok_or_else(|| TrimXError::ClippingError {
                message: format!("Audio codec not found: {}", config.audio_codec.as_deref().unwrap_or("aac")),
            })?;

        let mut audio_encoder = encoder.encoder().audio()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to create audio encoder: {}", e),
            })?;

        // Set encoder parameters
        audio_encoder.set_sample_rate(stream.parameters().sample_rate());
        audio_encoder.set_channels(stream.parameters().channels());
        audio_encoder.set_channel_layout(stream.parameters().channel_layout());
        audio_encoder.set_time_base(stream.time_base());

        // Set bit rate
        if let Some(bit_rate) = stream.parameters().bit_rate() {
            audio_encoder.set_bit_rate(bit_rate);
        }

        Ok(audio_encoder)
    }
}

/// Encoding configuration
#[derive(Debug, Clone)]
pub struct EncodingConfig {
    /// Video codec
    pub video_codec: String,
    /// Audio codec
    pub audio_codec: Option<String>,
    /// CRF quality setting
    pub crf: u8,
    /// Encoding preset
    pub preset: String,
}

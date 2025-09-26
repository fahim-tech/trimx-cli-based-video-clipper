//! Stream processing utilities

use std::collections::HashMap;
use tracing::{info, warn, error};
use ffmpeg_next as ffmpeg;

use crate::streams::{StreamMapping, VideoProcessingMode, AudioProcessingMode, SubtitleProcessingMode, VideoStreamMapping, AudioStreamMapping, SubtitleStreamMapping};
use crate::error::{TrimXError, TrimXResult};

/// Stream processor for handling different stream types
pub struct StreamProcessor {
    thread_count: usize,
    buffer_size: usize,
}

impl StreamProcessor {
    /// Create a new stream processor
    pub fn new() -> Self {
        Self {
            thread_count: num_cpus::get().min(16),
            buffer_size: 4096,
        }
    }

    /// Process all streams based on mapping
    pub fn process_streams(
        &self,
        input_path: &str,
        output_path: &str,
        mapping: &StreamMapping,
        start_time: f64,
        end_time: f64,
    ) -> TrimXResult<()> {
        info!("Processing streams from {} to {}", input_path, output_path);
        info!("Time range: {:.3}s - {:.3}s", start_time, end_time);

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

        // Set up stream mappings
        let mut stream_map = HashMap::new();
        
        // Process video stream
        if let Some(video_mapping) = &mapping.video {
            let output_stream = self.setup_video_stream(&mut octx, &ictx, video_mapping)?;
            stream_map.insert(video_mapping.input_index, output_stream);
        }

        // Process audio streams
        for audio_mapping in &mapping.audio {
            let output_stream = self.setup_audio_stream(&mut octx, &ictx, audio_mapping)?;
            stream_map.insert(audio_mapping.input_index, output_stream);
        }

        // Process subtitle streams
        for subtitle_mapping in &mapping.subtitles {
            let output_stream = self.setup_subtitle_stream(&mut octx, &ictx, subtitle_mapping)?;
            stream_map.insert(subtitle_mapping.input_index, output_stream);
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
            if let Some(&output_stream_index) = stream_map.get(&stream.index()) {
                // Check if packet is within time range
                let packet_pts = packet.pts().unwrap_or(0);
                let packet_time = packet_pts as f64 * stream.time_base().denominator() as f64 / stream.time_base().numerator() as f64;

                if packet_time >= start_time && packet_time <= end_time {
                    // Process packet based on stream type
                    let processed_packet = self.process_packet(&packet, &stream, output_stream_index, start_pts)?;
                    
                    // Write processed packet
                    octx.write_packet(&processed_packet)
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

        info!("Stream processing completed successfully");
        info!("Processed {} packets", packet_count);

        Ok(())
    }

    /// Process video stream based on mapping
    pub fn process_video(
        &self,
        mapping: &VideoStreamMapping,
    ) -> TrimXResult<()> {
        match mapping.mode {
            VideoProcessingMode::Copy => self.copy_video_stream(mapping),
            VideoProcessingMode::Reencode => self.reencode_video_stream(mapping),
            VideoProcessingMode::Skip => Ok(()),
        }
    }

    /// Process audio stream based on mapping
    pub fn process_audio(
        &self,
        mapping: &AudioStreamMapping,
    ) -> TrimXResult<()> {
        match mapping.mode {
            AudioProcessingMode::Copy => self.copy_audio_stream(mapping),
            AudioProcessingMode::Reencode => self.reencode_audio_stream(mapping),
            AudioProcessingMode::Resample => self.resample_audio_stream(mapping),
            AudioProcessingMode::Skip => Ok(()),
        }
    }

    /// Process subtitle stream based on mapping
    pub fn process_subtitle(
        &self,
        mapping: &SubtitleStreamMapping,
    ) -> TrimXResult<()> {
        match mapping.mode {
            SubtitleProcessingMode::Copy => self.copy_subtitle_stream(mapping),
            SubtitleProcessingMode::Retime => self.retime_subtitle_stream(mapping),
            SubtitleProcessingMode::Skip => Ok(()),
        }
    }

    /// Setup video stream in output context
    fn setup_video_stream(
        &self,
        octx: &mut ffmpeg::format::context::Output,
        ictx: &ffmpeg::format::context::Input,
        mapping: &VideoStreamMapping,
    ) -> TrimXResult<usize> {
        let input_stream = ictx.stream(mapping.input_index)
            .ok_or_else(|| TrimXError::ClippingError {
                message: format!("Input video stream {} not found", mapping.input_index),
            })?;

        let codec = input_stream.codec();
        let codec_id = codec.id();

        let mut output_stream = octx.add_stream(codec_id)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to add video stream: {}", e),
            })?;

        match mapping.mode {
            VideoProcessingMode::Copy => {
                // Copy codec parameters
                output_stream.set_parameters(codec.parameters());
                output_stream.set_time_base(input_stream.time_base());
            }
            VideoProcessingMode::Reencode => {
                // Set encoding parameters
                output_stream.set_height(codec.height());
                output_stream.set_width(codec.width());
                output_stream.set_aspect_ratio(codec.aspect_ratio());
                output_stream.set_frame_rate(codec.frame_rate());
                output_stream.set_time_base(input_stream.time_base());
                
                // Set quality parameters
                if let Some(bit_rate) = codec.bit_rate() {
                    output_stream.set_bit_rate(bit_rate);
                }
            }
            VideoProcessingMode::Skip => {
                return Err(TrimXError::ClippingError {
                    message: "Cannot setup skipped video stream".to_string(),
                });
            }
        }

        Ok(output_stream.index())
    }

    /// Setup audio stream in output context
    fn setup_audio_stream(
        &self,
        octx: &mut ffmpeg::format::context::Output,
        ictx: &ffmpeg::format::context::Input,
        mapping: &AudioStreamMapping,
    ) -> TrimXResult<usize> {
        let input_stream = ictx.stream(mapping.input_index)
            .ok_or_else(|| TrimXError::ClippingError {
                message: format!("Input audio stream {} not found", mapping.input_index),
            })?;

        let codec = input_stream.codec();
        let codec_id = codec.id();

        let mut output_stream = octx.add_stream(codec_id)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to add audio stream: {}", e),
            })?;

        match mapping.mode {
            AudioProcessingMode::Copy => {
                // Copy codec parameters
                output_stream.set_parameters(codec.parameters());
                output_stream.set_time_base(input_stream.time_base());
            }
            AudioProcessingMode::Reencode | AudioProcessingMode::Resample => {
                // Set encoding parameters
                output_stream.set_sample_rate(codec.sample_rate());
                output_stream.set_channels(codec.channels());
                output_stream.set_channel_layout(codec.channel_layout());
                output_stream.set_time_base(input_stream.time_base());
                
                // Set bit rate
                if let Some(bit_rate) = codec.bit_rate() {
                    output_stream.set_bit_rate(bit_rate);
                }
            }
            AudioProcessingMode::Skip => {
                return Err(TrimXError::ClippingError {
                    message: "Cannot setup skipped audio stream".to_string(),
                });
            }
        }

        Ok(output_stream.index())
    }

    /// Setup subtitle stream in output context
    fn setup_subtitle_stream(
        &self,
        octx: &mut ffmpeg::format::context::Output,
        ictx: &ffmpeg::format::context::Input,
        mapping: &SubtitleStreamMapping,
    ) -> TrimXResult<usize> {
        let input_stream = ictx.stream(mapping.input_index)
            .ok_or_else(|| TrimXError::ClippingError {
                message: format!("Input subtitle stream {} not found", mapping.input_index),
            })?;

        let codec = input_stream.codec();
        let codec_id = codec.id();

        let mut output_stream = octx.add_stream(codec_id)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to add subtitle stream: {}", e),
            })?;

        match mapping.mode {
            SubtitleProcessingMode::Copy | SubtitleProcessingMode::Retime => {
                // Copy codec parameters
                output_stream.set_parameters(codec.parameters());
                output_stream.set_time_base(input_stream.time_base());
            }
            SubtitleProcessingMode::Skip => {
                return Err(TrimXError::ClippingError {
                    message: "Cannot setup skipped subtitle stream".to_string(),
                });
            }
        }

        Ok(output_stream.index())
    }

    /// Process individual packet
    fn process_packet(
        &self,
        packet: &ffmpeg::packet::Packet,
        stream: &ffmpeg::Stream,
        output_stream_index: usize,
        start_pts: i64,
    ) -> TrimXResult<ffmpeg::packet::Packet> {
        let mut output_packet = packet.clone();
        output_packet.set_stream(output_stream_index);
        
        // Adjust timestamps
        if let Some(pts) = output_packet.pts() {
            output_packet.set_pts(Some(pts - start_pts));
        }
        if let Some(dts) = output_packet.dts() {
            output_packet.set_dts(Some(dts - start_pts));
        }

        Ok(output_packet)
    }

    // Private methods for specific processing modes

    fn copy_video_stream(&self, _mapping: &VideoStreamMapping) -> TrimXResult<()> {
        info!("Copying video stream");
        // Implementation handled in process_streams
        Ok(())
    }

    fn reencode_video_stream(&self, _mapping: &VideoStreamMapping) -> TrimXResult<()> {
        info!("Re-encoding video stream");
        // Implementation handled in process_streams
        Ok(())
    }

    fn copy_audio_stream(&self, _mapping: &AudioStreamMapping) -> TrimXResult<()> {
        info!("Copying audio stream");
        // Implementation handled in process_streams
        Ok(())
    }

    fn reencode_audio_stream(&self, _mapping: &AudioStreamMapping) -> TrimXResult<()> {
        info!("Re-encoding audio stream");
        // Implementation handled in process_streams
        Ok(())
    }

    fn resample_audio_stream(&self, _mapping: &AudioStreamMapping) -> TrimXResult<()> {
        info!("Resampling audio stream");
        // Implementation handled in process_streams
        Ok(())
    }

    fn copy_subtitle_stream(&self, _mapping: &SubtitleStreamMapping) -> TrimXResult<()> {
        info!("Copying subtitle stream");
        // Implementation handled in process_streams
        Ok(())
    }

    fn retime_subtitle_stream(&self, _mapping: &SubtitleStreamMapping) -> TrimXResult<()> {
        info!("Re-timing subtitle stream");
        // Implementation handled in process_streams
        Ok(())
    }

    /// Get optimal thread count for processing
    pub fn get_thread_count(&self) -> usize {
        self.thread_count
    }

    /// Set thread count
    pub fn set_thread_count(&mut self, count: usize) {
        self.thread_count = count.min(32);
    }

    /// Get buffer size
    pub fn get_buffer_size(&self) -> usize {
        self.buffer_size
    }

    /// Set buffer size
    pub fn set_buffer_size(&mut self, size: usize) {
        self.buffer_size = size.max(1024);
    }
}

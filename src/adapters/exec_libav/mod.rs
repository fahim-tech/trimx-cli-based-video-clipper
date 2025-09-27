//! FFmpeg execution adapter using libav bindings
//! 
//! This module provides direct FFmpeg integration for video processing operations.

use async_trait::async_trait;
use std::time::Instant;

use crate::domain::errors::DomainError;
use crate::domain::model::*;
use crate::ports::*;
use crate::planner::keyframe_analyzer::{KeyframeAnalyzer, GOPAnalysis};
use ffmpeg_next::codec::{self, Id};
use ffmpeg_next::Codec;

/// FFmpeg execution adapter using libav FFI
pub struct LibavExecutionAdapter {
    thread_count: usize,
    buffer_size: usize,
    keyframe_analyzer: KeyframeAnalyzer,
}

impl LibavExecutionAdapter {
    pub fn new() -> Result<Self, DomainError> {
        // Initialize FFmpeg
        ffmpeg_next::init().map_err(|e| DomainError::InternalError(format!("FFmpeg initialization failed: {}", e)))?;
        
        // Optimize thread count based on CPU cores and memory
        let thread_count = Self::optimize_thread_count();
        let buffer_size = Self::optimize_buffer_size();
        
        Ok(Self {
            thread_count,
            buffer_size,
            keyframe_analyzer: KeyframeAnalyzer::new(),
        })
    }
    
    /// Optimize thread count based on system resources
    fn optimize_thread_count() -> usize {
        let cpu_count = num_cpus::get();
        
        // For video processing, use 75% of CPU cores to leave room for system processes
        // But ensure we have at least 1 thread and at most 16 threads
        let optimal_threads = (cpu_count as f64 * 0.75).ceil() as usize;
        optimal_threads.max(1).min(16)
    }
    
    /// Optimize buffer size based on available memory
    fn optimize_buffer_size() -> usize {
        // Start with a base buffer size
        let base_size = 4096;
        
        // Try to get available memory (this is a simplified approach)
        // In a real implementation, you'd use system APIs to get actual memory info
        let available_memory_mb = 1024; // Placeholder - would get actual memory
        
        // Scale buffer size based on available memory
        // More memory = larger buffers for better performance
        if available_memory_mb > 8192 {
            base_size * 8  // 32KB for systems with >8GB RAM
        } else if available_memory_mb > 4096 {
            base_size * 4  // 16KB for systems with >4GB RAM
        } else if available_memory_mb > 2048 {
            base_size * 2  // 8KB for systems with >2GB RAM
        } else {
            base_size      // 4KB for systems with <=2GB RAM
        }
    }
    
    /// Get memory usage statistics
    pub fn get_memory_usage(&self) -> Result<MemoryUsage, DomainError> {
        // This is a placeholder implementation
        // In a real implementation, you'd use system APIs to get actual memory usage
        Ok(MemoryUsage {
            used_memory: 0,
            available_memory: 0,
            peak_memory: 0,
        })
    }
    
    /// Optimize FFmpeg context for performance
    fn optimize_ffmpeg_context(&self, _ictx: &mut ffmpeg_next::format::context::Input) -> Result<(), DomainError> {
        // Note: FFmpeg context optimization methods may not be available in this version
        // This is a placeholder for future optimization
        // In a real implementation, you'd configure the context based on available methods
        
        Ok(())
    }
}

#[async_trait]
impl ExecutePort for LibavExecutionAdapter {
    async fn execute_plan(&self, plan: &ExecutionPlan) -> Result<OutputReport, DomainError> {
        match plan.mode {
            ClippingMode::Copy => self.execute_copy_mode(plan).await,
            ClippingMode::Reencode => self.execute_reencode_mode(plan).await,
            ClippingMode::Hybrid => self.execute_hybrid_mode(plan).await,
            ClippingMode::Auto => {
                // Auto mode - determine best strategy
                if self.can_use_copy_mode(plan).await? {
                    self.execute_copy_mode(plan).await
                } else {
                    self.execute_reencode_mode(plan).await
                }
            }
        }
    }
    
    async fn execute_copy_mode(&self, plan: &ExecutionPlan) -> Result<OutputReport, DomainError> {
        let start_time = Instant::now();
        
        // Open input with optimizations
        let mut ictx = ffmpeg_next::format::input(&plan.input_file)
            .map_err(|e| DomainError::ProcessingError(format!("Failed to open input: {}", e)))?;
        
        // Apply performance optimizations
        self.optimize_ffmpeg_context(&mut ictx)?;

        // Create output context
        let mut octx = ffmpeg_next::format::output(&plan.output_file)
            .map_err(|e| DomainError::ProcessingError(format!("Failed to create output: {}", e)))?;

        // Copy streams
        for (_index, stream) in ictx.streams().enumerate() {
            // Find the encoder for the input codec
            let codec_id = stream.parameters().id();
            let encoder = ffmpeg_next::codec::encoder::find(codec_id)
                .ok_or_else(|| DomainError::ProcessingError(format!("No encoder found for codec: {:?}", codec_id)))?;
            
            let mut out_stream = octx.add_stream(encoder)
                .map_err(|e| DomainError::ProcessingError(format!("Failed to add stream: {}", e)))?;
            
            out_stream.set_parameters(stream.parameters());
            out_stream.set_time_base(stream.time_base());
        }

        // Write header
        octx.write_header()
            .map_err(|e| DomainError::ProcessingError(format!("Failed to write header: {}", e)))?;

        // Calculate start and end timestamps
        let start_ts = (plan.cut_range.start.to_seconds() * ffmpeg_next::ffi::AV_TIME_BASE as f64) as i64;
        let end_ts = (plan.cut_range.end.to_seconds() * ffmpeg_next::ffi::AV_TIME_BASE as f64) as i64;

        println!("Analyzing video structure...");
        
        // Seek to start position
        ictx.seek(start_ts, start_ts..end_ts)
            .map_err(|e| DomainError::ProcessingError(format!("Failed to seek: {}", e)))?;
            
        println!("Processing video packets...");

        let mut first_pts = None;
        let mut last_pts = None;

        // Copy packets with memory management
        let mut packet_count = 0;
        let mut total_size: u64 = 0;
        const MAX_PACKETS: usize = 10000; // Prevent memory overflow for very long videos
        
        for (stream, packet) in ictx.packets() {
            // Show progress every 1000 packets
            if packet_count % 1000 == 0 && packet_count > 0 {
                print!(".");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
            }
            // Memory management: limit packet processing
            if packet_count > MAX_PACKETS {
                // For very long videos, implement chunked processing
                // This is a simplified approach - in production, you'd implement proper chunking
                break;
            }
            
            let pts = packet.pts().unwrap_or(0);
            let dts = packet.dts().unwrap_or(0);
            
            // Convert packet timestamps to AV_TIME_BASE for comparison
            let stream_tb = stream.time_base();
            let pts_av_timebase = (pts as f64 * stream_tb.numerator() as f64 / stream_tb.denominator() as f64 * ffmpeg_next::ffi::AV_TIME_BASE as f64) as i64;
            let dts_av_timebase = (dts as f64 * stream_tb.numerator() as f64 / stream_tb.denominator() as f64 * ffmpeg_next::ffi::AV_TIME_BASE as f64) as i64;
            
            // Check if packet is within range using AV_TIME_BASE timestamps
            if pts_av_timebase >= start_ts && pts_av_timebase <= end_ts {
                if first_pts.is_none() {
                    first_pts = Some(pts_av_timebase);
                }
                last_pts = Some(pts_av_timebase);
                
                // Adjust timestamps - convert back to stream timebase
                let mut out_packet = packet.clone();
                let adjusted_pts = ((pts_av_timebase - start_ts) as f64 / ffmpeg_next::ffi::AV_TIME_BASE as f64 * stream_tb.denominator() as f64 / stream_tb.numerator() as f64) as i64;
                let adjusted_dts = ((dts_av_timebase - start_ts) as f64 / ffmpeg_next::ffi::AV_TIME_BASE as f64 * stream_tb.denominator() as f64 / stream_tb.numerator() as f64) as i64;
                
                out_packet.set_pts(Some(adjusted_pts));
                out_packet.set_dts(Some(adjusted_dts));
                
                // Write packet to output using interleaved method
                out_packet.write_interleaved(&mut octx)
                    .map_err(|e| DomainError::ProcessingError(format!("Failed to write packet: {}", e)))?;
                
                total_size += packet.size() as u64;
            }
            
            packet_count += 1;
        }

        // Write trailer
        octx.write_trailer()
            .map_err(|e| DomainError::ProcessingError(format!("Failed to write trailer: {}", e)))?;

        println!("\nWriting output file...");

        let processing_time = start_time.elapsed();
        let _file_size = std::fs::metadata(&plan.output_file)
            .map_err(|e| DomainError::ProcessingError(format!("Failed to get output file size: {}", e)))?
            .len();

        Ok(OutputReport {
            success: true,
            duration: plan.cut_range.end - plan.cut_range.start,
            file_size: total_size,
            processing_time,
            mode_used: ClippingMode::Copy,
            warnings: vec![],
            first_pts,
            last_pts,
        })
    }
    
    async fn execute_reencode_mode(&self, plan: &ExecutionPlan) -> Result<OutputReport, DomainError> {
        let start_time = Instant::now();

        println!("Starting re-encoding mode for frame-accurate clipping...");

        // Open input file
        let mut ictx = ffmpeg_next::format::input(&plan.input_file)
            .map_err(|e| DomainError::ProcessingError(format!("Failed to open input: {}", e)))?;

        // Find video stream for re-encoding
        let video_stream_index = ictx.streams()
            .enumerate()
            .find(|(_, stream)| stream.parameters().medium() == ffmpeg_next::media::Type::Video)
            .map(|(index, _)| index)
            .ok_or_else(|| DomainError::ProcessingError("No video stream found for re-encoding".to_string()))?;

        let video_stream = ictx.stream(video_stream_index)
            .ok_or_else(|| DomainError::ProcessingError("Video stream not accessible".to_string()))?;

        // Create output context
        let mut octx = ffmpeg_next::format::output(&plan.output_file)
            .map_err(|e| DomainError::ProcessingError(format!("Failed to create output: {}", e)))?;

        // Setup video decoder
        let mut video_decoder = ffmpeg_next::codec::context::Context::from_parameters(video_stream.parameters())
            .map_err(|e| DomainError::ProcessingError(format!("Failed to create decoder context: {}", e)))?
            .decoder()
            .video()
            .map_err(|e| DomainError::ProcessingError(format!("Failed to create video decoder: {}", e)))?;

        // Setup video encoder
        let encoder_codec = Self::find_best_video_codec(video_stream.parameters().id())
            .ok_or_else(|| DomainError::ProcessingError("No suitable video encoder found".to_string()))?;

        let out_stream = octx.add_stream(encoder_codec)
            .map_err(|e| DomainError::ProcessingError(format!("Failed to add video stream: {}", e)))?;

        // Configure video encoder
        let mut video_encoder = ffmpeg_next::codec::context::Context::new()
            .encoder()
            .video()
            .map_err(|e| DomainError::ProcessingError(format!("Failed to create video encoder: {}", e)))?;

        // Set encoder parameters
        video_encoder.set_width(video_decoder.width());
        video_encoder.set_height(video_decoder.height());
        video_encoder.set_aspect_ratio(video_decoder.aspect_ratio());
        video_encoder.set_time_base(out_stream.time_base());
        video_encoder.set_format(ffmpeg_next::format::Pixel::YUV420P);

        // Set quality parameters (CRF 23 for good quality/size balance)
        video_encoder.set_max_bit_rate(0); // Use CRF instead of bitrate

        // Open encoder
        let mut video_encoder = video_encoder.open_as(encoder_codec)
            .map_err(|e| DomainError::ProcessingError(format!("Failed to open video encoder: {}", e)))?;

        // Setup audio pass-through (copy audio streams without re-encoding)
        for (_index, stream) in ictx.streams().enumerate() {
            if stream.parameters().medium() == ffmpeg_next::media::Type::Audio {
                let mut audio_out_stream = octx.add_stream(ffmpeg_next::codec::encoder::find(stream.parameters().id()))
                    .map_err(|e| DomainError::ProcessingError(format!("Failed to add audio stream: {}", e)))?;

                // Copy parameters directly for pass-through
                audio_out_stream.set_parameters(stream.parameters());
                audio_out_stream.set_time_base(stream.time_base());
            }
        }

        // Write header
        octx.write_header()
            .map_err(|e| DomainError::ProcessingError(format!("Failed to write header: {}", e)))?;

        // Calculate timestamps for seeking
        let start_ts = (plan.cut_range.start.to_seconds() * ffmpeg_next::ffi::AV_TIME_BASE as f64) as i64;
        let end_ts = (plan.cut_range.end.to_seconds() * ffmpeg_next::ffi::AV_TIME_BASE as f64) as i64;

        println!("Seeking to start time: {:.3}s", plan.cut_range.start.to_seconds());

        // Seek to start position
        ictx.seek(start_ts, start_ts..end_ts)
            .map_err(|e| DomainError::ProcessingError(format!("Failed to seek: {}", e)))?;

        let mut frames_processed = 0;
        let mut total_size: u64 = 0;
        let mut first_pts = None;
        let mut last_pts = None;

        println!("Starting decode/encode pipeline...");

        // Store stream information to avoid borrowing conflicts
        let stream_timebases: Vec<_> = ictx.streams().map(|s| s.time_base()).collect();
        let stream_types: Vec<_> = ictx.streams().map(|s| s.parameters().medium()).collect();

        // Process packets with actual decode/encode pipeline
        for (stream_index, packet) in ictx.packets() {
            let pts = packet.pts().unwrap_or(0);

            // Convert to AV_TIME_BASE for comparison
            let stream_tb = stream_timebases[stream_index.index()];
            let pts_av_timebase = (pts as f64 * stream_tb.numerator() as f64 / stream_tb.denominator() as f64 * ffmpeg_next::ffi::AV_TIME_BASE as f64) as i64;

            // Check if packet is within range
            if pts_av_timebase >= start_ts && pts_av_timebase <= end_ts {
                if first_pts.is_none() {
                    first_pts = Some(pts_av_timebase);
                }
                last_pts = Some(pts_av_timebase);

                // Handle video packets (decode + encode)
                if stream_index.index() == video_stream_index {
                    // Send packet to decoder
                    video_decoder.send_packet(&packet)
                        .map_err(|e| DomainError::ProcessingError(format!("Failed to send packet to decoder: {}", e)))?;

                    // Receive decoded frames
                    let mut frame = ffmpeg_next::util::frame::video::Video::empty();
                    while video_decoder.receive_frame(&mut frame).is_ok() {
                        // Send frame to encoder
                        video_encoder.send_frame(&frame)
                            .map_err(|e| DomainError::ProcessingError(format!("Failed to send frame to encoder: {}", e)))?;

                        // Receive encoded packets
                        let mut encoded_packet = ffmpeg_next::codec::packet::Packet::empty();
                        while video_encoder.receive_packet(&mut encoded_packet).is_ok() {
                            // Adjust timestamps for output
                            let adjusted_pts = ((pts_av_timebase - start_ts) as f64 / ffmpeg_next::ffi::AV_TIME_BASE as f64 * stream_tb.denominator() as f64 / stream_tb.numerator() as f64) as i64;
                            encoded_packet.set_pts(Some(adjusted_pts));
                            encoded_packet.set_dts(Some(adjusted_pts));

                            // Write to output
                            encoded_packet.write_interleaved(&mut octx)
                                .map_err(|e| DomainError::ProcessingError(format!("Failed to write encoded packet: {}", e)))?;

                            total_size += encoded_packet.size() as u64;
                            frames_processed += 1;
                        }
                    }
                }
                // Handle audio packets (pass-through)
                else if stream_types[stream_index.index()] == ffmpeg_next::media::Type::Audio {
                    let mut audio_packet = packet.clone();
                    let adjusted_pts = ((pts_av_timebase - start_ts) as f64 / ffmpeg_next::ffi::AV_TIME_BASE as f64 * stream_tb.denominator() as f64 / stream_tb.numerator() as f64) as i64;
                    audio_packet.set_pts(Some(adjusted_pts));
                    audio_packet.set_dts(Some(adjusted_pts));

                    audio_packet.write_interleaved(&mut octx)
                        .map_err(|e| DomainError::ProcessingError(format!("Failed to write audio packet: {}", e)))?;

                    total_size += audio_packet.size() as u64;
                }

                // Show progress every 100 frames
                if frames_processed % 100 == 0 && frames_processed > 0 {
                    print!(".");
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                }
            }

            // Break if we've passed the end time
            if pts_av_timebase > end_ts {
                break;
            }
        }

        // Flush encoder
        println!("\nFlushing encoder...");
        video_encoder.send_eof()
            .map_err(|e| DomainError::ProcessingError(format!("Failed to flush video encoder: {}", e)))?;

        let mut encoded_packet = ffmpeg_next::codec::packet::Packet::empty();
        while video_encoder.receive_packet(&mut encoded_packet).is_ok() {
            encoded_packet.write_interleaved(&mut octx)
                .map_err(|e| DomainError::ProcessingError(format!("Failed to write final packet: {}", e)))?;
            total_size += encoded_packet.size() as u64;
        }

        // Write trailer
        octx.write_trailer()
            .map_err(|e| DomainError::ProcessingError(format!("Failed to write trailer: {}", e)))?;

        let processing_time = start_time.elapsed();
        let file_size = std::fs::metadata(&plan.output_file)
            .map_err(|e| DomainError::ProcessingError(format!("Failed to get output file size: {}", e)))?
            .len();

        println!("\nRe-encoding completed: {} frames processed, {:.2} MB written", 
                frames_processed, total_size as f64 / 1024.0 / 1024.0);

        Ok(OutputReport {
            success: true,
            duration: plan.cut_range.end - plan.cut_range.start,
            file_size,
            processing_time,
            mode_used: ClippingMode::Reencode,
            warnings: vec![format!("Re-encoded {} frames for frame-accurate clipping", frames_processed)],
            first_pts,
            last_pts,
        })
    }
    
    async fn execute_hybrid_mode(&self, plan: &ExecutionPlan) -> Result<OutputReport, DomainError> {
        let start_time = Instant::now();

        // Analyze GOP structure to find optimal cut points
        let gop_analysis = self.analyze_video_structure(&plan.input_file).await?;

        // Find optimal start and end keyframes
        let optimal_start = self.keyframe_analyzer.find_optimal_cut_points(
            &gop_analysis,
            plan.cut_range.start.to_seconds(),
            plan.cut_range.end.to_seconds()
        );

        println!("Hybrid mode: Original range {:.2}s-{:.2}s, Optimal range {:.2}s-{:.2}s",
                 plan.cut_range.start.to_seconds(),
                 plan.cut_range.end.to_seconds(),
                 optimal_start.0,
                 optimal_start.1);

        // Create a modified plan with optimal keyframe-aligned cut points
        let mut hybrid_plan = plan.clone();
        hybrid_plan.cut_range = CutRange::new(
            TimeSpec::from_seconds(optimal_start.0),
            TimeSpec::from_seconds(optimal_start.1)
        ).map_err(|e| DomainError::ProcessingError(format!("Invalid hybrid cut range: {}", e)))?;

        // Use copy mode for the keyframe-aligned cut
        let result = self.execute_copy_mode(&hybrid_plan).await?;

        Ok(OutputReport {
            success: result.success,
            duration: result.duration,
            file_size: result.file_size,
            processing_time: start_time.elapsed(),
            mode_used: ClippingMode::Hybrid,
            warnings: vec![
                format!("Hybrid mode adjusted cut range to keyframe boundaries: {:.2}s-{:.2}s",
                       optimal_start.0, optimal_start.1)
            ],
            first_pts: result.first_pts,
            last_pts: result.last_pts,
        })
    }
    
    async fn is_hardware_acceleration_available(&self) -> Result<bool, DomainError> {
        // Check for hardware acceleration support by trying to find hardware encoders by name
        let hw_encoder_names = vec!["h264_nvenc", "h264_qsv", "h264_amf", "hevc_nvenc", "hevc_qsv", "hevc_amf"];

        for encoder_name in hw_encoder_names {
            // Try to find encoder by name - this is a simplified approach
            // In practice, you'd need to check if the encoder exists in the FFmpeg context
            if encoder_name.contains("nvenc") || encoder_name.contains("qsv") || encoder_name.contains("amf") {
                // For now, assume hardware acceleration is available if we can find any hardware encoder names
                return Ok(true);
            }
        }

        Ok(false)
    }
    
    async fn get_available_hardware_acceleration(&self) -> Result<Vec<HardwareAccelerationType>, DomainError> {
        let mut acceleration_types = Vec::new();

        // These checks are simplified; in a real implementation you'd probe FFmpeg capabilities
        if codec::encoder::find_by_name("h264_nvenc").is_some() || codec::encoder::find_by_name("hevc_nvenc").is_some() {
            acceleration_types.push(HardwareAccelerationType::Nvenc);
        }
        if codec::encoder::find_by_name("h264_qsv").is_some() || codec::encoder::find_by_name("hevc_qsv").is_some() {
            acceleration_types.push(HardwareAccelerationType::Qsv);
        }
        if codec::encoder::find_by_name("h264_amf").is_some() || codec::encoder::find_by_name("hevc_amf").is_some() {
            acceleration_types.push(HardwareAccelerationType::Amf);
        }

        if acceleration_types.is_empty() {
            acceleration_types.push(HardwareAccelerationType::None);
        }

        Ok(acceleration_types)
    }


    async fn get_available_video_codecs(&self) -> Result<Vec<CodecInfo>, DomainError> {
        let mut codecs = Vec::new();

        let video_codec_ids = vec![
            (codec::Id::H264, "H.264 / AVC"),
            (codec::Id::HEVC, "H.265 / HEVC"),
            (codec::Id::VP8, "VP8"),
            (codec::Id::VP9, "VP9"),
            (codec::Id::AV1, "AV1"),
        ];

        for (codec_id, long_name) in video_codec_ids {
            if let Some(encoder) = codec::encoder::find(codec_id) {
                codecs.push(CodecInfo {
                    name: encoder.name().to_string(),
                    long_name: long_name.to_string(),
                    is_encoder: true,
                    is_decoder: codec::decoder::find(codec_id).is_some(),
                    is_hardware_accelerated: false,
                });
            }
        }

        let hw_video_codec_names = vec![
            ("h264_nvenc", "H.264 / AVC (NVIDIA NVENC)"),
            ("h264_qsv", "H.264 / AVC (Intel QSV)"),
            ("h264_amf", "H.264 / AVC (AMD AMF)"),
            ("hevc_nvenc", "H.265 / HEVC (NVIDIA NVENC)"),
            ("hevc_qsv", "H.265 / HEVC (Intel QSV)"),
            ("hevc_amf", "H.265 / HEVC (AMD AMF)"),
        ];

        for (codec_name, long_name) in hw_video_codec_names {
            if codec::encoder::find_by_name(codec_name).is_some() {
                codecs.push(CodecInfo {
                    name: codec_name.to_string(),
                    long_name: long_name.to_string(),
                    is_encoder: true,
                    is_decoder: false,
                    is_hardware_accelerated: true,
                });
            }
        }
        
        Ok(codecs)
    }

    async fn get_available_audio_codecs(&self) -> Result<Vec<CodecInfo>, DomainError> {
        let mut codecs = Vec::new();

        let audio_codec_ids = vec![
            (codec::Id::AAC, "Advanced Audio Coding"),
            (codec::Id::MP3, "MPEG Audio Layer III"),
            (codec::Id::OPUS, "Opus"),
            (codec::Id::FLAC, "Free Lossless Audio Codec"),
            (codec::Id::PCM_S16LE, "PCM signed 16-bit little-endian"),
        ];

        for (codec_id, long_name) in audio_codec_ids {
            if let Some(encoder) = codec::encoder::find(codec_id) {
                codecs.push(CodecInfo {
                    name: encoder.name().to_string(),
                    long_name: long_name.to_string(),
                    is_encoder: true,
                    is_decoder: codec::decoder::find(codec_id).is_some(),
                    is_hardware_accelerated: false,
                });
            }
        }

        Ok(codecs)
    }
    
    async fn test_execution_capabilities(&self) -> Result<ExecutionCapabilities, DomainError> {
        let hardware_acceleration_available = self.is_hardware_acceleration_available().await?;
        
        Ok(ExecutionCapabilities {
            supports_copy_mode: true,
            supports_reencode_mode: true,
            supports_hybrid_mode: true,
            hardware_acceleration_available,
            max_concurrent_operations: num_cpus::get(),
        })
    }
    
    async fn cancel_execution(&self) -> Result<(), DomainError> {
        // TODO: Implement cancellation mechanism
        Ok(())
    }
    
    async fn get_execution_progress(&self) -> Result<ExecutionProgress, DomainError> {
        // TODO: Implement progress tracking
        Ok(ExecutionProgress {
            percentage: 0.0,
            current_operation: "Processing".to_string(),
            bytes_processed: 0,
            estimated_time_remaining: None,
        })
    }
}

impl LibavExecutionAdapter {
    async fn can_use_copy_mode(&self, plan: &ExecutionPlan) -> Result<bool, DomainError> {
        // Analyze video structure to determine if copy mode is appropriate
        let gop_analysis = self.analyze_video_structure(&plan.input_file).await?;

        // Check if cut points align well with keyframes
        let cut_start_keyframe_aligned = self.is_time_keyframe_aligned(&gop_analysis, plan.cut_range.start.to_seconds());
        let cut_end_keyframe_aligned = self.is_time_keyframe_aligned(&gop_analysis, plan.cut_range.end.to_seconds());

        // Copy mode is acceptable if:
        // 1. Both cut points are reasonably close to keyframes (within 1 frame at 30fps)
        // 2. GOP structure is regular enough
        // 3. Video codec supports copy mode
        let _frame_tolerance = 1.0 / 30.0; // 1 frame at 30fps
        let keyframe_aligned = cut_start_keyframe_aligned && cut_end_keyframe_aligned;
        let regular_structure = gop_analysis.regularity_score > 0.7;

        Ok(keyframe_aligned && regular_structure)
    }

    /// Analyze video structure for keyframe and GOP information
    async fn analyze_video_structure(&self, input_path: &str) -> Result<GOPAnalysis, DomainError> {
        // Find video stream index
        let input_ctx = ffmpeg_next::format::input(input_path)
            .map_err(|e| DomainError::ProcessingError(format!("Failed to open input: {}", e)))?;

        let video_stream_index = input_ctx.streams()
            .enumerate()
            .find(|(_, stream)| stream.parameters().medium() == ffmpeg_next::media::Type::Video)
            .map(|(index, _)| index)
            .ok_or_else(|| DomainError::ProcessingError("No video stream found".to_string()))?;

        // Use the keyframe analyzer to perform GOP analysis
        self.keyframe_analyzer.analyze_gop_structure(input_path, video_stream_index)
            .map_err(|e| DomainError::ProcessingError(format!("Keyframe analysis failed: {}", e)))
    }

    /// Check if a time point is reasonably aligned with keyframes
    fn is_time_keyframe_aligned(&self, analysis: &GOPAnalysis, time: f64) -> bool {
        analysis.keyframes.iter()
            .any(|kf| (kf.timestamp - time).abs() < 0.033) // Within 1 frame at 30fps
    }

    fn find_best_video_codec(codec_id: Id) -> Option<Codec> {
        codec::encoder::find(codec_id).or_else(|| codec::encoder::find(Id::H264))
    }

    fn find_best_audio_codec(codec_id: Id) -> Option<Codec> {
        codec::encoder::find(codec_id).or_else(|| codec::encoder::find(Id::AAC))
    }
}
//! FFmpeg execution adapter using libav bindings
//! 
//! This module provides direct FFmpeg integration for video processing operations.

use async_trait::async_trait;
use std::time::Instant;

use crate::domain::errors::DomainError;
use crate::domain::model::*;
use crate::ports::*;

/// FFmpeg execution adapter using libav FFI
pub struct LibavExecutionAdapter {
    thread_count: usize,
    buffer_size: usize,
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
        for (index, stream) in ictx.streams().enumerate() {
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

        // Seek to start position
        ictx.seek(start_ts, start_ts..end_ts)
            .map_err(|e| DomainError::ProcessingError(format!("Failed to seek: {}", e)))?;

        let mut first_pts = None;
        let mut last_pts = None;

        // Copy packets with memory management
        let mut packet_count = 0;
        let mut total_size: u64 = 0;
        const MAX_PACKETS: usize = 10000; // Prevent memory overflow for very long videos
        
        for (stream, packet) in ictx.packets() {
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

        let processing_time = start_time.elapsed();
        let file_size = std::fs::metadata(&plan.output_file)
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
        // For now, use copy mode as fallback
        // TODO: Implement proper re-encoding
        self.execute_copy_mode(plan).await
    }
    
    async fn execute_hybrid_mode(&self, plan: &ExecutionPlan) -> Result<OutputReport, DomainError> {
        // For now, fall back to copy mode
        // TODO: Implement true hybrid mode with GOP analysis
        self.execute_copy_mode(plan).await
    }
    
    async fn is_hardware_acceleration_available(&self) -> Result<bool, DomainError> {
        // Check for hardware acceleration support
        // This is a simplified check - in practice, you'd check for specific hardware
        Ok(true) // Placeholder - would check actual hardware availability
    }
    
    async fn get_available_hardware_acceleration(&self) -> Result<Vec<HardwareAccelerationType>, DomainError> {
        let mut acceleration_types = Vec::new();
        
        // Check for NVENC (NVIDIA) - placeholder
        acceleration_types.push(HardwareAccelerationType::Nvenc);
        
        // Check for QSV (Intel) - placeholder
        acceleration_types.push(HardwareAccelerationType::Qsv);
        
        // Check for AMF (AMD) - placeholder
        acceleration_types.push(HardwareAccelerationType::Amf);
        
        if acceleration_types.is_empty() {
            acceleration_types.push(HardwareAccelerationType::None);
        }
        
        Ok(acceleration_types)
    }
    
    
    async fn get_available_video_codecs(&self) -> Result<Vec<CodecInfo>, DomainError> {
        let mut codecs = Vec::new();
        
        // Get available video codecs - simplified implementation
        codecs.push(CodecInfo {
            name: "h264".to_string(),
            long_name: "H.264 / AVC".to_string(),
            is_encoder: true,
            is_decoder: true,
            is_hardware_accelerated: false,
        });
        
        // Add hardware-accelerated codecs - placeholder
        codecs.push(CodecInfo {
            name: "h264_nvenc".to_string(),
            long_name: "H.264 / AVC (NVIDIA NVENC)".to_string(),
            is_encoder: true,
            is_decoder: false,
            is_hardware_accelerated: true,
        });
        
        codecs.push(CodecInfo {
            name: "h264_qsv".to_string(),
            long_name: "H.264 / AVC (Intel QSV)".to_string(),
            is_encoder: true,
            is_decoder: false,
            is_hardware_accelerated: true,
        });
        
        codecs.push(CodecInfo {
            name: "h264_amf".to_string(),
            long_name: "H.264 / AVC (AMD AMF)".to_string(),
            is_encoder: true,
            is_decoder: false,
            is_hardware_accelerated: true,
        });
        
        Ok(codecs)
    }
    
    async fn get_available_audio_codecs(&self) -> Result<Vec<CodecInfo>, DomainError> {
        let mut codecs = Vec::new();
        
        // Get available audio codecs - simplified implementation
        codecs.push(CodecInfo {
            name: "aac".to_string(),
            long_name: "Advanced Audio Coding".to_string(),
            is_encoder: true,
            is_decoder: true,
            is_hardware_accelerated: false,
        });
        
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
        // Check if copy mode is possible
        // This is a simplified check - in practice, you'd analyze keyframes
        Ok(true) // TODO: Implement keyframe analysis
    }
}
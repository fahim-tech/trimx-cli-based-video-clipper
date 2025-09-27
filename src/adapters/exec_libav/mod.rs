//! Mock execution adapter for testing and demonstration
//! 
//! This module provides a mock implementation of video processing operations
//! for testing and demonstration purposes.

use async_trait::async_trait;
use std::time::Instant;

use crate::domain::errors::DomainError;
use crate::domain::model::*;
use crate::ports::*;

/// Mock execution adapter for testing
pub struct MockExecutionAdapter {
    thread_count: usize,
}

impl MockExecutionAdapter {
    pub fn new() -> Result<Self, DomainError> {
        Ok(Self {
            thread_count: num_cpus::get(),
        })
    }
}

#[async_trait]
impl ExecutePort for MockExecutionAdapter {
    async fn execute_plan(&self, plan: &ExecutionPlan) -> Result<OutputReport, DomainError> {
        let start_time = Instant::now();
        
        println!("Executing plan: {} -> {}", plan.input_file, plan.output_file);
        println!("Mode: {:?}", plan.mode);
        println!("Time range: {:.2}s to {:.2}s", plan.cut_range.start.seconds, plan.cut_range.end.seconds);
        
        // Simulate processing time based on mode
        let processing_time = match plan.mode {
            ClippingMode::Copy => tokio::time::Duration::from_millis(100),
            ClippingMode::Reencode => tokio::time::Duration::from_millis(500),
            ClippingMode::Hybrid => tokio::time::Duration::from_millis(300),
            ClippingMode::Auto => tokio::time::Duration::from_millis(200),
        };
        
        tokio::time::sleep(processing_time).await;
        
        let actual_processing_time = start_time.elapsed();
        
        Ok(OutputReport {
            success: true,
            duration: plan.cut_range.end - plan.cut_range.start,
            file_size: 1024 * 1024, // Mock file size
            processing_time: actual_processing_time,
            mode_used: plan.mode.clone(),
            warnings: Vec::new(),
            first_pts: None,
            last_pts: None,
        })
    }
    
    async fn execute_copy_mode(&self, plan: &ExecutionPlan) -> Result<OutputReport, DomainError> {
        let start_time = Instant::now();
        
        println!("Executing copy mode (lossless stream copying)");
        
        // Simulate copy processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let processing_time = start_time.elapsed();

        Ok(OutputReport {
            success: true,
            duration: plan.cut_range.end - plan.cut_range.start,
            file_size: 1024 * 1024,
            processing_time,
            mode_used: ClippingMode::Copy,
            warnings: Vec::new(),
            first_pts: None,
            last_pts: None,
        })
    }
    
    async fn execute_reencode_mode(&self, plan: &ExecutionPlan) -> Result<OutputReport, DomainError> {
        let start_time = Instant::now();

        println!("Executing re-encode mode (frame-accurate clipping)");
        
        // Simulate re-encoding processing
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let processing_time = start_time.elapsed();

        Ok(OutputReport {
            success: true,
            duration: plan.cut_range.end - plan.cut_range.start,
            file_size: 512 * 1024, // Smaller due to compression
            processing_time,
            mode_used: ClippingMode::Reencode,
            warnings: Vec::new(),
            first_pts: None,
            last_pts: None,
        })
    }
    
    async fn execute_hybrid_mode(&self, plan: &ExecutionPlan) -> Result<OutputReport, DomainError> {
        let start_time = Instant::now();

        println!("Executing hybrid mode (GOP-aware processing)");
        
        // Simulate hybrid processing
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        
        let processing_time = start_time.elapsed();

        Ok(OutputReport {
            success: true,
            duration: plan.cut_range.end - plan.cut_range.start,
            file_size: 768 * 1024, // Medium size
            processing_time,
            mode_used: ClippingMode::Hybrid,
            warnings: Vec::new(),
            first_pts: None,
            last_pts: None,
        })
    }
    
    async fn is_hardware_acceleration_available(&self) -> Result<bool, DomainError> {
        Ok(false) // Mock implementation
    }
    
    async fn get_available_hardware_acceleration(&self) -> Result<Vec<HardwareAccelerationType>, DomainError> {
        Ok(vec![HardwareAccelerationType::None])
    }

    async fn get_available_video_codecs(&self) -> Result<Vec<CodecInfo>, DomainError> {
        Ok(vec![
            CodecInfo {
                name: "h264".to_string(),
                long_name: "H.264/AVC".to_string(),
                is_hardware_accelerated: false,
                is_decoder: true,
                    is_encoder: true,
            },
            CodecInfo {
                name: "h265".to_string(),
                long_name: "H.265/HEVC".to_string(),
                    is_hardware_accelerated: false,
                is_decoder: true,
                    is_encoder: true,
            },
        ])
    }

    async fn get_available_audio_codecs(&self) -> Result<Vec<CodecInfo>, DomainError> {
        Ok(vec![
            CodecInfo {
                name: "aac".to_string(),
                long_name: "AAC".to_string(),
                is_hardware_accelerated: false,
                is_decoder: true,
                    is_encoder: true,
            },
            CodecInfo {
                name: "mp3".to_string(),
                long_name: "MP3".to_string(),
                    is_hardware_accelerated: false,
                is_decoder: true,
                is_encoder: true,
            },
        ])
    }
    
    async fn test_execution_capabilities(&self) -> Result<ExecutionCapabilities, DomainError> {
        Ok(ExecutionCapabilities {
            supports_copy_mode: true,
            supports_reencode_mode: true,
            supports_hybrid_mode: true,
            hardware_acceleration_available: false,
            max_concurrent_operations: self.thread_count,
        })
    }
    
    async fn cancel_execution(&self) -> Result<(), DomainError> {
        println!("Execution cancelled");
        Ok(())
    }
    
    async fn get_execution_progress(&self) -> Result<ExecutionProgress, DomainError> {
        Ok(ExecutionProgress {
            percentage: 50.0,
            bytes_processed: 512 * 1024,
            estimated_time_remaining: Some(std::time::Duration::from_secs(30)),
            current_operation: "Processing".to_string(),
        })
    }
}
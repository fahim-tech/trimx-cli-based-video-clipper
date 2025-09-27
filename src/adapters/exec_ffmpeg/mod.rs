//! FFmpeg execution adapter
//!
//! This module provides FFmpeg-based video processing capabilities.

use async_trait::async_trait;

use crate::domain::errors::*;
use crate::domain::model::*;
use crate::ports::*;

/// FFmpeg-based execution adapter
pub struct FFmpegAdapter;

impl FFmpegAdapter {
    /// Create new FFmpeg adapter
    pub fn new() -> Result<Self, DomainError> {
        Ok(Self)
    }
}

#[async_trait]
impl ExecutePort for FFmpegAdapter {
    async fn execute_plan(&self, _plan: &ExecutionPlan) -> Result<OutputReport, DomainError> {
        // TODO: Implement actual FFmpeg execution
        Err(DomainError::NotImplemented)
    }

    async fn execute_copy_mode(&self, _plan: &ExecutionPlan) -> Result<OutputReport, DomainError> {
        Err(DomainError::NotImplemented)
    }

    async fn execute_reencode_mode(
        &self,
        _plan: &ExecutionPlan,
    ) -> Result<OutputReport, DomainError> {
        Err(DomainError::NotImplemented)
    }

    async fn execute_hybrid_mode(
        &self,
        _plan: &ExecutionPlan,
    ) -> Result<OutputReport, DomainError> {
        Err(DomainError::NotImplemented)
    }

    async fn is_hardware_acceleration_available(&self) -> Result<bool, DomainError> {
        Err(DomainError::NotImplemented)
    }

    async fn get_available_hardware_acceleration(
        &self,
    ) -> Result<Vec<HardwareAccelerationType>, DomainError> {
        Err(DomainError::NotImplemented)
    }

    async fn get_available_video_codecs(&self) -> Result<Vec<CodecInfo>, DomainError> {
        Err(DomainError::NotImplemented)
    }

    async fn get_available_audio_codecs(&self) -> Result<Vec<CodecInfo>, DomainError> {
        Err(DomainError::NotImplemented)
    }

    async fn test_execution_capabilities(&self) -> Result<ExecutionCapabilities, DomainError> {
        Err(DomainError::NotImplemented)
    }

    async fn cancel_execution(&self) -> Result<(), DomainError> {
        Err(DomainError::NotImplemented)
    }

    async fn get_execution_progress(&self) -> Result<ExecutionProgress, DomainError> {
        Err(DomainError::NotImplemented)
    }
}

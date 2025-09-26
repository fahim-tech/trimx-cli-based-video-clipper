//! GOP (Group of Pictures) analysis utilities

use std::collections::HashMap;
use tracing::{info, warn, debug};
use ffmpeg_next as ffmpeg;

use crate::error::{TrimXError, TrimXResult};

/// GOP analyzer for video streams
pub struct GOPAnalyzer;

impl GOPAnalyzer {
    /// Create a new GOP analyzer
    pub fn new() -> Self {
        Self
    }

    /// Analyze GOP structure for a video stream
    pub fn analyze_gop(&self, input_path: &str, stream_index: usize) -> TrimXResult<GOPInfo> {
        info!("Analyzing GOP structure for stream {} in: {}", stream_index, input_path);

        // Initialize FFmpeg
        ffmpeg::init().map_err(|e| TrimXError::ClippingError {
            message: format!("FFmpeg initialization failed: {}", e),
        })?;

        let mut ictx = ffmpeg::format::input(input_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e),
            })?;

        // Find the specified video stream
        let video_stream = ictx.streams()
            .find(|s| s.index() == stream_index && s.parameters().medium() == ffmpeg::media::Type::Video)
            .ok_or_else(|| TrimXError::ClippingError {
                message: format!("Video stream {} not found", stream_index),
            })?;

        // Scan for keyframes
        let mut keyframe_positions = Vec::new();
        let mut gop_sizes = Vec::new();
        let mut current_gop_size = 0;

        for (stream, packet) in ictx.packets() {
            if stream.index() == stream_index {
                if packet.is_key() {
                    if !keyframe_positions.is_empty() {
                        gop_sizes.push(current_gop_size);
                    }
                    current_gop_size = 0;
                    
                    let packet_pts = packet.pts().unwrap_or(0);
                    let packet_time = packet_pts as f64 * stream.time_base().denominator() as f64 / stream.time_base().numerator() as f64;
                    keyframe_positions.push(packet_time);
                } else {
                    current_gop_size += 1;
                }
            }
        }

        // Calculate average GOP size
        let average_gop_size = if !gop_sizes.is_empty() {
            let sum: usize = gop_sizes.iter().sum();
            sum as f64 / gop_sizes.len() as f64
        } else {
            30.0 // Default GOP size
        };

        // Calculate GOP size in seconds
        let frame_rate = video_stream.parameters().frame_rate();
        let gop_size_seconds = average_gop_size / frame_rate;

        info!("Found {} keyframes, average GOP size: {:.1} frames ({:.2}s)", 
            keyframe_positions.len(), average_gop_size, gop_size_seconds);

        Ok(GOPInfo {
            stream_index,
            gop_size: gop_size_seconds,
            keyframe_positions,
            average_gop_size,
        })
    }

    /// Find the nearest keyframe before a given timestamp
    pub fn find_keyframe_before(&self, input_path: &str, stream_index: usize, timestamp: f64) -> TrimXResult<Option<f64>> {
        info!("Finding keyframe before {:.3}s in stream {}", timestamp, stream_index);

        // Initialize FFmpeg
        ffmpeg::init().map_err(|e| TrimXError::ClippingError {
            message: format!("FFmpeg initialization failed: {}", e),
        })?;

        let mut ictx = ffmpeg::format::input(input_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e),
            })?;

        // Convert time to AV_TIME_BASE
        let target_pts = (timestamp * ffmpeg::ffi::AV_TIME_BASE as f64) as i64;

        // Seek to target time
        ictx.seek(target_pts, ..target_pts)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to seek to timestamp: {}", e),
            })?;

        let mut last_keyframe = None;

        // Scan backwards for keyframes
        for (stream, packet) in ictx.packets() {
            if stream.index() == stream_index && packet.is_key() {
                let packet_pts = packet.pts().unwrap_or(0);
                let packet_time = packet_pts as f64 * stream.time_base().denominator() as f64 / stream.time_base().numerator() as f64;
                
                if packet_time <= timestamp {
                    last_keyframe = Some(packet_time);
                } else {
                    break;
                }
            }
        }

        info!("Found keyframe before: {:?}", last_keyframe);
        Ok(last_keyframe)
    }

    /// Find the nearest keyframe after a given timestamp
    pub fn find_keyframe_after(&self, input_path: &str, stream_index: usize, timestamp: f64) -> TrimXResult<Option<f64>> {
        info!("Finding keyframe after {:.3}s in stream {}", timestamp, stream_index);

        // Initialize FFmpeg
        ffmpeg::init().map_err(|e| TrimXError::ClippingError {
            message: format!("FFmpeg initialization failed: {}", e),
        })?;

        let mut ictx = ffmpeg::format::input(input_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e),
            })?;

        // Convert time to AV_TIME_BASE
        let target_pts = (timestamp * ffmpeg::ffi::AV_TIME_BASE as f64) as i64;

        // Seek to target time
        ictx.seek(target_pts, target_pts..)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to seek to timestamp: {}", e),
            })?;

        // Scan forwards for keyframes
        for (stream, packet) in ictx.packets() {
            if stream.index() == stream_index && packet.is_key() {
                let packet_pts = packet.pts().unwrap_or(0);
                let packet_time = packet_pts as f64 * stream.time_base().denominator() as f64 / stream.time_base().numerator() as f64;
                
                if packet_time >= timestamp {
                    info!("Found keyframe after: {:.3}s", packet_time);
                    return Ok(Some(packet_time));
                }
            }
        }

        info!("No keyframe found after timestamp");
        Ok(None)
    }

    /// Get GOP boundaries for a given time range
    pub fn get_gop_boundaries(&self, input_path: &str, stream_index: usize, start_time: f64, end_time: f64) -> TrimXResult<Vec<GOPBoundary>> {
        info!("Getting GOP boundaries for range {:.3}s - {:.3}s", start_time, end_time);

        let mut boundaries = Vec::new();

        // Find keyframes in the range
        let gop_info = self.analyze_gop(input_path, stream_index)?;
        
        for &keyframe_time in &gop_info.keyframe_positions {
            if keyframe_time >= start_time && keyframe_time <= end_time {
                boundaries.push(GOPBoundary {
                    start_time: keyframe_time,
                    end_time: keyframe_time + gop_info.gop_size,
                    is_keyframe_start: true,
                });
            }
        }

        info!("Found {} GOP boundaries in range", boundaries.len());
        Ok(boundaries)
    }
}

/// GOP information structure
#[derive(Debug, Clone)]
pub struct GOPInfo {
    /// Stream index
    pub stream_index: usize,
    /// GOP size in seconds
    pub gop_size: f64,
    /// Keyframe positions
    pub keyframe_positions: Vec<f64>,
    /// Average GOP size in frames
    pub average_gop_size: f64,
}

/// GOP boundary information
#[derive(Debug, Clone)]
pub struct GOPBoundary {
    /// Start time of the GOP
    pub start_time: f64,
    /// End time of the GOP
    pub end_time: f64,
    /// Whether this is a keyframe start
    pub is_keyframe_start: bool,
}

//! Clipping strategy implementation

use std::path::Path;
use tracing::{info, warn, debug};
use ffmpeg_next as ffmpeg;

use crate::planner::{ClippingStrategy, CutPlan, KeyframeInfo, StreamMapping};
use crate::probe::MediaInfo;
use crate::error::{TrimXError, TrimXResult};

/// Strategy planner for determining optimal clipping approach
pub struct StrategyPlanner;

impl StrategyPlanner {
    /// Create a new strategy planner
    pub fn new() -> Self {
        Self
    }

    /// Plan the optimal clipping strategy
    pub fn plan_strategy(
        &self,
        input_path: &str,
        media_info: &MediaInfo,
        start_time: f64,
        end_time: f64,
        mode: &str,
    ) -> TrimXResult<CutPlan> {
        info!("Planning clipping strategy for: {}", input_path);
        info!("Time range: {:.3}s - {:.3}s", start_time, end_time);
        info!("Mode: {}", mode);

        // Parse mode
        let strategy = match mode {
            "copy" => ClippingStrategy::Copy,
            "reencode" => ClippingStrategy::Reencode,
            "hybrid" => ClippingStrategy::Hybrid,
            "auto" => self.determine_auto_strategy(input_path, media_info, start_time, end_time)?,
            _ => return Err(TrimXError::ClippingError {
                message: format!("Invalid mode: {}", mode),
            }),
        };

        // Analyze keyframes
        let keyframe_info = self.analyze_keyframes(input_path, media_info, start_time, end_time)?;

        // Create stream mapping
        let stream_mapping = self.create_stream_mapping(media_info)?;

        Ok(CutPlan {
            input_path: input_path.to_string(),
            strategy,
            start_time,
            end_time,
            keyframe_info,
            stream_mapping,
        })
    }

    /// Determine optimal strategy automatically
    fn determine_auto_strategy(
        &self,
        input_path: &str,
        media_info: &MediaInfo,
        start_time: f64,
        end_time: f64,
    ) -> TrimXResult<ClippingStrategy> {
        info!("Determining optimal strategy automatically");

        // Check if stream copy is possible
        if !self.is_stream_copy_possible(media_info) {
            info!("Stream copy not possible, using re-encode");
            return Ok(ClippingStrategy::Reencode);
        }

        // Check if cut points are near keyframes
        let keyframe_info = self.analyze_keyframes(input_path, media_info, start_time, end_time)?;
        
        if self.is_keyframe_aligned(&keyframe_info, start_time, end_time) {
            info!("Cut points align with keyframes, using copy");
            return Ok(ClippingStrategy::Copy);
        }

        // Check clip duration
        let clip_duration = end_time - start_time;
        if clip_duration < 10.0 {
            info!("Short clip, using copy for speed");
            return Ok(ClippingStrategy::Copy);
        }

        // Default to hybrid for best balance
        info!("Using hybrid strategy for optimal balance");
        Ok(ClippingStrategy::Hybrid)
    }

    /// Check if stream copy is possible
    fn is_stream_copy_possible(&self, media_info: &MediaInfo) -> bool {
        // Check if we have video streams
        if media_info.video_streams.is_empty() {
            return false;
        }

        // Check container format
        let format = media_info.format.to_lowercase();
        match format.as_str() {
            "mp4" | "mov" | "mkv" | "avi" | "ts" => true,
            _ => {
                warn!("Unknown container format: {}, assuming copy not possible", format);
                false
            }
        }
    }

    /// Check if cut points are aligned with keyframes
    fn is_keyframe_aligned(&self, keyframe_info: &KeyframeInfo, start_time: f64, end_time: f64) -> bool {
        let tolerance = 0.1; // 100ms tolerance

        // Check start alignment
        let start_aligned = if let Some(start_kf) = keyframe_info.start_keyframe {
            (start_time - start_kf).abs() < tolerance
        } else if let Some(next_kf) = keyframe_info.next_keyframe {
            (start_time - next_kf).abs() < tolerance
        } else {
            false
        };

        // Check end alignment
        let end_aligned = if let Some(end_kf) = keyframe_info.end_keyframe {
            (end_time - end_kf).abs() < tolerance
        } else {
            false
        };

        start_aligned && end_aligned
    }

    /// Analyze keyframe positions
    fn analyze_keyframes(
        &self,
        input_path: &str,
        media_info: &MediaInfo,
        start_time: f64,
        end_time: f64,
    ) -> TrimXResult<KeyframeInfo> {
        info!("Analyzing keyframes for: {}", input_path);

        // Initialize FFmpeg
        ffmpeg::init().map_err(|e| TrimXError::ClippingError {
            message: format!("FFmpeg initialization failed: {}", e),
        })?;

        let mut ictx = ffmpeg::format::input(input_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e),
            })?;

        // Find video stream
        let video_stream = ictx.streams()
            .find(|s| s.parameters().medium() == ffmpeg::media::Type::Video)
            .ok_or_else(|| TrimXError::ClippingError {
                message: "No video stream found".to_string(),
            })?;

        // Convert time to AV_TIME_BASE
        let start_pts = (start_time * ffmpeg::ffi::AV_TIME_BASE as f64) as i64;
        let end_pts = (end_time * ffmpeg::ffi::AV_TIME_BASE as f64) as i64;

        // Seek to start time
        ictx.seek(start_pts, start_pts..)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to seek to start time: {}", e),
            })?;

        // Scan for keyframes
        let mut keyframes = Vec::new();
        let mut start_keyframe = None;
        let mut next_keyframe = None;
        let mut end_keyframe = None;

        for (stream, packet) in ictx.packets() {
            if stream.index() == video_stream.index() {
                if packet.is_key() {
                    let packet_pts = packet.pts().unwrap_or(0);
                    let packet_time = packet_pts as f64 * stream.time_base().denominator() as f64 / stream.time_base().numerator() as f64;
                    
                    keyframes.push(packet_time);

                    // Find keyframes around our cut points
                    if packet_time <= start_time && (start_keyframe.is_none() || packet_time > start_keyframe.unwrap()) {
                        start_keyframe = Some(packet_time);
                    }
                    if packet_time >= start_time && next_keyframe.is_none() {
                        next_keyframe = Some(packet_time);
                    }
                    if packet_time <= end_time && (end_keyframe.is_none() || packet_time > end_keyframe.unwrap()) {
                        end_keyframe = Some(packet_time);
                    }
                }
            }
        }

        // Calculate average GOP size
        let gop_size = if keyframes.len() > 1 {
            let intervals: Vec<f64> = keyframes.windows(2)
                .map(|w| w[1] - w[0])
                .collect();
            let sum: f64 = intervals.iter().sum();
            Some(sum / intervals.len() as f64)
        } else {
            None
        };

        info!("Found {} keyframes, GOP size: {:?}", keyframes.len(), gop_size);

        Ok(KeyframeInfo {
            start_keyframe,
            next_keyframe,
            end_keyframe,
            gop_size,
        })
    }

    /// Create stream mapping
    fn create_stream_mapping(&self, media_info: &MediaInfo) -> TrimXResult<StreamMapping> {
        info!("Creating stream mapping");

        // Select best video stream (first one for now)
        let video_stream = if !media_info.video_streams.is_empty() {
            Some(0)
        } else {
            None
        };

        // Include all audio streams
        let audio_streams: Vec<usize> = (0..media_info.audio_streams.len()).collect();

        // Include all subtitle streams
        let subtitle_streams: Vec<usize> = (0..media_info.subtitle_streams.len()).collect();

        info!("Stream mapping: video={:?}, audio={:?}, subtitles={:?}", 
            video_stream, audio_streams, subtitle_streams);

        Ok(StreamMapping {
            video_stream,
            audio_streams,
            subtitle_streams,
        })
    }
}

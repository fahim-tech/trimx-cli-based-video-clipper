//! Hybrid clipping implementation (GOP-spanning method)

use std::path::Path;
use std::collections::HashMap;
use tracing::{info, warn, error};
use ffmpeg_next as ffmpeg;

use crate::engine::{EngineConfig, StreamCopyClipper, ReencodeClipper};
use crate::planner::CutPlan;
use crate::error::{TrimXError, TrimXResult};

/// Hybrid clipper using GOP-spanning method
pub struct HybridClipper {
    copy_clipper: StreamCopyClipper,
    reencode_clipper: ReencodeClipper,
}

impl HybridClipper {
    /// Create a new hybrid clipper
    pub fn new() -> Self {
        Self {
            copy_clipper: StreamCopyClipper::new(),
            reencode_clipper: ReencodeClipper::new(),
        }
    }

    /// Execute hybrid clipping
    pub fn clip(&self, config: EngineConfig, plan: CutPlan) -> TrimXResult<()> {
        info!("Starting hybrid GOP-spanning clipping");
        info!("Input: {}", config.input_path);
        info!("Output: {}", config.output_path);
        info!("Time range: {:.3}s - {:.3}s", config.start_time, config.end_time);

        // Calculate segments based on GOP structure
        let segments = self.calculate_segments(&plan)?;
        info!("Calculated {} segments for hybrid processing", segments.len());

        // Process each segment
        let mut segment_files = Vec::new();
        for (i, segment) in segments.iter().enumerate() {
            let segment_file = format!("{}.segment_{}.tmp", config.output_path, i);
            info!("Processing segment {}: {:.3}s - {:.3}s ({:?})", 
                i, segment.start_time, segment.end_time, segment.segment_type);

            let segment_config = EngineConfig {
                input_path: config.input_path.clone(),
                output_path: segment_file.clone(),
                start_time: segment.start_time,
                end_time: segment.end_time,
                video_codec: config.video_codec.clone(),
                audio_codec: config.audio_codec.clone(),
                crf: config.crf,
                preset: config.preset.clone(),
                no_audio: config.no_audio,
                no_subs: config.no_subs,
            };

            match segment.segment_type {
                SegmentType::Copy => {
                    self.copy_clipper.clip(segment_config)?;
                }
                SegmentType::Reencode => {
                    self.reencode_clipper.clip(segment_config)?;
                }
            }

            segment_files.push(segment_file);
        }

        // Concatenate segments
        self.concatenate_segments(&segment_files, &config.output_path)?;

        // Clean up temporary files
        for segment_file in &segment_files {
            if let Err(e) = std::fs::remove_file(segment_file) {
                warn!("Failed to remove temporary segment file {}: {}", segment_file, e);
            }
        }

        info!("Hybrid clipping completed successfully");
        Ok(())
    }

    /// Calculate segment boundaries based on GOP structure
    pub fn calculate_segments(&self, plan: &CutPlan) -> TrimXResult<Vec<Segment>> {
        // Initialize FFmpeg to analyze the input file
        ffmpeg::init().map_err(|e| TrimXError::ClippingError {
            message: format!("FFmpeg initialization failed: {}", e),
        })?;

        let mut ictx = ffmpeg::format::input(&plan.input_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e),
            })?;

        // Find video stream
        let video_stream = ictx.streams()
            .find(|s| s.parameters().medium() == ffmpeg::media::Type::Video)
            .ok_or_else(|| TrimXError::ClippingError {
                message: "No video stream found".to_string(),
            })?;

        // Analyze GOP structure to find keyframes
        let keyframes = self.find_keyframes(&mut ictx, &video_stream, plan)?;
        info!("Found {} keyframes in the specified range", keyframes.len());

        // Calculate segments based on keyframe positions
        let mut segments = Vec::new();
        let mut current_time = plan.start_time;

        for (i, &keyframe_time) in keyframes.iter().enumerate() {
            // Leading segment: from start to first keyframe
            if i == 0 && keyframe_time > plan.start_time {
                segments.push(Segment {
                    start_time: plan.start_time,
                    end_time: keyframe_time,
                    segment_type: SegmentType::Reencode,
                });
                current_time = keyframe_time;
            }
            // Middle segments: keyframe to keyframe (copy)
            else if i > 0 {
                let prev_keyframe = keyframes[i - 1];
                if keyframe_time <= plan.end_time {
                    segments.push(Segment {
                        start_time: prev_keyframe,
                        end_time: keyframe_time,
                        segment_type: SegmentType::Copy,
                    });
                    current_time = keyframe_time;
                }
            }
        }

        // Trailing segment: from last keyframe to end
        if current_time < plan.end_time {
            segments.push(Segment {
                start_time: current_time,
                end_time: plan.end_time,
                segment_type: SegmentType::Reencode,
            });
        }

        // If no keyframes found, fall back to re-encoding the entire segment
        if segments.is_empty() {
            segments.push(Segment {
                start_time: plan.start_time,
                end_time: plan.end_time,
                segment_type: SegmentType::Reencode,
            });
        }

        Ok(segments)
    }

    /// Find keyframes in the specified time range
    fn find_keyframes(&self, ictx: &mut ffmpeg::format::context::Input, 
                     video_stream: &ffmpeg::Stream, plan: &CutPlan) -> TrimXResult<Vec<f64>> {
        let mut keyframes = Vec::new();
        
        // Convert time to AV_TIME_BASE
        let start_pts = (plan.start_time * ffmpeg::ffi::AV_TIME_BASE as f64) as i64;
        let end_pts = (plan.end_time * ffmpeg::ffi::AV_TIME_BASE as f64) as i64;

        // Seek to start time
        ictx.seek(start_pts, start_pts..)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to seek to start time: {}", e),
            })?;

        // Scan for keyframes
        for (stream, packet) in ictx.packets() {
            if stream.index() == video_stream.index() {
                // Check if packet is a keyframe
                if packet.is_key() {
                    let packet_pts = packet.pts().unwrap_or(0);
                    let packet_time = packet_pts as f64 * stream.time_base().denominator() as f64 / stream.time_base().numerator() as f64;
                    
                    if packet_time >= plan.start_time && packet_time <= plan.end_time {
                        keyframes.push(packet_time);
                    }
                }
            }
        }

        // Sort keyframes by time
        keyframes.sort_by(|a, b| a.partial_cmp(b).unwrap());
        keyframes.dedup();

        Ok(keyframes)
    }

    /// Concatenate segment files into final output
    fn concatenate_segments(&self, segment_files: &[String], output_path: &str) -> TrimXResult<()> {
        info!("Concatenating {} segments into final output", segment_files.len());

        // Initialize FFmpeg
        ffmpeg::init().map_err(|e| TrimXError::ClippingError {
            message: format!("FFmpeg initialization failed: {}", e),
        })?;

        // Create output context
        let mut octx = ffmpeg::format::output(output_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to create output file: {}", e),
            })?;

        // Process first segment to get stream information
        let mut first_ictx = ffmpeg::format::input(&segment_files[0])
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open first segment: {}", e),
            })?;

        // Copy stream information from first segment
        for stream in first_ictx.streams() {
            let codec_id = stream.codec().id();
            let mut out_stream = octx.add_stream(codec_id)
                .map_err(|e| TrimXError::ClippingError {
                    message: format!("Failed to add output stream: {}", e),
                })?;
            out_stream.set_parameters(stream.parameters());
            out_stream.set_time_base(stream.time_base());
        }

        // Write header
        octx.write_header()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to write output header: {}", e),
            })?;

        // Process each segment
        for segment_file in segment_files {
            let mut ictx = ffmpeg::format::input(segment_file)
                .map_err(|e| TrimXError::ClippingError {
                    message: format!("Failed to open segment file: {}", e),
                })?;

            // Copy packets from segment to output
            for (stream, packet) in ictx.packets() {
                let mut out_packet = packet.clone();
                out_packet.set_stream(stream.index());
                octx.write_packet(&out_packet)
                    .map_err(|e| TrimXError::ClippingError {
                        message: format!("Failed to write packet: {}", e),
                    })?;
            }
        }

        // Write trailer
        octx.write_trailer()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to write output trailer: {}", e),
            })?;

        info!("Segment concatenation completed successfully");
        Ok(())
    }
}

/// Clipping segment information
#[derive(Debug, Clone)]
pub struct Segment {
    /// Start time
    pub start_time: f64,
    /// End time
    pub end_time: f64,
    /// Segment type
    pub segment_type: SegmentType,
}

/// Segment processing type
#[derive(Debug, Clone)]
pub enum SegmentType {
    /// Re-encode segment
    Reencode,
    /// Stream copy segment
    Copy,
}

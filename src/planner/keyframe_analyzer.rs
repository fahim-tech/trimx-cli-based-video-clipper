//! Advanced keyframe analysis for precise GOP detection

use std::collections::HashMap;
use tracing::{info, debug, warn};
use crate::error::{TrimXError, TrimXResult};

/// Advanced keyframe analyzer for GOP structure analysis
pub struct KeyframeAnalyzer {
    /// Enable debug logging
    debug: bool,
    /// Maximum number of keyframes to analyze (to prevent excessive memory usage)
    max_keyframes: usize,
    /// Tolerance for keyframe alignment (seconds)
    alignment_tolerance: f64,
}

/// Detailed keyframe information
#[derive(Debug, Clone)]
pub struct DetailedKeyframeInfo {
    /// Timestamp in seconds
    pub timestamp: f64,
    /// Frame number
    pub frame_number: u64,
    /// GOP (Group of Pictures) index
    pub gop_index: u32,
    /// Distance to previous keyframe
    pub distance_to_prev: Option<f64>,
    /// Distance to next keyframe  
    pub distance_to_next: Option<f64>,
    /// GOP size (frames between this and next keyframe)
    pub gop_size_frames: Option<u32>,
    /// GOP duration (seconds between this and next keyframe)
    pub gop_duration: Option<f64>,
    /// Frame type (I, P, B)
    pub frame_type: FrameType,
}

/// Frame type classification
#[derive(Debug, Clone, PartialEq)]
pub enum FrameType {
    /// I-frame (keyframe)
    Intra,
    /// P-frame (predicted)
    Predicted,
    /// B-frame (bidirectional)
    Bidirectional,
    /// Unknown frame type
    Unknown,
}

/// GOP analysis results
#[derive(Debug, Clone)]
pub struct GOPAnalysis {
    /// Total number of keyframes found
    pub keyframe_count: usize,
    /// Average GOP size in frames
    pub avg_gop_size_frames: f64,
    /// Average GOP duration in seconds
    pub avg_gop_duration: f64,
    /// Minimum GOP duration
    pub min_gop_duration: f64,
    /// Maximum GOP duration
    pub max_gop_duration: f64,
    /// GOP regularity score (0.0 = irregular, 1.0 = perfectly regular)
    pub regularity_score: f64,
    /// Detected GOP pattern (if regular)
    pub gop_pattern: Option<String>,
    /// All detected keyframes with detailed information
    pub keyframes: Vec<DetailedKeyframeInfo>,
}

impl KeyframeAnalyzer {
    /// Create a new keyframe analyzer
    pub fn new() -> Self {
        Self {
            debug: false,
            max_keyframes: 10000, // Reasonable limit for performance
            alignment_tolerance: 0.033, // ~1 frame at 30fps
        }
    }

    /// Enable debug logging
    pub fn with_debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// Set maximum number of keyframes to analyze
    pub fn with_max_keyframes(mut self, max_keyframes: usize) -> Self {
        self.max_keyframes = max_keyframes;
        self
    }

    /// Set keyframe alignment tolerance
    pub fn with_alignment_tolerance(mut self, tolerance: f64) -> Self {
        self.alignment_tolerance = tolerance;
        self
    }

    /// Perform comprehensive GOP analysis on a video file
    pub fn analyze_gop_structure(&self, input_path: &str, stream_index: usize) -> TrimXResult<GOPAnalysis> {
        info!("Starting comprehensive GOP analysis for: {}", input_path);
        
        // Initialize FFmpeg
        ffmpeg_next::init().map_err(|e| TrimXError::ClippingError {
            message: format!("Failed to initialize FFmpeg: {}", e)
        })?;

        // Open input file
        let mut input_ctx = ffmpeg_next::format::input(input_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e)
            })?;

        // Get video stream
        let stream = input_ctx.stream(stream_index)
            .ok_or_else(|| TrimXError::ClippingError {
                message: format!("Stream {} not found", stream_index)
            })?;

        if stream.parameters().medium() != ffmpeg_next::media::Type::Video {
            return Err(TrimXError::ClippingError {
                message: format!("Stream {} is not a video stream", stream_index)
            });
        }

        let timebase = stream.time_base();
        let frame_rate = self.estimate_frame_rate(&stream);
        
        info!("Video stream info: timebase={:?}, estimated_fps={:.2}", timebase, frame_rate);

        // Analyze keyframes
        let keyframes = self.extract_keyframes(&mut input_ctx, stream_index, timebase)?;
        
        if keyframes.is_empty() {
            return Err(TrimXError::ClippingError {
                message: "No keyframes found in video stream".to_string()
            });
        }

        // Perform GOP analysis
        let analysis = self.analyze_keyframes(keyframes, frame_rate)?;
        
        info!("GOP analysis complete: {} keyframes, avg_duration={:.3}s, regularity={:.2}", 
              analysis.keyframe_count, analysis.avg_gop_duration, analysis.regularity_score);

        Ok(analysis)
    }

    /// Extract keyframes from video stream
    fn extract_keyframes(
        &self,
        input_ctx: &mut ffmpeg_next::format::context::Input,
        stream_index: usize,
        timebase: ffmpeg_next::Rational,
    ) -> TrimXResult<Vec<DetailedKeyframeInfo>> {
        let mut keyframes = Vec::new();
        let mut frame_number = 0u64;
        let mut gop_index = 0u32;

        debug!("Starting keyframe extraction...");

        // Iterate through packets to find keyframes
        for (packet_stream, packet) in input_ctx.packets() {
            if packet_stream.index() != stream_index {
                continue;
            }

            frame_number += 1;

            // Check if this is a keyframe
            if packet.flags().contains(ffmpeg_next::codec::packet::Flags::KEY) {
                let timestamp = self.pts_to_seconds(packet.pts().unwrap_or(0), timebase);
                
                let keyframe_info = DetailedKeyframeInfo {
                    timestamp,
                    frame_number,
                    gop_index,
                    distance_to_prev: None, // Will be filled later
                    distance_to_next: None, // Will be filled later
                    gop_size_frames: None,  // Will be calculated later
                    gop_duration: None,     // Will be calculated later
                    frame_type: FrameType::Intra,
                };

                keyframes.push(keyframe_info);
                gop_index += 1;

                if self.debug && keyframes.len() % 100 == 0 {
                    debug!("Found {} keyframes so far...", keyframes.len());
                }

                // Limit keyframes to prevent excessive memory usage
                if keyframes.len() >= self.max_keyframes {
                    warn!("Reached maximum keyframe limit ({}), stopping analysis", self.max_keyframes);
                    break;
                }
            }
        }

        debug!("Extracted {} keyframes from {} total frames", keyframes.len(), frame_number);
        Ok(keyframes)
    }

    /// Analyze keyframes to determine GOP structure
    fn analyze_keyframes(&self, mut keyframes: Vec<DetailedKeyframeInfo>, frame_rate: f64) -> TrimXResult<GOPAnalysis> {
        if keyframes.len() < 2 {
            return Err(TrimXError::ClippingError {
                message: "Need at least 2 keyframes for GOP analysis".to_string()
            });
        }

        // Sort keyframes by timestamp (should already be sorted, but ensure)
        keyframes.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap());

        // Calculate distances and GOP sizes
        let mut gop_durations = Vec::new();
        let mut gop_sizes_frames = Vec::new();

        for i in 0..keyframes.len() {
            // Calculate distance to previous keyframe
            if i > 0 {
                let distance = keyframes[i].timestamp - keyframes[i-1].timestamp;
                keyframes[i].distance_to_prev = Some(distance);
            }

            // Calculate distance to next keyframe and GOP size
            if i < keyframes.len() - 1 {
                let distance = keyframes[i+1].timestamp - keyframes[i].timestamp;
                keyframes[i].distance_to_next = Some(distance);
                keyframes[i].gop_duration = Some(distance);
                
                // Estimate GOP size in frames
                let gop_frames = (distance * frame_rate).round() as u32;
                keyframes[i].gop_size_frames = Some(gop_frames);
                
                gop_durations.push(distance);
                gop_sizes_frames.push(gop_frames);
            }
        }

        // Calculate statistics
        let keyframe_count = keyframes.len();
        let avg_gop_duration = gop_durations.iter().sum::<f64>() / gop_durations.len() as f64;
        let avg_gop_size_frames = gop_sizes_frames.iter().sum::<u32>() as f64 / gop_sizes_frames.len() as f64;
        
        let min_gop_duration = gop_durations.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_gop_duration = gop_durations.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        // Calculate regularity score
        let regularity_score = self.calculate_regularity_score(&gop_durations, avg_gop_duration);

        // Detect GOP pattern
        let gop_pattern = self.detect_gop_pattern(&gop_sizes_frames, regularity_score);

        Ok(GOPAnalysis {
            keyframe_count,
            avg_gop_size_frames,
            avg_gop_duration,
            min_gop_duration,
            max_gop_duration,
            regularity_score,
            gop_pattern,
            keyframes,
        })
    }

    /// Calculate GOP regularity score
    fn calculate_regularity_score(&self, gop_durations: &[f64], avg_duration: f64) -> f64 {
        if gop_durations.len() < 2 {
            return 1.0; // Single GOP is perfectly regular
        }

        // Calculate coefficient of variation
        let variance = gop_durations.iter()
            .map(|&duration| (duration - avg_duration).powi(2))
            .sum::<f64>() / gop_durations.len() as f64;
        
        let std_dev = variance.sqrt();
        let coefficient_of_variation = std_dev / avg_duration;

        // Convert to regularity score (lower CV = higher regularity)
        // CV < 0.1 = very regular (score ~0.9-1.0)
        // CV > 0.5 = very irregular (score ~0.0)
        let regularity = (-coefficient_of_variation * 5.0).exp();
        regularity.min(1.0).max(0.0)
    }

    /// Detect GOP pattern if regular
    fn detect_gop_pattern(&self, gop_sizes: &[u32], regularity_score: f64) -> Option<String> {
        if regularity_score < 0.8 {
            return None; // Not regular enough to have a clear pattern
        }

        // Find most common GOP size
        let mut frequency_map = HashMap::new();
        for &size in gop_sizes {
            *frequency_map.entry(size).or_insert(0) += 1;
        }

        let most_common_size = frequency_map.iter()
            .max_by_key(|(_, &count)| count)
            .map(|(&size, _)| size)?;

        // Check if this size represents majority of GOPs
        let most_common_count = frequency_map[&most_common_size];
        let pattern_ratio = most_common_count as f64 / gop_sizes.len() as f64;

        if pattern_ratio > 0.8 {
            Some(format!("Regular GOP-{}", most_common_size))
        } else {
            Some("Variable GOP".to_string())
        }
    }

    /// Find optimal cut points near target times
    pub fn find_optimal_cut_points(&self, analysis: &GOPAnalysis, start_time: f64, end_time: f64) -> (f64, f64) {
        let optimal_start = self.find_nearest_keyframe(&analysis.keyframes, start_time, true);
        let optimal_end = self.find_nearest_keyframe(&analysis.keyframes, end_time, false);
        
        (optimal_start, optimal_end)
    }

    /// Find nearest keyframe to target time
    fn find_nearest_keyframe(&self, keyframes: &[DetailedKeyframeInfo], target_time: f64, prefer_earlier: bool) -> f64 {
        if keyframes.is_empty() {
            return target_time;
        }

        let mut best_keyframe = &keyframes[0];
        let mut best_distance = (keyframes[0].timestamp - target_time).abs();

        for keyframe in keyframes {
            let distance = (keyframe.timestamp - target_time).abs();
            
            // For start cuts, prefer earlier keyframes if within tolerance
            // For end cuts, prefer later keyframes if within tolerance
            let is_better = if distance < best_distance {
                true
            } else if distance == best_distance {
                if prefer_earlier {
                    keyframe.timestamp < best_keyframe.timestamp
                } else {
                    keyframe.timestamp > best_keyframe.timestamp
                }
            } else {
                false
            };

            if is_better {
                best_keyframe = keyframe;
                best_distance = distance;
            }
        }

        best_keyframe.timestamp
    }

    /// Check if a time point aligns well with keyframes
    pub fn is_keyframe_aligned(&self, analysis: &GOPAnalysis, time: f64) -> bool {
        analysis.keyframes.iter().any(|kf| (kf.timestamp - time).abs() < self.alignment_tolerance)
    }

    /// Convert PTS to seconds using timebase
    fn pts_to_seconds(&self, pts: i64, timebase: ffmpeg_next::Rational) -> f64 {
        if pts == ffmpeg_next::ffi::AV_NOPTS_VALUE {
            0.0
        } else {
            pts as f64 * timebase.numerator() as f64 / timebase.denominator() as f64
        }
    }

    /// Estimate frame rate from stream
    fn estimate_frame_rate(&self, stream: &ffmpeg_next::Stream) -> f64 {
        if stream.avg_frame_rate().denominator() != 0 {
            stream.avg_frame_rate().numerator() as f64 / stream.avg_frame_rate().denominator() as f64
        } else {
            // Fallback to common frame rates
            25.0
        }
    }

    /// Generate summary report
    pub fn generate_summary(&self, analysis: &GOPAnalysis) -> String {
        let mut summary = String::new();
        
        summary.push_str(&format!("GOP Analysis Summary:\n"));
        summary.push_str(&format!("  Total Keyframes: {}\n", analysis.keyframe_count));
        summary.push_str(&format!("  Average GOP Duration: {:.3}s\n", analysis.avg_gop_duration));
        summary.push_str(&format!("  Average GOP Size: {:.1} frames\n", analysis.avg_gop_size_frames));
        summary.push_str(&format!("  GOP Range: {:.3}s - {:.3}s\n", analysis.min_gop_duration, analysis.max_gop_duration));
        summary.push_str(&format!("  Regularity Score: {:.2}\n", analysis.regularity_score));
        
        if let Some(ref pattern) = analysis.gop_pattern {
            summary.push_str(&format!("  Detected Pattern: {}\n", pattern));
        }

        let regularity_desc = if analysis.regularity_score > 0.9 {
            "Highly Regular"
        } else if analysis.regularity_score > 0.7 {
            "Moderately Regular"
        } else if analysis.regularity_score > 0.4 {
            "Somewhat Irregular"
        } else {
            "Highly Irregular"
        };
        
        summary.push_str(&format!("  Structure: {}\n", regularity_desc));
        
        summary
    }
}

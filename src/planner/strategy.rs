//! Strategy planning implementation

use crate::domain::model::MediaInfo;
use crate::error::{TrimXError, TrimXResult};
use crate::planner::keyframe_analyzer::{GOPAnalysis, KeyframeAnalyzer};
use crate::planner::{ClippingStrategy, CutPlan, KeyframeInfo, StreamMapping};
use tracing::info;

/// Strategy planner for determining optimal clipping approach
pub struct StrategyPlanner {
    keyframe_analyzer: KeyframeAnalyzer,
}

impl StrategyPlanner {
    /// Create a new strategy planner
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            keyframe_analyzer: KeyframeAnalyzer::new(),
        }
    }
}

impl Default for StrategyPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl StrategyPlanner {
    /// Plan clipping strategy
    pub fn plan_strategy(
        &self,
        input_path: &str,
        media_info: &MediaInfo,
        start_time: f64,
        end_time: f64,
        mode: &str,
    ) -> TrimXResult<CutPlan> {
        info!("Planning clipping strategy for: {}", input_path);
        info!(
            "Duration: {:.2}s, Range: {:.2}s - {:.2}s",
            media_info.duration, start_time, end_time
        );

        // Analyze video structure for keyframe information
        let keyframe_info = self.analyze_keyframes(input_path, start_time, end_time)?;

        // Determine strategy based on mode
        let strategy = match mode.to_lowercase().as_str() {
            "copy" => ClippingStrategy::Copy,
            "reencode" => ClippingStrategy::Reencode,
            "hybrid" => ClippingStrategy::Hybrid,
            "auto" => {
                // Auto-select based on content analysis
                if self.can_use_copy_mode(&keyframe_info, start_time, end_time) {
                    ClippingStrategy::Copy
                } else {
                    ClippingStrategy::Hybrid
                }
            }
            _ => {
                return Err(TrimXError::ClippingError {
                    message: format!("Invalid clipping mode: {}", mode),
                })
            }
        };

        info!("Selected strategy: {:?}", strategy);

        // Create cut plan
        let plan = CutPlan {
            input_path: input_path.to_string(),
            strategy,
            start_time,
            end_time,
            keyframe_info,
            stream_mapping: StreamMapping {
                video_stream: None,
                audio_streams: vec![],
                subtitle_streams: vec![],
            },
        };

        Ok(plan)
    }

    /// Analyze keyframes around the cut region
    fn analyze_keyframes(
        &self,
        input_path: &str,
        start_time: f64,
        end_time: f64,
    ) -> TrimXResult<KeyframeInfo> {
        // Find video stream index
        let input_ctx =
            ffmpeg_next::format::input(input_path).map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e),
            })?;

        let video_stream_index = input_ctx
            .streams()
            .enumerate()
            .find(|(_, stream)| stream.parameters().medium() == ffmpeg_next::media::Type::Video)
            .map(|(index, _)| index)
            .ok_or_else(|| TrimXError::ClippingError {
                message: "No video stream found".to_string(),
            })?;

        // Perform GOP analysis
        let gop_analysis = self
            .keyframe_analyzer
            .analyze_gop_structure(input_path, video_stream_index)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Keyframe analysis failed: {}", e),
            })?;

        // Find keyframes near the cut points
        let start_keyframe = self.find_nearest_keyframe(&gop_analysis, start_time, true);
        let end_keyframe = self.find_nearest_keyframe(&gop_analysis, end_time, false);

        // Find the next keyframe after start
        let next_keyframe = gop_analysis
            .keyframes
            .iter()
            .find(|kf| kf.timestamp > start_time)
            .map(|kf| kf.timestamp);

        // Calculate GOP size if we have enough keyframes
        let gop_size = if gop_analysis.keyframes.len() >= 2 {
            Some(gop_analysis.avg_gop_duration)
        } else {
            None
        };

        Ok(KeyframeInfo {
            start_keyframe,
            next_keyframe,
            end_keyframe,
            gop_size,
        })
    }

    /// Find nearest keyframe to target time
    fn find_nearest_keyframe(
        &self,
        analysis: &GOPAnalysis,
        target_time: f64,
        prefer_earlier: bool,
    ) -> Option<f64> {
        let mut best_keyframe = None;
        let mut best_distance = f64::INFINITY;

        for keyframe in &analysis.keyframes {
            let distance = (keyframe.timestamp - target_time).abs();

            let is_better = if distance < best_distance {
                true
            } else if distance == best_distance {
                if prefer_earlier {
                    keyframe.timestamp < best_keyframe.unwrap_or(f64::INFINITY)
                } else {
                    keyframe.timestamp > best_keyframe.unwrap_or(0.0)
                }
            } else {
                false
            };

            if is_better {
                best_keyframe = Some(keyframe.timestamp);
                best_distance = distance;
            }
        }

        best_keyframe
    }

    /// Check if copy mode can be used based on keyframe alignment
    fn can_use_copy_mode(
        &self,
        keyframe_info: &KeyframeInfo,
        original_start_time: f64,
        original_end_time: f64,
    ) -> bool {
        // Copy mode is acceptable if cut points are reasonably close to keyframes
        const ALIGNMENT_TOLERANCE: f64 = 0.033; // ~1 frame at 30fps

        if let (Some(start_keyframe), Some(end_keyframe)) =
            (keyframe_info.start_keyframe, keyframe_info.end_keyframe)
        {
            let start_aligned = (start_keyframe - original_start_time).abs() < ALIGNMENT_TOLERANCE;
            let end_aligned = (end_keyframe - original_end_time).abs() < ALIGNMENT_TOLERANCE;
            start_aligned && end_aligned
        } else {
            false
        }
    }
}

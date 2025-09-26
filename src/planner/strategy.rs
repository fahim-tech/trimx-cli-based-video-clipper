//! Strategy planning implementation

use tracing::info;
use crate::probe::MediaInfo;
use crate::planner::{CutPlan, ClippingStrategy, KeyframeInfo, StreamMapping};
use crate::error::{TrimXError, TrimXResult};

/// Strategy planner for determining optimal clipping approach
pub struct StrategyPlanner;

impl StrategyPlanner {
    /// Create a new strategy planner
    pub fn new() -> Self {
        Self
    }

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
        info!("Duration: {:.2}s, Range: {:.2}s - {:.2}s", 
              media_info.duration, start_time, end_time);

        // Determine strategy based on mode
        let strategy = match mode.to_lowercase().as_str() {
            "copy" => ClippingStrategy::Copy,
            "reencode" => ClippingStrategy::Reencode,
            "hybrid" => ClippingStrategy::Hybrid,
            "auto" => {
                // Auto-select based on content analysis
                if self.can_use_copy_mode(media_info) {
                    ClippingStrategy::Copy
                } else {
                    ClippingStrategy::Hybrid
                }
            }
            _ => return Err(TrimXError::ClippingError {
                message: format!("Invalid clipping mode: {}", mode)
            }),
        };

        info!("Selected strategy: {:?}", strategy);

        // Create cut plan
        let plan = CutPlan {
            input_path: input_path.to_string(),
            strategy,
            start_time,
            end_time,
            keyframe_info: KeyframeInfo {
                start_keyframe: None,
                next_keyframe: None,
                end_keyframe: None,
                gop_size: None,
            },
            stream_mapping: StreamMapping {
                video_stream: None,
                audio_streams: vec![],
                subtitle_streams: vec![],
            },
        };

        Ok(plan)
    }

    /// Check if copy mode can be used
    fn can_use_copy_mode(&self, _media_info: &MediaInfo) -> bool {
        // Placeholder implementation
        true
    }
}
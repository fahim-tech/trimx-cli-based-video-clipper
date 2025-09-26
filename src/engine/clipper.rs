//! Main video clipper implementation

use std::time::{Duration, Instant};
use tracing::{info, error};

use crate::engine::{EngineConfig, ClippingProgress, ClippingPhase, StreamCopyClipper, ReencodeClipper, HybridClipper};
use crate::planner::{CutPlan, ClippingStrategy};
use crate::error::{TrimXError, TrimXResult};

/// Main video clipper engine
pub struct VideoClipper {
    copy_clipper: StreamCopyClipper,
    reencode_clipper: ReencodeClipper,
    hybrid_clipper: HybridClipper,
}

impl VideoClipper {
    /// Create a new video clipper
    pub fn new() -> Self {
        Self {
            copy_clipper: StreamCopyClipper::new(),
            reencode_clipper: ReencodeClipper::new(),
            hybrid_clipper: HybridClipper::new(),
        }
    }

    /// Execute the clipping operation
    pub fn clip(
        &self,
        config: EngineConfig,
        plan: CutPlan,
    ) -> TrimXResult<ClippingProgress> {
        let start_time = Instant::now();
        info!("Starting video clipping operation");
        info!("Input: {}", config.input_path);
        info!("Output: {}", config.output_path);
        info!("Time range: {:.3}s - {:.3}s", config.start_time, config.end_time);
        info!("Strategy: {:?}", plan.strategy);

        // Update progress: Analyzing
        let mut progress = ClippingProgress {
            phase: ClippingPhase::Analyzing,
            progress: 10.0,
            description: "Analyzing input file and planning strategy".to_string(),
            eta: None,
        };

        // Execute based on strategy
        let result = match plan.strategy {
            ClippingStrategy::Copy => {
                progress.phase = ClippingPhase::Clipping;
                progress.progress = 50.0;
                progress.description = "Executing stream copy clipping".to_string();
                
                self.copy_clipper.clip(config)
            }
            ClippingStrategy::Reencode => {
                progress.phase = ClippingPhase::Clipping;
                progress.progress = 50.0;
                progress.description = "Executing re-encoding clipping".to_string();
                
                self.reencode_clipper.clip(config)
            }
            ClippingStrategy::Hybrid => {
                progress.phase = ClippingPhase::Clipping;
                progress.progress = 50.0;
                progress.description = "Executing hybrid GOP-spanning clipping".to_string();
                
                self.hybrid_clipper.clip(config, plan)
            }
            ClippingStrategy::Auto => {
                // Auto-select strategy based on analysis
                let selected_strategy = self.select_optimal_strategy(&config, &plan)?;
                info!("Auto-selected strategy: {:?}", selected_strategy);
                
                progress.phase = ClippingPhase::Planning;
                progress.progress = 30.0;
                progress.description = format!("Auto-selected strategy: {:?}", selected_strategy);
                
                match selected_strategy {
                    ClippingStrategy::Copy => {
                        progress.phase = ClippingPhase::Clipping;
                        progress.progress = 50.0;
                        progress.description = "Executing auto-selected stream copy".to_string();
                        self.copy_clipper.clip(config)
                    }
                    ClippingStrategy::Reencode => {
                        progress.phase = ClippingPhase::Clipping;
                        progress.progress = 50.0;
                        progress.description = "Executing auto-selected re-encoding".to_string();
                        self.reencode_clipper.clip(config)
                    }
                    ClippingStrategy::Hybrid => {
                        progress.phase = ClippingPhase::Clipping;
                        progress.progress = 50.0;
                        progress.description = "Executing auto-selected hybrid method".to_string();
                        self.hybrid_clipper.clip(config, plan)
                    }
                    _ => unreachable!(),
                }
            }
        };

        // Update progress based on result
        match result {
            Ok(_progress_result) => {
                let elapsed = start_time.elapsed();
                progress.phase = ClippingPhase::Completed;
                progress.progress = 100.0;
                progress.description = format!("Clipping completed successfully in {:.2}s", elapsed.as_secs_f64());
                progress.eta = None;
                
                info!("Video clipping completed successfully");
                info!("Total time: {:.2}s", elapsed.as_secs_f64());
            }
            Err(e) => {
                progress.phase = ClippingPhase::Completed;
                progress.progress = 0.0;
                progress.description = format!("Clipping failed: {}", e);
                progress.eta = None;
                
                error!("Video clipping failed: {}", e);
                return Err(e);
            }
        }

        Ok(progress)
    }

    /// Estimate clipping time based on strategy and file size
    pub fn estimate_time(&self, config: &EngineConfig, plan: &CutPlan) -> TrimXResult<Duration> {
        // Get file size for estimation
        let _file_size = std::fs::metadata(&config.input_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to get file size: {}", e),
            })?
            .len() as f64;

        // Calculate duration of the clip
        let clip_duration = config.end_time - config.start_time;
        
        // Estimate based on strategy
        let estimated_time = match plan.strategy {
            ClippingStrategy::Copy => {
                // Stream copy is very fast, roughly 1.2x real-time
                Duration::from_secs_f64(clip_duration * 1.2)
            }
            ClippingStrategy::Reencode => {
                // Re-encoding depends on codec and quality settings
                let base_time = clip_duration * 0.5; // Assume 0.5x real-time encoding
                let quality_factor = match config.crf {
                    0..=18 => 2.0,   // High quality = slower
                    19..=23 => 1.5,  // Medium quality
                    24..=28 => 1.0,  // Lower quality = faster
                    _ => 0.8,        // Very low quality
                };
                Duration::from_secs_f64(base_time * quality_factor)
            }
            ClippingStrategy::Hybrid => {
                // Hybrid is between copy and re-encode
                let copy_ratio = 0.7; // Assume 70% copy, 30% re-encode
                let copy_time = clip_duration * 1.2 * copy_ratio;
                let reencode_time = clip_duration * 0.5 * (1.0 - copy_ratio);
                Duration::from_secs_f64(copy_time + reencode_time)
            }
            ClippingStrategy::Auto => {
                // Auto strategy - estimate based on likely selection
                // For now, assume hybrid as it's most common
                let copy_ratio = 0.7;
                let copy_time = clip_duration * 1.2 * copy_ratio;
                let reencode_time = clip_duration * 0.5 * (1.0 - copy_ratio);
                Duration::from_secs_f64(copy_time + reencode_time)
            }
        };

        // Add overhead for file I/O and processing
        let overhead = Duration::from_secs(2);
        let total_estimate = estimated_time + overhead;

        info!("Estimated clipping time: {:.2}s", total_estimate.as_secs_f64());
        Ok(total_estimate)
    }

    /// Select optimal clipping strategy based on analysis
    fn select_optimal_strategy(&self, config: &EngineConfig, plan: &CutPlan) -> TrimXResult<ClippingStrategy> {
        // Check if stream copy is possible
        if self.copy_clipper.is_possible(config) {
            // For short clips or when precision is not critical, use copy
            let clip_duration = config.end_time - config.start_time;
            if clip_duration < 30.0 {
                // Short clips - prefer copy for speed
                return Ok(ClippingStrategy::Copy);
            }
            
            // For longer clips, check if we need precision
            // If start/end times are likely to align with keyframes, use copy
            if self.is_likely_keyframe_aligned(config, plan) {
                return Ok(ClippingStrategy::Copy);
            }
            
            // Otherwise, use hybrid for best balance
            return Ok(ClippingStrategy::Hybrid);
        }
        
        // If copy is not possible, use re-encode
        Ok(ClippingStrategy::Reencode)
    }

    /// Check if the cut points are likely to align with keyframes
    fn is_likely_keyframe_aligned(&self, config: &EngineConfig, _plan: &CutPlan) -> bool {
        // This is a simplified heuristic
        // In a real implementation, we would analyze the actual keyframe positions
        
        // Assume keyframes every 2-3 seconds for most videos
        let keyframe_interval = 2.5;
        
        // Check if start time is close to a keyframe
        let start_remainder = config.start_time % keyframe_interval;
        let start_aligned = start_remainder < 0.1 || start_remainder > (keyframe_interval - 0.1);
        
        // Check if end time is close to a keyframe
        let end_remainder = config.end_time % keyframe_interval;
        let end_aligned = end_remainder < 0.1 || end_remainder > (keyframe_interval - 0.1);
        
        start_aligned && end_aligned
    }
}

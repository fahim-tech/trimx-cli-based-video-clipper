//! Hybrid clipping implementation combining stream copy and re-encoding

use std::time::Instant;
use std::sync::Arc;
use tracing::{info, warn, debug};
use crate::engine::{EngineConfig, ClippingProgress, ClippingPhase, StreamCopyClipper, ReencodeClipper, ProgressTracker, ProgressPhase};
use crate::planner::{CutPlan, ClippingStrategy};
use crate::error::{TrimXError, TrimXResult};

/// Hybrid clipper that uses GOP-spanning method:
/// - Re-encode only the leading and trailing GOPs that need frame-accurate cuts
/// - Stream copy the middle section for maximum quality and speed
pub struct HybridClipper {
    /// Enable debug logging
    debug: bool,
    /// Stream copy engine for middle section
    copy_engine: StreamCopyClipper,
    /// Re-encoding engine for GOP boundaries
    reencode_engine: ReencodeClipper,
    /// Minimum segment duration to justify hybrid approach (seconds)
    min_copy_duration: f64,
}

impl HybridClipper {
    /// Create a new hybrid clipper
    pub fn new() -> Self {
        Self {
            debug: false,
            copy_engine: StreamCopyClipper::new(),
            reencode_engine: ReencodeClipper::new(),
            min_copy_duration: 2.0, // Minimum 2 seconds for stream copy to be worthwhile
        }
    }

    /// Enable debug logging
    pub fn with_debug(mut self) -> Self {
        self.debug = true;
        self.copy_engine = self.copy_engine.with_debug();
        self.reencode_engine = self.reencode_engine.with_debug();
        self
    }

    /// Set minimum duration for middle section to use stream copy
    pub fn with_min_copy_duration(mut self, duration: f64) -> Self {
        self.min_copy_duration = duration;
        self
    }

    /// Set encoding quality for re-encoded sections
    pub fn with_quality(mut self, crf: u8) -> Self {
        self.reencode_engine = self.reencode_engine.with_crf(crf);
        self
    }

    /// Execute hybrid clipping using the provided cut plan
    pub fn clip(&self, config: EngineConfig, plan: CutPlan) -> TrimXResult<ClippingProgress> {
        self.clip_with_progress(config, plan, None)
    }

    /// Execute hybrid clipping with custom progress tracker
    pub fn clip_with_progress(&self, config: EngineConfig, plan: CutPlan, progress_tracker: Option<Arc<ProgressTracker>>) -> TrimXResult<ClippingProgress> {
        let tracker = progress_tracker.unwrap_or_else(|| Arc::new(ProgressTracker::new("Hybrid Clipping")));
        
        tracker.start("Hybrid clipping operation", None);
        tracker.set_phase(ProgressPhase::Initializing, Some("Starting hybrid clipping".to_string()));

        let _start_time = Instant::now();
        info!("Starting hybrid clipping operation");
        info!("Input: {}", config.input_path);
        info!("Output: {}", config.output_path);
        info!("Time range: {:.3}s - {:.3}s", config.start_time, config.end_time);

        // Check for cancellation
        if tracker.is_cancelled() {
            return Err(TrimXError::ClippingError {
                message: "Operation cancelled during initialization".to_string()
            });
        }

        // Validate configuration
        tracker.set_phase(ProgressPhase::Planning, Some("Validating configuration".to_string()));
        if let Err(e) = self.validate_config(&config, &plan) {
            tracker.error(&e.to_string());
            return Err(e);
        }

        // Determine hybrid strategy
        tracker.update(10, Some("Planning hybrid strategy".to_string()));
        let strategy = match self.plan_hybrid_strategy(&config, &plan) {
            Ok(s) => s,
            Err(e) => {
                tracker.error(&e.to_string());
                return Err(e);
            }
        };

        match strategy {
            HybridStrategy::FullStreamCopy => {
                info!("Using full stream copy (cuts align with keyframes)");
                self.copy_engine.clip(config)
            }
            HybridStrategy::FullReencode => {
                info!("Using full re-encoding (cuts don't align well with keyframes)");
                self.reencode_engine.clip(config)
            }
            HybridStrategy::ThreeWay(segments) => {
                info!("Using three-way hybrid approach");
                self.execute_three_way_hybrid(config, segments)
            }
        }
    }

    /// Validate configuration and plan for hybrid clipping
    fn validate_config(&self, config: &EngineConfig, plan: &CutPlan) -> TrimXResult<()> {
        // Basic validation
        if !std::path::Path::new(&config.input_path).exists() {
            return Err(TrimXError::InputFileNotFound {
                path: config.input_path.clone()
            });
        }

        if config.start_time >= config.end_time {
            return Err(TrimXError::ClippingError {
                message: format!(
                    "Invalid time range: start ({:.3}s) must be before end ({:.3}s)",
                    config.start_time, config.end_time
                )
            });
        }

        // Validate plan strategy
        if !matches!(plan.strategy, ClippingStrategy::Hybrid | ClippingStrategy::Auto) {
            return Err(TrimXError::ClippingError {
                message: format!("Invalid strategy for hybrid clipper: {:?}", plan.strategy)
            });
        }

        Ok(())
    }

    /// Plan the optimal hybrid strategy based on keyframe analysis
    fn plan_hybrid_strategy(&self, config: &EngineConfig, plan: &CutPlan) -> TrimXResult<HybridStrategy> {
        let duration = config.end_time - config.start_time;
        
        // If very short clip, just use re-encoding
        if duration < 1.0 {
            debug!("Short clip ({:.2}s), using full re-encoding", duration);
            return Ok(HybridStrategy::FullReencode);
        }

        // Analyze keyframe alignment
        let keyframe_info = &plan.keyframe_info;
        
        // Check if cuts align well with keyframes
        let start_aligned = self.is_time_near_keyframe(config.start_time, keyframe_info)?;
        let end_aligned = self.is_time_near_keyframe(config.end_time, keyframe_info)?;

        if start_aligned && end_aligned {
            debug!("Both cuts align with keyframes, using full stream copy");
            return Ok(HybridStrategy::FullStreamCopy);
        }

        if !start_aligned && !end_aligned && duration < self.min_copy_duration * 3.0 {
            debug!("Short clip with no keyframe alignment, using full re-encoding");
            return Ok(HybridStrategy::FullReencode);
        }

        // Plan three-way hybrid approach
        let segments = self.plan_three_way_segments(config, keyframe_info)?;
        
        // Validate that middle segment is worth stream copying
        let middle_duration = segments.middle_end - segments.middle_start;
        if middle_duration < self.min_copy_duration {
            debug!("Middle segment too short ({:.2}s), using full re-encoding", middle_duration);
            return Ok(HybridStrategy::FullReencode);
        }

        info!("Using three-way hybrid: leading {:.2}s, middle {:.2}s, trailing {:.2}s",
              segments.middle_start - config.start_time,
              middle_duration,
              config.end_time - segments.middle_end);

        Ok(HybridStrategy::ThreeWay(segments))
    }

    /// Check if a time point is near a keyframe
    fn is_time_near_keyframe(&self, time: f64, keyframe_info: &crate::planner::KeyframeInfo) -> TrimXResult<bool> {
        // For now, use a simple heuristic based on GOP size
        // In a full implementation, we'd check actual keyframe positions
        
        let gop_size = keyframe_info.gop_size.unwrap_or(2.0); // Default 2 second GOP
        let tolerance = gop_size * 0.1; // 10% of GOP size
        
        // Check if time is close to any multiple of GOP size
        let gop_position = time % gop_size;
        let aligned = gop_position < tolerance || gop_position > (gop_size - tolerance);
        
        if self.debug {
            debug!("Time {:.3}s, GOP size {:.2}s, position {:.3}s, aligned: {}", 
                   time, gop_size, gop_position, aligned);
        }
        
        Ok(aligned)
    }

    /// Plan three-way segmentation for hybrid approach
    fn plan_three_way_segments(&self, config: &EngineConfig, keyframe_info: &crate::planner::KeyframeInfo) -> TrimXResult<ThreeWaySegments> {
        let gop_size = keyframe_info.gop_size.unwrap_or(2.0);
        
        // Find first keyframe after start time
        let middle_start = if let Some(next_kf) = keyframe_info.next_keyframe {
            next_kf
        } else {
            // Estimate next keyframe position
            let gops_from_start = (config.start_time / gop_size).ceil();
            gops_from_start * gop_size
        };

        // Find last keyframe before end time
        let middle_end = if let Some(end_kf) = keyframe_info.end_keyframe {
            end_kf
        } else {
            // Estimate previous keyframe position
            let gops_from_start = (config.end_time / gop_size).floor();
            gops_from_start * gop_size
        };

        // Ensure segments make sense
        let middle_start = middle_start.max(config.start_time + 0.1); // At least 0.1s leading
        let middle_end = middle_end.min(config.end_time - 0.1); // At least 0.1s trailing

        Ok(ThreeWaySegments {
            middle_start,
            middle_end,
        })
    }

    /// Execute three-way hybrid clipping
    fn execute_three_way_hybrid(&self, config: EngineConfig, segments: ThreeWaySegments) -> TrimXResult<ClippingProgress> {
        let temp_dir = std::env::temp_dir().join("trimx_hybrid");
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| TrimXError::IoError(e))?;

        // Generate temporary file names
        let leading_file = temp_dir.join("leading.mp4");
        let middle_file = temp_dir.join("middle.mp4");
        let trailing_file = temp_dir.join("trailing.mp4");

        let mut total_progress = 0.0;

        // Step 1: Re-encode leading segment (start to first keyframe)
        info!("Step 1/4: Re-encoding leading segment ({:.3}s to {:.3}s)", 
              config.start_time, segments.middle_start);
        
        let leading_config = EngineConfig {
            start_time: config.start_time,
            end_time: segments.middle_start,
            output_path: leading_file.to_string_lossy().to_string(),
            ..config.clone()
        };
        
        self.reencode_engine.clip(leading_config)?;
        total_progress += 25.0;

        // Step 2: Stream copy middle segment (keyframe to keyframe)
        info!("Step 2/4: Stream copying middle segment ({:.3}s to {:.3}s)", 
              segments.middle_start, segments.middle_end);
        
        let middle_config = EngineConfig {
            start_time: segments.middle_start,
            end_time: segments.middle_end,
            output_path: middle_file.to_string_lossy().to_string(),
            ..config.clone()
        };
        
        self.copy_engine.clip(middle_config)?;
        total_progress += 25.0;

        // Step 3: Re-encode trailing segment (last keyframe to end)
        info!("Step 3/4: Re-encoding trailing segment ({:.3}s to {:.3}s)", 
              segments.middle_end, config.end_time);
        
        let trailing_config = EngineConfig {
            start_time: segments.middle_end,
            end_time: config.end_time,
            output_path: trailing_file.to_string_lossy().to_string(),
            ..config.clone()
        };
        
        self.reencode_engine.clip(trailing_config)?;
        let _ = total_progress += 25.0;

        // Step 4: Concatenate segments
        info!("Step 4/4: Concatenating segments");
        self.concatenate_segments(
            &[
                leading_file.to_string_lossy().to_string(),
                middle_file.to_string_lossy().to_string(),
                trailing_file.to_string_lossy().to_string(),
            ],
            &config.output_path,
        )?;
        total_progress = 100.0;

        // Cleanup temporary files
        let _ = std::fs::remove_dir_all(&temp_dir);

        Ok(ClippingProgress {
            phase: ClippingPhase::Completed,
            progress: total_progress,
            description: "Hybrid clipping completed successfully".to_string(),
            eta: None,
        })
    }

    /// Concatenate video segments using FFmpeg
    fn concatenate_segments(&self, input_files: &[String], output_file: &str) -> TrimXResult<()> {
        info!("Concatenating {} segments into {}", input_files.len(), output_file);

        if input_files.is_empty() {
            return Err(TrimXError::ClippingError {
                message: "No input files to concatenate".to_string()
            });
        }

        if input_files.len() == 1 {
            // Single file, just copy
            std::fs::copy(&input_files[0], output_file)
                .map_err(|e| TrimXError::IoError(e))?;
            return Ok(());
        }

        // Initialize FFmpeg
        ffmpeg_next::init().map_err(|e| TrimXError::ClippingError {
            message: format!("Failed to initialize FFmpeg: {}", e)
        })?;

        // Create concat list file
        let concat_list_path = self.create_concat_list(input_files)?;
        
        // Use FFmpeg concat demuxer for proper concatenation
        match self.concat_with_demuxer(&concat_list_path, output_file) {
            Ok(()) => {
                // Cleanup concat list file
                let _ = std::fs::remove_file(&concat_list_path);
                info!("Successfully concatenated {} segments", input_files.len());
                Ok(())
            }
            Err(e) => {
                // Cleanup concat list file on error
                let _ = std::fs::remove_file(&concat_list_path);
                
                // Fallback to manual concatenation
                warn!("Demuxer concatenation failed: {}, trying manual approach", e);
                self.concat_manually(input_files, output_file)
            }
        }
    }

    /// Create a concat list file for FFmpeg demuxer
    fn create_concat_list(&self, input_files: &[String]) -> TrimXResult<String> {
        let temp_dir = std::env::temp_dir().join("trimx_concat");
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| TrimXError::IoError(e))?;

        let concat_list_path = temp_dir.join("concat_list.txt");
        
        let mut list_content = String::new();
        for file in input_files {
            // Escape file paths for FFmpeg
            let escaped_path = file.replace("'", "'\"'\"'");
            list_content.push_str(&format!("file '{}'\n", escaped_path));
        }

        std::fs::write(&concat_list_path, list_content)
            .map_err(|e| TrimXError::IoError(e))?;

        Ok(concat_list_path.to_string_lossy().to_string())
    }

    /// Concatenate using FFmpeg concat demuxer
    fn concat_with_demuxer(&self, concat_list_path: &str, output_file: &str) -> TrimXResult<()> {
        // Open concat demuxer input
        let mut input_ctx = ffmpeg_next::format::input_with_dictionary(
            concat_list_path,
            ffmpeg_next::Dictionary::new()
        ).map_err(|e| TrimXError::ClippingError {
            message: format!("Failed to open concat list: {}", e)
        })?;

        // Set format to concat
        // Note: This might need adjustment based on ffmpeg-next API
        
        // Create output context
        let mut output_ctx = ffmpeg_next::format::output(output_file)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to create output file: {}", e)
            })?;

        // Copy stream information
        for input_stream in input_ctx.streams() {
            let mut output_stream = output_ctx.add_stream(
                ffmpeg_next::codec::encoder::find(ffmpeg_next::codec::Id::None)
            ).map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to add output stream: {}", e)
            })?;

            output_stream.set_parameters(input_stream.parameters());
            output_stream.set_time_base(input_stream.time_base());
        }

        // Write header
        output_ctx.write_header()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to write output header: {}", e)
            })?;

        // Copy all packets
        for (input_stream, mut packet) in input_ctx.packets() {
            if let Some(output_stream) = output_ctx.stream(input_stream.index()) {
                packet.rescale_ts(input_stream.time_base(), output_stream.time_base());
                packet.write_interleaved(&mut output_ctx)
                    .map_err(|e| TrimXError::ClippingError {
                        message: format!("Failed to write packet: {}", e)
                    })?;
            }
        }

        // Write trailer
        output_ctx.write_trailer()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to write output trailer: {}", e)
            })?;

        Ok(())
    }

    /// Manual concatenation fallback
    fn concat_manually(&self, input_files: &[String], output_file: &str) -> TrimXResult<()> {
        info!("Using manual concatenation fallback");

        // Open first file to get format information
        let first_input = ffmpeg_next::format::input(&input_files[0])
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open first input file: {}", e)
        })?;

        // Create output context
        let mut output_ctx = ffmpeg_next::format::output(output_file)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to create output file: {}", e)
            })?;

        // Copy stream information from first file
        for input_stream in first_input.streams() {
            let mut output_stream = output_ctx.add_stream(
                ffmpeg_next::codec::encoder::find(ffmpeg_next::codec::Id::None)
            ).map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to add output stream: {}", e)
            })?;

            output_stream.set_parameters(input_stream.parameters());
            output_stream.set_time_base(input_stream.time_base());
        }

        // Write header
        output_ctx.write_header()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to write output header: {}", e)
            })?;

        let mut current_dts = vec![0i64; output_ctx.nb_streams() as usize];

        // Process each input file
        for (file_index, input_file) in input_files.iter().enumerate() {
            info!("Processing segment {}/{}: {}", file_index + 1, input_files.len(), input_file);

            let mut input_ctx = ffmpeg_next::format::input(input_file)
                .map_err(|e| TrimXError::ClippingError {
                    message: format!("Failed to open input file {}: {}", input_file, e)
                })?;

            // Copy packets with timestamp adjustment
            for (input_stream, mut packet) in input_ctx.packets() {
                let stream_index = input_stream.index();
                
                if let Some(output_stream) = output_ctx.stream(stream_index) {
                    // Adjust timestamps for seamless concatenation
                    if let Some(pts) = packet.pts() {
                        packet.set_pts(Some(pts + current_dts[stream_index]));
                    }
                    if let Some(dts) = packet.dts() {
                        packet.set_dts(Some(dts + current_dts[stream_index]));
                    }

                    packet.rescale_ts(input_stream.time_base(), output_stream.time_base());
                    packet.write_interleaved(&mut output_ctx)
                    .map_err(|e| TrimXError::ClippingError {
                            message: format!("Failed to write packet: {}", e)
                        })?;

                    // Update DTS offset for next file
                    if let Some(dts) = packet.dts() {
                        current_dts[stream_index] = current_dts[stream_index].max(dts + 1);
                    }
                }
            }
        }

        // Write trailer
        output_ctx.write_trailer()
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to write output trailer: {}", e)
            })?;

        info!("Manual concatenation completed successfully");
        Ok(())
    }

    /// Estimate output size for hybrid clipping
    pub fn estimate_output_size(&self, config: &EngineConfig, plan: &CutPlan) -> TrimXResult<u64> {
        let strategy = self.plan_hybrid_strategy(config, plan)?;

        match strategy {
            HybridStrategy::FullStreamCopy => {
                self.copy_engine.estimate_output_size(config)
            }
            HybridStrategy::FullReencode => {
                self.reencode_engine.estimate_output_size(config)
            }
            HybridStrategy::ThreeWay(segments) => {
                // Estimate each segment separately
                let leading_duration = segments.middle_start - config.start_time;
                let middle_duration = segments.middle_end - segments.middle_start;
                let trailing_duration = config.end_time - segments.middle_end;
                
                // Re-encoded segments (leading + trailing)
                let reencode_duration = leading_duration + trailing_duration;
                let reencode_config = EngineConfig {
                    start_time: 0.0,
                    end_time: reencode_duration,
                    ..config.clone()
                };
                let reencode_size = self.reencode_engine.estimate_output_size(&reencode_config)?;

                // Stream copied segment (middle)
                let copy_config = EngineConfig {
                    start_time: 0.0,
                    end_time: middle_duration,
                    ..config.clone()
                };
                let copy_size = self.copy_engine.estimate_output_size(&copy_config)?;

                Ok(reencode_size + copy_size)
            }
        }
    }
}

/// Strategy for hybrid clipping approach
#[derive(Debug, Clone)]
enum HybridStrategy {
    /// Use full stream copy (cuts align with keyframes)
    FullStreamCopy,
    /// Use full re-encoding (cuts don't align well)
    FullReencode,
    /// Use three-way approach: reencode + copy + reencode
    ThreeWay(ThreeWaySegments),
}

/// Segments for three-way hybrid approach
#[derive(Debug, Clone)]
struct ThreeWaySegments {
    /// Start time of middle (stream copy) segment
    middle_start: f64,
    /// End time of middle (stream copy) segment
    middle_end: f64,
}
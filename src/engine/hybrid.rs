//! Hybrid clipping implementation combining stream copy and re-encoding

use std::time::Instant;
use std::sync::Arc;
use tracing::{info, warn, debug};
use crate::engine::{EngineConfig, ClippingProgress, ClippingPhase, StreamCopyClipper, ReencodeClipper, ProgressTracker};
use crate::engine::progress::ProgressPhase;
use crate::planner::{CutPlan, ClippingStrategy};
use crate::planner::keyframe_analyzer::{KeyframeAnalyzer, GOPAnalysis};
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
    /// Keyframe analyzer for GOP detection
    keyframe_analyzer: KeyframeAnalyzer,
}

impl HybridClipper {
    /// Create a new hybrid clipper
    pub fn new() -> Self {
        Self {
            debug: false,
            copy_engine: StreamCopyClipper::new(),
            reencode_engine: ReencodeClipper::new(),
            min_copy_duration: 2.0, // Minimum 2 seconds for stream copy to be worthwhile
            keyframe_analyzer: KeyframeAnalyzer::new(),
        }
    }

    /// Enable debug logging
    pub fn with_debug(mut self) -> Self {
        self.debug = true;
        self.copy_engine = self.copy_engine.with_debug();
        self.reencode_engine = self.reencode_engine.with_debug();
        self.keyframe_analyzer = self.keyframe_analyzer.with_debug();
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
    fn plan_hybrid_strategy(&self, config: &EngineConfig, _plan: &CutPlan) -> TrimXResult<HybridStrategy> {
        let duration = config.end_time - config.start_time;
        
        // If very short clip, just use re-encoding
        if duration < 1.0 {
            debug!("Short clip ({:.2}s), using full re-encoding", duration);
            return Ok(HybridStrategy::FullReencode);
        }

        // Perform comprehensive GOP analysis
        let gop_analysis = self.perform_gop_analysis(&config.input_path)?;
        
        // Check if cuts align well with keyframes
        let start_aligned = self.keyframe_analyzer.is_keyframe_aligned(&gop_analysis, config.start_time);
        let end_aligned = self.keyframe_analyzer.is_keyframe_aligned(&gop_analysis, config.end_time);

        if start_aligned && end_aligned {
            debug!("Both cuts align with keyframes, using full stream copy");
            return Ok(HybridStrategy::FullStreamCopy);
        }

        if !start_aligned && !end_aligned && duration < self.min_copy_duration * 3.0 {
            debug!("Short clip with no keyframe alignment, using full re-encoding");
            return Ok(HybridStrategy::FullReencode);
        }

        // Plan three-way hybrid approach using actual GOP boundaries
        let segments = self.plan_three_way_segments_with_gop_analysis(config, &gop_analysis)?;
        
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

    /// Perform comprehensive GOP analysis on the input file
    fn perform_gop_analysis(&self, input_path: &str) -> TrimXResult<GOPAnalysis> {
        info!("Performing GOP analysis for hybrid clipping: {}", input_path);
        
        // Find video stream index
        let input_ctx = ffmpeg_next::format::input(input_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e)
            })?;

        let video_stream_index = input_ctx.streams()
            .enumerate()
            .find(|(_, stream)| stream.parameters().medium() == ffmpeg_next::media::Type::Video)
            .map(|(index, _)| index)
            .ok_or_else(|| TrimXError::ClippingError {
                message: "No video stream found".to_string()
            })?;

        // Perform comprehensive GOP analysis
        let gop_analysis = self.keyframe_analyzer.analyze_gop_structure(input_path, video_stream_index)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("GOP analysis failed: {}", e)
            })?;

        if self.debug {
            info!("GOP Analysis Results:");
            info!("  Total keyframes: {}", gop_analysis.keyframe_count);
            info!("  Average GOP duration: {:.3}s", gop_analysis.avg_gop_duration);
            info!("  Regularity score: {:.2}", gop_analysis.regularity_score);
            if let Some(ref pattern) = gop_analysis.gop_pattern {
                info!("  Detected pattern: {}", pattern);
            }
        }

        Ok(gop_analysis)
    }

    /// Plan three-way segmentation using comprehensive GOP analysis
    fn plan_three_way_segments_with_gop_analysis(&self, config: &EngineConfig, gop_analysis: &GOPAnalysis) -> TrimXResult<ThreeWaySegments> {
        // Find optimal cut points using actual keyframe positions
        let (_optimal_start, _optimal_end) = self.keyframe_analyzer.find_optimal_cut_points(
            gop_analysis, config.start_time, config.end_time
        );

        // Find the first keyframe after the start time
        let middle_start = gop_analysis.keyframes.iter()
            .find(|kf| kf.timestamp > config.start_time)
            .map(|kf| kf.timestamp)
            .unwrap_or_else(|| {
                // Fallback: estimate next keyframe based on GOP size
                let gop_size = gop_analysis.avg_gop_duration;
                let gops_from_start = (config.start_time / gop_size).ceil();
                gops_from_start * gop_size
            });

        // Find the last keyframe before the end time
        let middle_end = gop_analysis.keyframes.iter()
            .rev()
            .find(|kf| kf.timestamp < config.end_time)
            .map(|kf| kf.timestamp)
            .unwrap_or_else(|| {
                // Fallback: estimate previous keyframe based on GOP size
                let gop_size = gop_analysis.avg_gop_duration;
                let gops_from_start = (config.end_time / gop_size).floor();
                gops_from_start * gop_size
            });

        // Ensure segments make sense and provide reasonable leading/trailing segments
        let min_leading_duration = 0.1; // Minimum 0.1s leading segment
        let min_trailing_duration = 0.1; // Minimum 0.1s trailing segment
        
        let middle_start = middle_start.max(config.start_time + min_leading_duration);
        let middle_end = middle_end.min(config.end_time - min_trailing_duration);

        // Validate that we have a reasonable middle segment
        if middle_start >= middle_end {
            return Err(TrimXError::ClippingError {
                message: "No suitable GOP boundaries found for hybrid approach".to_string()
            });
        }

        if self.debug {
            debug!("Three-way segmentation:");
            debug!("  Original range: {:.3}s - {:.3}s", config.start_time, config.end_time);
            debug!("  Leading segment: {:.3}s - {:.3}s", config.start_time, middle_start);
            debug!("  Middle segment: {:.3}s - {:.3}s", middle_start, middle_end);
            debug!("  Trailing segment: {:.3}s - {:.3}s", middle_end, config.end_time);
        }

        Ok(ThreeWaySegments {
            middle_start,
            middle_end,
        })
    }

    /// Execute three-way hybrid clipping with improved error handling and progress tracking
    fn execute_three_way_hybrid(&self, config: EngineConfig, segments: ThreeWaySegments) -> TrimXResult<ClippingProgress> {
        let temp_dir = std::env::temp_dir().join("trimx_hybrid");
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| TrimXError::IoError(e))?;

        // Generate temporary file names with unique identifiers
        let leading_file = temp_dir.join("leading.mp4");
        let middle_file = temp_dir.join("middle.mp4");
        let trailing_file = temp_dir.join("trailing.mp4");

        let mut total_progress = 0.0;
        let mut segment_files = Vec::new();

        // Step 1: Re-encode leading segment (start to first keyframe)
        let leading_duration = segments.middle_start - config.start_time;
        if leading_duration > 0.01 { // Only process if segment is meaningful
            info!("Step 1/4: Re-encoding leading segment ({:.3}s to {:.3}s, duration: {:.3}s)", 
                  config.start_time, segments.middle_start, leading_duration);
            
            let leading_config = EngineConfig {
                start_time: config.start_time,
                end_time: segments.middle_start,
                output_path: leading_file.to_string_lossy().to_string(),
                ..config.clone()
            };
            
            self.reencode_engine.clip(leading_config)
                .map_err(|e| TrimXError::ClippingError {
                    message: format!("Failed to re-encode leading segment: {}", e)
                })?;
            
            segment_files.push(leading_file.to_string_lossy().to_string());
            let _ = total_progress + 25.0;
            info!("Leading segment completed successfully");
        }

        // Step 2: Stream copy middle segment (keyframe to keyframe)
        let middle_duration = segments.middle_end - segments.middle_start;
        if middle_duration > 0.01 { // Only process if segment is meaningful
            info!("Step 2/4: Stream copying middle segment ({:.3}s to {:.3}s, duration: {:.3}s)", 
                  segments.middle_start, segments.middle_end, middle_duration);
            
            let middle_config = EngineConfig {
                start_time: segments.middle_start,
                end_time: segments.middle_end,
                output_path: middle_file.to_string_lossy().to_string(),
                ..config.clone()
            };
            
            self.copy_engine.clip(middle_config)
                .map_err(|e| TrimXError::ClippingError {
                    message: format!("Failed to stream copy middle segment: {}", e)
                })?;
            
            segment_files.push(middle_file.to_string_lossy().to_string());
            let _ = total_progress + 25.0;
            info!("Middle segment completed successfully");
        }

        // Step 3: Re-encode trailing segment (last keyframe to end)
        let trailing_duration = config.end_time - segments.middle_end;
        if trailing_duration > 0.01 { // Only process if segment is meaningful
            info!("Step 3/4: Re-encoding trailing segment ({:.3}s to {:.3}s, duration: {:.3}s)", 
                  segments.middle_end, config.end_time, trailing_duration);
            
            let trailing_config = EngineConfig {
                start_time: segments.middle_end,
                end_time: config.end_time,
                output_path: trailing_file.to_string_lossy().to_string(),
                ..config.clone()
            };
            
            self.reencode_engine.clip(trailing_config)
                .map_err(|e| TrimXError::ClippingError {
                    message: format!("Failed to re-encode trailing segment: {}", e)
                })?;
            
            segment_files.push(trailing_file.to_string_lossy().to_string());
            let _ = total_progress + 25.0;
            info!("Trailing segment completed successfully");
        }

        // Step 4: Concatenate segments
        info!("Step 4/4: Concatenating {} segments", segment_files.len());
        
        if segment_files.is_empty() {
            return Err(TrimXError::ClippingError {
                message: "No segments to concatenate".to_string()
            });
        }

        self.concatenate_segments(&segment_files, &config.output_path)
            .map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to concatenate segments: {}", e)
            })?;
        
        total_progress = 100.0;

        // Cleanup temporary files
        if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
            warn!("Failed to cleanup temporary directory {:?}: {}", temp_dir, e);
        }

        info!("Hybrid clipping completed successfully with {} segments", segment_files.len());
        Ok(ClippingProgress {
            phase: ClippingPhase::Completed,
            progress: total_progress,
            description: format!("Hybrid clipping completed with {} segments", segment_files.len()),
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
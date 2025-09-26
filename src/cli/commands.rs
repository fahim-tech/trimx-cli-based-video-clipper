//! Command implementations

use std::path::Path;
use anyhow::{Result, Context};
use tracing::{info, warn, error};

use crate::cli::args::{ClipArgs, InspectArgs, VerifyArgs};
use crate::probe::inspector::VideoInspector;
use crate::engine::{clipper::VideoClipper, EngineConfig};
use crate::planner::strategy::StrategyPlanner;
use crate::output::verifier::ClipVerifier;
use crate::domain::model::{TimeSpec, ClippingMode};
use crate::error::{TrimXError, TrimXResult};

/// Execute the clip command
pub fn clip(args: ClipArgs) -> Result<()> {
    info!("Starting clip operation");
    info!("Input: {}", args.input);
    info!("Start: {}", args.start);
    info!("End: {}", args.end);
    info!("Mode: {}", args.mode);

    // Validate input file exists
    if !Path::new(&args.input).exists() {
        return Err(anyhow::anyhow!("Input file does not exist: {}", args.input));
    }

    // Parse time arguments
    let start_time = TimeSpec::parse(&args.start)
        .map_err(|e| anyhow::anyhow!("Invalid start time '{}': {}", args.start, e))?;
    let end_time = TimeSpec::parse(&args.end)
        .map_err(|e| anyhow::anyhow!("Invalid end time '{}': {}", args.end, e))?;

    let start_seconds = start_time.as_seconds();
    let end_seconds = end_time.as_seconds();

    if start_seconds >= end_seconds {
        return Err(anyhow::anyhow!("Start time must be before end time"));
    }

    // Generate output filename if not provided
    let output_path = if let Some(output) = args.output {
        output
    } else {
        generate_output_filename(&args.input, start_seconds, end_seconds)?
    };

    info!("Output: {}", output_path);

    // Probe video file
    let inspector = VideoInspector::new()?;
    let media_info = inspector.inspect(&args.input)
        .context("Failed to inspect input file")?;

    info!("Media info: {} streams, duration: {:.2}s", 
        media_info.total_streams(), media_info.duration);

    // Plan clipping strategy
    let planner = StrategyPlanner::new();
    let cut_plan = planner.plan_strategy(
        &args.input,
        &media_info,
        start_seconds,
        end_seconds,
        &args.mode,
    ).context("Failed to plan clipping strategy")?;

    info!("Selected strategy: {:?}", cut_plan.strategy);

    // Create engine configuration
    let engine_config = EngineConfig {
        input_path: args.input.clone(),
        output_path: output_path.clone(),
        start_time: start_seconds,
        end_time: end_seconds,
        video_codec: args.codec.unwrap_or_else(|| "h264".to_string()),
        audio_codec: None, // Keep original audio codec
        crf: args.crf.unwrap_or(18),
        preset: args.preset.unwrap_or_else(|| "medium".to_string()),
        no_audio: args.no_audio,
        no_subs: args.no_subs,
    };

    // Execute clipping
    let clipper = VideoClipper::new();
    let progress = clipper.clip(engine_config, cut_plan)
        .context("Failed to execute clipping")?;

    info!("Clipping completed: {}", progress.description);

    // Verify output if requested
    if args.verify {
        info!("Verifying output file");
        let verifier = ClipVerifier::new()?;
        let verification_result = verifier.verify(&output_path, start_seconds, end_seconds)
            .context("Failed to verify output")?;

        if verification_result.success {
            info!("Verification passed: {:.1}% overall score", verification_result.overall_score);
        } else {
            warn!("Verification failed: {}", verification_result.error_message);
        }
    }

    info!("Clip operation completed successfully");
    Ok(())
}

/// Execute the inspect command
pub fn inspect(args: InspectArgs) -> Result<()> {
    info!("Starting inspect operation");
    info!("Input: {}", args.input);

    // Validate input file exists
    if !Path::new(&args.input).exists() {
        return Err(anyhow::anyhow!("Input file does not exist: {}", args.input));
    }

    // Probe video file
    let inspector = VideoInspector::new()?;
    let media_info = inspector.inspect(&args.input)
        .context("Failed to inspect input file")?;

    // Display information
    if args.json {
        // Output JSON format
        let json = serde_json::to_string_pretty(&media_info)
            .context("Failed to serialize media info to JSON")?;
        println!("{}", json);
    } else {
        // Output human-readable format
        display_media_info(&media_info);
    }

    info!("Inspect operation completed successfully");
    Ok(())
}

/// Execute the verify command
pub fn verify(args: VerifyArgs) -> Result<()> {
    info!("Starting verify operation");
    info!("Input: {}", args.input);
    info!("Start: {}", args.start);
    info!("End: {}", args.end);

    // Validate input file exists
    if !Path::new(&args.input).exists() {
        return Err(anyhow::anyhow!("Input file does not exist: {}", args.input));
    }

    // Parse time arguments
    let start_time = TimeSpec::parse(&args.start)
        .map_err(|e| anyhow::anyhow!("Invalid start time '{}': {}", args.start, e))?;
    let end_time = TimeSpec::parse(&args.end)
        .map_err(|e| anyhow::anyhow!("Invalid end time '{}': {}", args.end, e))?;

    let start_seconds = start_time.as_seconds();
    let end_seconds = end_time.as_seconds();

    if start_seconds >= end_seconds {
        return Err(anyhow::anyhow!("Start time must be before end time"));
    }

    // Verify clipped file
    let verifier = ClipVerifier::new()?;
    let verification_result = verifier.verify(&args.input, start_seconds, end_seconds)
        .context("Failed to verify clipped file")?;

    // Display results
    if args.json {
        // Output JSON format
        let json = serde_json::to_string_pretty(&verification_result)
            .context("Failed to serialize verification result to JSON")?;
        println!("{}", json);
    } else {
        // Output human-readable format
        display_verification_result(&verification_result);
    }

    if verification_result.success {
        info!("Verify operation completed successfully");
        Ok(())
    } else {
        error!("Verification failed: {}", verification_result.error_message);
        Err(anyhow::anyhow!("Verification failed"))
    }
}

/// Generate output filename based on input and time range
fn generate_output_filename(input_path: &str, start_time: f64, end_time: f64) -> Result<String> {
    let path = Path::new(input_path);
    let stem = path.file_stem()
        .ok_or_else(|| anyhow::anyhow!("Invalid input file path"))?
        .to_string_lossy();
    let extension = path.extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_else(|| ".mp4".to_string());

    let start_str = format_time_short(start_time);
    let end_str = format_time_short(end_time);
    
    Ok(format!("{}_clip_{}_{}{}", stem, start_str, end_str, extension))
}

/// Format time as short string for filename
fn format_time_short(seconds: f64) -> String {
    let total_seconds = seconds as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;
    let ms = ((seconds - total_seconds as f64) * 1000.0) as u64;

    if hours > 0 {
        format!("{:02}h{:02}m{:02}s{:03}ms", hours, minutes, secs, ms)
    } else if minutes > 0 {
        format!("{:02}m{:02}s{:03}ms", minutes, secs, ms)
    } else {
        format!("{:02}s{:03}ms", secs, ms)
    }
}

/// Display media information in human-readable format
fn display_media_info(media_info: &crate::probe::MediaInfo) {
    println!("Media Information");
    println!("=================");
    println!("File: {}", media_info.file_path);
    println!("Format: {}", media_info.format);
    println!("Duration: {:.3}s", media_info.duration);
    println!("File Size: {} bytes", media_info.file_size);
    println!("Bit Rate: {} bps", media_info.bit_rate.unwrap_or(0));
    println!();

    if !media_info.video_streams.is_empty() {
        println!("Video Streams:");
        for (i, stream) in media_info.video_streams.iter().enumerate() {
            println!("  Stream {}: {}x{} @ {:.2} fps", 
                i, stream.width, stream.height, stream.frame_rate);
            println!("    Codec: {}", stream.codec);
            println!("    Bit Rate: {} bps", stream.bit_rate.unwrap_or(0));
        }
        println!();
    }

    if !media_info.audio_streams.is_empty() {
        println!("Audio Streams:");
        for (i, stream) in media_info.audio_streams.iter().enumerate() {
            println!("  Stream {}: {} Hz, {} channels", 
                i, stream.sample_rate, stream.channels);
            println!("    Codec: {}", stream.codec);
            println!("    Bit Rate: {} bps", stream.bit_rate.unwrap_or(0));
        }
        println!();
    }

    if !media_info.subtitle_streams.is_empty() {
        println!("Subtitle Streams:");
        for (i, stream) in media_info.subtitle_streams.iter().enumerate() {
            println!("  Stream {}: {}", i, stream.codec);
        }
        println!();
    }

    if !media_info.metadata.is_empty() {
        println!("Metadata:");
        for (key, value) in &media_info.metadata {
            println!("  {}: {}", key, value);
        }
    }
}

/// Display verification result in human-readable format
fn display_verification_result(result: &crate::output::verifier::VerificationResult) {
    println!("Verification Results");
    println!("===================");
    println!("Overall Score: {:.1}%", result.overall_score);
    println!("Success: {}", if result.success { "✓" } else { "✗" });
    println!();

    if let Some(error_msg) = &result.error_message {
        println!("Error: {}", error_msg);
        println!();
    }

    println!("Checks:");
    for check in &result.checks {
        let status = if check.success { "✓" } else { "✗" };
        println!("  {} {}: {}", status, check.check_type, check.details);
        if let Some(error_msg) = &check.error_message {
            println!("    Error: {}", error_msg);
        }
    }
}

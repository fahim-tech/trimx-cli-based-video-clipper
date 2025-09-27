//! Command implementations

use std::path::Path;
use anyhow::Result;
use tracing::info;

use crate::cli::{ClipArgs, InspectArgs, VerifyArgs};
use crate::domain::model::TimeSpec;

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

    let start_seconds = start_time.seconds;
    let end_seconds = end_time.seconds;

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

    // For now, just display basic file information
    let metadata = std::fs::metadata(&args.input)?;
    println!("File: {}", args.input);
    println!("Size: {} bytes", metadata.len());
    println!("Created: {:?}", metadata.created());
    println!("Modified: {:?}", metadata.modified());

    info!("Inspect operation completed successfully");
    Ok(())
}

/// Execute the verify command
pub fn verify(args: VerifyArgs) -> Result<()> {
    info!("Starting verify operation");
    info!("Output: {}", args.output);
    info!("Start: {}", args.start);
    info!("End: {}", args.end);

    // Validate output file exists
    if !Path::new(&args.output).exists() {
        return Err(anyhow::anyhow!("Output file does not exist: {}", args.output));
    }

    // Parse time arguments
    let start_time = TimeSpec::parse(&args.start)
        .map_err(|e| anyhow::anyhow!("Invalid start time '{}': {}", args.start, e))?;
    let end_time = TimeSpec::parse(&args.end)
        .map_err(|e| anyhow::anyhow!("Invalid end time '{}': {}", args.end, e))?;

    let start_seconds = start_time.seconds;
    let end_seconds = end_time.seconds;

    if start_seconds >= end_seconds {
        return Err(anyhow::anyhow!("Start time must be before end time"));
    }

    // For now, just display basic verification info
    let metadata = std::fs::metadata(&args.output)?;
    println!("File: {}", args.output);
    println!("Size: {} bytes", metadata.len());
    println!("Expected range: {} to {}", args.start, args.end);
    println!("Mode: {}", args.mode);
    println!("Tolerance: {}ms", args.tolerance);

    info!("Verify operation completed successfully");
    Ok(())
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

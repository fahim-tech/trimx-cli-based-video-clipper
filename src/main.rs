//! TrimX CLI Video Clipper
//!
//! A Windows-native command-line tool for precise video clipping with intelligent
//! lossless stream-copy and fallback re-encoding capabilities.
//!
//! # Features
//!
//! - Smart clipping strategy selection (auto/copy/reencode)
//! - GOP-spanning method for precise cuts
//! - Stream preservation (video, audio, subtitles)
//! - Multiple time format support
//! - Windows-optimized with long-path support
//!
//! # Usage
//!
//! ```bash
//! clipper.exe clip --in "video.mov" --start 00:01:00 --end 00:02:00
//! clipper.exe inspect --in "video.mov"
//! clipper.exe verify --in "clipped.mov" --start 00:01:00 --end 00:02:00
//! ```

use anyhow::Result;
use clap::Parser;
use tracing::{error, info};

mod cli;
mod domain;
mod app;
mod adapters;
mod ports;

use cli::{Cli, Commands, ClipArgs, InspectArgs, VerifyArgs};
use domain::model::*;
use domain::errors::*;
use app::*;
use adapters::*;

/// Main entry point for the TrimX CLI application
fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting TrimX CLI Video Clipper");

    // Parse command line arguments
    let cli = Cli::parse();

    // Execute the requested command
    match cli.command {
        Commands::Clip(args) => {
            info!("Executing clip command");
            execute_clip_command(args)?;
        }
        Commands::Inspect(args) => {
            info!("Executing inspect command");
            execute_inspect_command(args)?;
        }
        Commands::Verify(args) => {
            info!("Executing verify command");
            execute_verify_command(args)?;
        }
    }

    info!("TrimX CLI completed successfully");
    Ok(())
}

/// Execute clip command using hexagonal architecture
fn execute_clip_command(args: ClipArgs) -> Result<()> {
    // Parse time specifications
    let start_time = TimeSpec::parse(&args.start)
        .map_err(|e| anyhow::anyhow!("Invalid start time: {}", e))?;
    let end_time = TimeSpec::parse(&args.end)
        .map_err(|e| anyhow::anyhow!("Invalid end time: {}", e))?;
    
    // Parse clipping mode
    let mode = ClippingMode::parse(&args.mode)
        .map_err(|e| anyhow::anyhow!("Invalid clipping mode: {}", e))?;
    
    // Create cut range
    let cut_range = CutRange::new(start_time, end_time)
        .map_err(|e| anyhow::anyhow!("Invalid cut range: {}", e))?;
    
    // Create clip request
    let request = ClipRequest::new(
        args.input,
        args.output,
        cut_range,
        mode,
    ).map_err(|e| anyhow::anyhow!("Invalid clip request: {}", e))?;
    
    // TODO: Initialize adapters and execute through interactor
    info!("Clip command would be executed with request: {:?}", request);
    
    Ok(())
}

/// Execute inspect command using hexagonal architecture
fn execute_inspect_command(args: InspectArgs) -> Result<()> {
    // Create inspect request
    let request = InspectRequest::with_options(
        args.input,
        args.streams,
        args.metadata,
    );
    
    // TODO: Initialize adapters and execute through interactor
    info!("Inspect command would be executed with request: {:?}", request);
    
    Ok(())
}

/// Execute verify command using hexagonal architecture
fn execute_verify_command(args: VerifyArgs) -> Result<()> {
    // Parse time specifications
    let start_time = TimeSpec::parse(&args.start)
        .map_err(|e| anyhow::anyhow!("Invalid start time: {}", e))?;
    let end_time = TimeSpec::parse(&args.end)
        .map_err(|e| anyhow::anyhow!("Invalid end time: {}", e))?;
    
    // Parse clipping mode
    let mode = ClippingMode::parse(&args.mode)
        .map_err(|e| anyhow::anyhow!("Invalid clipping mode: {}", e))?;
    
    // Create cut range
    let cut_range = CutRange::new(start_time, end_time)
        .map_err(|e| anyhow::anyhow!("Invalid cut range: {}", e))?;
    
    // Create verify request
    let request = VerifyRequest::with_tolerance(
        args.output,
        cut_range,
        mode,
        args.tolerance,
    );
    
    // TODO: Initialize adapters and execute through interactor
    info!("Verify command would be executed with request: {:?}", request);
    
    Ok(())
}

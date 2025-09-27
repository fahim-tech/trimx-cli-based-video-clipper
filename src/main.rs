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
use adapters::*;
use crate::app::clip_interactor::{ClipRequest, ClipInteractor};
use crate::app::inspect_interactor::InspectInteractor;
use crate::app::verify_interactor::{VerifyRequest, VerifyInteractor};

/// Main entry point for the TrimX CLI application
fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting TrimX CLI Video Clipper");

    // Parse command line arguments
    let cli = Cli::parse();

    // Execute the requested command with comprehensive error handling
    let result = match cli.command {
        Commands::Clip(args) => {
            info!("Executing clip command");
            execute_clip_command(args)
        }
        Commands::Inspect(args) => {
            info!("Executing inspect command");
            execute_inspect_command(args)
        }
        Commands::Verify(args) => {
            info!("Executing verify command");
            execute_verify_command(args)
        }
    };

    match result {
        Ok(()) => {
            info!("TrimX CLI completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("TrimX CLI failed: {}", e);
            
            // Provide helpful error messages based on error type
            if let Some(domain_error) = e.downcast_ref::<DomainError>() {
                match domain_error {
                    DomainError::BadArgs(msg) => {
                        error!("Invalid arguments: {}", msg);
                        error!("Use --help for usage information");
                        std::process::exit(2);
                    }
                    DomainError::FileNotFound(msg) => {
                        error!("File not found: {}", msg);
                        error!("Please check the file path and try again");
                        std::process::exit(3);
                    }
                    DomainError::ProbeFail(msg) => {
                        error!("Failed to analyze media file: {}", msg);
                        error!("The file may be corrupted or in an unsupported format");
                        std::process::exit(4);
                    }
                    DomainError::ProcessingError(msg) => {
                        error!("Video processing failed: {}", msg);
                        error!("Please check the input file and try again");
                        std::process::exit(5);
                    }
                    _ => {
                        error!("Operation failed: {}", domain_error);
                        std::process::exit(1);
                    }
                }
            } else {
                error!("Unexpected error: {}", e);
                std::process::exit(1);
            }
        }
    }
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
    
    // Initialize adapters and execute through interactor
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let probe_adapter = Box::new(LibavProbeAdapter::new()?);
        let exec_adapter = Box::new(LibavExecutionAdapter::new()?);
        let fs_adapter = Box::new(FsWindowsAdapter::new()?);
        let config_adapter = Box::new(TomlConfigAdapter::new()?);
        let log_adapter = Box::new(TracingLogAdapter::new()?);
        
        let interactor = ClipInteractor::new(
            probe_adapter,
            exec_adapter,
            fs_adapter,
            config_adapter,
            log_adapter,
        );
        
        println!("Starting video clipping...");
        println!("Input: {}", request.input_file);
        println!("Range: {} to {}", request.cut_range.start, request.cut_range.end);
        println!("Mode: {:?}", request.mode);
        
        let result = interactor.execute(request).await?;
        
        if result.success {
            println!("\nVideo clipping completed successfully!");
            println!("Output file: {}", result.output_file);
            println!("Duration: {}", result.duration);
            println!("Mode used: {:?}", result.mode_used);
            println!("Processing time: {:.2}s", result.processing_time.as_secs_f64());
            
            // Show any warnings
            for warning in &result.warnings {
                println!("Warning: {}", warning);
            }
            
            println!("\nReady to use: {}", result.output_file);
        } else {
            println!("\nVideo clipping failed!");
            for warning in &result.warnings {
                println!("Error: {}", warning);
            }
            return Err(anyhow::anyhow!("Clip operation failed"));
        }
        
        Ok(())
    })?;
    
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
    
    // Initialize adapters and execute through interactor
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let probe_adapter = Box::new(LibavProbeAdapter::new()?);
        let fs_adapter = Box::new(FsWindowsAdapter::new()?);
        let log_adapter = Box::new(TracingLogAdapter::new()?);
        
        let interactor = InspectInteractor::new(
            probe_adapter,
            fs_adapter,
            log_adapter,
        );
        
        println!("Analyzing video file...");
        
        let input_file = request.input_file.clone();
        let result = interactor.execute(request).await?;
        
        if result.success {
            println!("\nVideo analysis completed successfully!");
            println!("File: {}", input_file);
            println!("Format: {}", result.media_info.container);
            println!("Duration: {}", result.media_info.duration);
            println!("File size: {} bytes", result.media_info.file_size);
            println!("Streams: {} total", result.media_info.total_streams());
            
            for (i, video_stream) in result.media_info.video_streams.iter().enumerate() {
                println!("Video stream {}: {}x{} @ {:.2} fps", 
                    i, video_stream.width, video_stream.height, video_stream.frame_rate);
            }
            
            for (i, audio_stream) in result.media_info.audio_streams.iter().enumerate() {
                println!("Audio stream {}: {} Hz, {} channels", 
                    i, audio_stream.sample_rate, audio_stream.channels);
            }
        } else {
            println!("\nVideo analysis failed!");
            if let Some(error_msg) = &result.error_message {
                println!("Error: {}", error_msg);
            }
        }
        
        Ok::<(), DomainError>(())
    })?;
    
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
    
    // Initialize adapters and execute through interactor
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let probe_adapter = Box::new(LibavProbeAdapter::new()?);
        let fs_adapter = Box::new(FsWindowsAdapter::new()?);
        let log_adapter = Box::new(TracingLogAdapter::new()?);
        
        let interactor = VerifyInteractor::new(
            probe_adapter,
            fs_adapter,
            log_adapter,
        );
        
        let result = interactor.execute(request).await?;
        
        if result.verification_result.success {
            info!("Verify operation completed successfully");
            info!("Overall score: {:.1}%", result.verification_result.overall_score);
            
            for check in &result.verification_result.checks {
                let status = if check.success { "✓" } else { "✗" };
                info!("{} {}: {}", status, check.check_type, check.details);
            }
        } else {
            error!("Verify operation failed");
            error!("Error: {}", result.verification_result.error_message);
            
            for check in &result.verification_result.checks {
                if !check.success {
                    error!("Failed check: {} - {}", check.check_type, check.error_message);
                }
            }
        }
        
        Ok::<(), DomainError>(())
    })?;
    
    Ok(())
}

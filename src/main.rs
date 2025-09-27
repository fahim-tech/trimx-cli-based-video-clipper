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

mod adapters;
mod app;
mod cli;
mod config_initialization;
mod domain;
mod engine;
mod error;
mod output;
mod planner;
mod ports;
mod probe;
mod streams;
mod utils;

use crate::app::container::{AppContainer, DefaultAppContainer};
use crate::cli::{Cli, Commands};
use crate::domain::errors::DomainError;
use crate::domain::model::{CutRange, TimeSpec};

/// Main entry point for the TrimX CLI application
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting TrimX CLI Video Clipper");

    // Parse command line arguments
    let cli = Cli::parse();

    // Initialize dependency container
    let container = DefaultAppContainer::new()
        .map_err(|e| anyhow::anyhow!("Failed to initialize application: {}", e))?;

    // Initialize configuration hierarchy (CLI → Env → File → Defaults)
    config_initialization::initialize_configuration_hierarchy(&container, &cli).await?;

    // Execute the requested command using proper application layer
    let result = match cli.command {
        Commands::Clip(args) => {
            info!("Executing clip command");
            execute_clip_command(&container, args)
        }
        Commands::Inspect(args) => {
            info!("Executing inspect command");
            execute_inspect_command(&container, args)
        }
        Commands::Verify(args) => {
            info!("Executing verify command");
            execute_verify_command(&container, args)
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
                let exit_code = match domain_error {
                    DomainError::BadArgs(msg) => {
                        error!("Invalid arguments: {}", msg);
                        error!("Use --help for usage information");
                        2
                    }
                    DomainError::FileNotFound(msg) => {
                        error!("File not found: {}", msg);
                        error!("Please check the file path and try again");
                        3
                    }
                    DomainError::ProbeFail(msg) => {
                        error!("Failed to analyze media file: {}", msg);
                        error!("The file may be corrupted or in an unsupported format");
                        4
                    }
                    DomainError::ProcessingError(msg) => {
                        error!("Video processing failed: {}", msg);
                        error!("Please check the input file and try again");
                        5
                    }
                    DomainError::InternalError(msg) => {
                        error!("Internal error: {}", msg);
                        6
                    }
                    DomainError::ConfigError(msg) => {
                        error!("Configuration error: {}", msg);
                        7
                    }
                    DomainError::ValidationError(msg) => {
                        error!("Validation error: {}", msg);
                        2
                    }
                    DomainError::InvalidFormat(msg) => {
                        error!("Invalid format: {}", msg);
                        3
                    }
                    DomainError::UnsupportedCodec(msg) => {
                        error!("Unsupported codec: {}", msg);
                        4
                    }
                    DomainError::InvalidTimeRange(msg) => {
                        error!("Invalid time range: {}", msg);
                        2
                    }
                    DomainError::PermissionDenied(msg) => {
                        error!("Permission denied: {}", msg);
                        5
                    }
                    DomainError::ResourceUnavailable(msg) => {
                        error!("Resource unavailable: {}", msg);
                        6
                    }
                    DomainError::ValidationFailed(msg) => {
                        error!("Validation failed: {}", msg);
                        2
                    }
                    DomainError::FsFail(msg) => {
                        error!("File system error: {}", msg);
                        7
                    }
                    DomainError::NotImplemented => {
                        error!("Feature not implemented yet");
                        8
                    }
                    DomainError::OutOfRange(msg) => {
                        error!("Out of range: {}", msg);
                        2
                    }
                };
                std::process::exit(exit_code);
            } else {
                error!("Unexpected error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

/// Execute clip command using application layer
fn execute_clip_command(container: &DefaultAppContainer, args: crate::cli::ClipArgs) -> Result<()> {
    let interactor = container.clip_interactor();

    // Convert CLI args to domain model
    let request = crate::app::clip_interactor::ClipRequest {
        input_path: args.input.clone(),
        input_file: args.input,
        output_path: args.output.clone(),
        output_file: args.output,
        start_time: args.start,
        end_time: args.end,
        cut_range: CutRange::new(TimeSpec::from_seconds(0.0), TimeSpec::from_seconds(0.001))
            .unwrap(),
        mode: crate::domain::model::ClippingMode::parse(&args.mode)?,
        quality: args.quality,
        quality_settings: None,
        overwrite: args.overwrite,
        threads: args.threads,
    };

    match tokio::runtime::Runtime::new() {
        Ok(rt) => rt.block_on(async {
            let _result = interactor
                .clip_video(request)
                .await
                .map_err(|e| anyhow::anyhow!("Clip operation failed: {}", e))?;
            Ok(())
        }),
        Err(e) => Err(anyhow::anyhow!("Failed to create async runtime: {}", e)),
    }
}

/// Execute inspect command using application layer
fn execute_inspect_command(
    container: &DefaultAppContainer,
    args: crate::cli::InspectArgs,
) -> Result<()> {
    let interactor = container.inspect_interactor();

    let request = crate::app::inspect_interactor::InspectRequest {
        input_path: args.input.clone(),
        input: args.input,
        format: args.format,
        show_streams: args.show_streams,
        show_keyframes: args.show_keyframes,
    };

    match tokio::runtime::Runtime::new() {
        Ok(rt) => rt.block_on(async {
            let result = interactor
                .inspect_file(request)
                .await
                .map_err(|e| anyhow::anyhow!("Inspect operation failed: {}", e))?;

            // Display results
            println!("{}", result.summary);
            Ok(())
        }),
        Err(e) => Err(anyhow::anyhow!("Failed to create async runtime: {}", e)),
    }
}

/// Execute verify command using application layer
fn execute_verify_command(
    container: &DefaultAppContainer,
    args: crate::cli::VerifyArgs,
) -> Result<()> {
    let interactor = container.verify_interactor();

    let request = crate::app::verify_interactor::VerifyRequest {
        output_path: args.output.clone(),
        output_file: args.output,
        expected_start: args.start,
        expected_end: args.end,
        expected_range: CutRange::new(TimeSpec::from_seconds(0.0), TimeSpec::from_seconds(0.001))
            .unwrap(),
        mode: crate::domain::model::ClippingMode::parse(&args.mode)?,
        expected_mode: crate::domain::model::ClippingMode::parse(&args.mode)?,
        tolerance: args.tolerance,
        tolerance_ms: args.tolerance,
    };

    match tokio::runtime::Runtime::new() {
        Ok(rt) => rt.block_on(async {
            let result = interactor
                .verify_output(request)
                .await
                .map_err(|e| anyhow::anyhow!("Verify operation failed: {}", e))?;

            // Display results
            println!(
                "Verification Result: {}",
                if result.is_valid { "PASS" } else { "FAIL" }
            );
            if let Some(ref message) = result.message {
                println!("Details: {}", message);
            }

            if !result.is_valid {
                std::process::exit(1);
            }

            Ok(())
        }),
        Err(e) => Err(anyhow::anyhow!("Failed to create async runtime: {}", e)),
    }
}

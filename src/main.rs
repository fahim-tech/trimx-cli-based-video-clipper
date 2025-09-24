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
mod error;
mod probe;
mod planner;
mod engine;
mod streams;
mod output;
mod utils;

use cli::{Cli, Commands};
use error::TrimXError;

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
            cli::commands::clip(args)?;
        }
        Commands::Inspect(args) => {
            info!("Executing inspect command");
            cli::commands::inspect(args)?;
        }
        Commands::Verify(args) => {
            info!("Executing verify command");
            cli::commands::verify(args)?;
        }
    }

    info!("TrimX CLI completed successfully");
    Ok(())
}

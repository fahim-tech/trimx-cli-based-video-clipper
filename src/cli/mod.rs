//! CLI module for TrimX
//!
//! This module handles command-line argument parsing and command execution.

use clap::{Parser, Subcommand, Args};

/// TrimX CLI Video Clipper
///
/// A Windows-native command-line tool for precise video clipping with intelligent
/// lossless stream-copy and fallback re-encoding capabilities.
#[derive(Parser)]
#[command(name = "clipper")]
#[command(about = "TrimX CLI Video Clipper - Precise video clipping made simple")]
#[command(version)]
#[command(long_about = None)]
pub struct Cli {
    /// Logging level
    #[arg(long, default_value = "info", global = true)]
    pub log_level: String,

    /// Overwrite behavior
    #[arg(long, default_value = "prompt", global = true)]
    pub overwrite: String,

    /// The command to execute
    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands
#[derive(Subcommand)]
pub enum Commands {
    /// Extract a segment from a video file
    Clip(ClipArgs),
    /// Inspect video file information
    Inspect(InspectArgs),
    /// Verify a clipped segment
    Verify(VerifyArgs),
}

/// Arguments for clip command
#[derive(Args)]
pub struct ClipArgs {
    /// Input video file
    #[arg(short, long)]
    pub input: String,
    
    /// Output video file
    #[arg(short, long)]
    pub output: String,
    
    /// Start time (HH:MM:SS or seconds)
    #[arg(short, long)]
    pub start: String,
    
    /// End time (HH:MM:SS or seconds)
    #[arg(short, long)]
    pub end: String,
    
    /// Clipping mode (auto, copy, reencode, hybrid)
    #[arg(short, long, default_value = "auto")]
    pub mode: String,
}

/// Arguments for inspect command
#[derive(Args)]
pub struct InspectArgs {
    /// Input video file
    #[arg(short, long)]
    pub input: String,
    
    /// Include detailed stream information
    #[arg(long, default_value = "true")]
    pub streams: bool,
    
    /// Include metadata
    #[arg(long, default_value = "true")]
    pub metadata: bool,
}

/// Arguments for verify command
#[derive(Args)]
pub struct VerifyArgs {
    /// Output video file to verify
    #[arg(short, long)]
    pub output: String,
    
    /// Expected start time
    #[arg(short, long)]
    pub start: String,
    
    /// Expected end time
    #[arg(short, long)]
    pub end: String,
    
    /// Expected mode
    #[arg(short, long)]
    pub mode: String,
    
    /// Tolerance in milliseconds
    #[arg(short, long, default_value = "100")]
    pub tolerance: u32,
}

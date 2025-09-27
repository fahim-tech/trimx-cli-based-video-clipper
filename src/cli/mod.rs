//! CLI module for TrimX
//!
//! This module handles command-line argument parsing and command execution.

pub mod commands;

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
    #[arg(value_name = "FILE")]
    pub input: String,
    
    /// Start time (HH:MM:SS or seconds)
    #[arg(short, long)]
    pub start: String,
    
    /// End time (HH:MM:SS or seconds)
    #[arg(short, long)]
    pub end: String,
    
    /// Output video file (optional - auto-generated if not provided)
    #[arg(short, long)]
    pub output: Option<String>,
    
    /// Clipping mode (auto, copy, reencode, hybrid)
    #[arg(short, long, default_value = "auto")]
    pub mode: String,

    /// Quality setting (0-51)
    #[arg(long)]
    pub quality: Option<u8>,

    /// Overwrite output file if it exists
    #[arg(long)]
    pub overwrite: bool,

    /// Number of threads to use
    #[arg(long)]
    pub threads: Option<usize>,
}

/// Arguments for inspect command
#[derive(Args)]
pub struct InspectArgs {
    /// Input video file
    #[arg(value_name = "FILE")]
    pub input: String,
    
    /// Include detailed stream information
    #[arg(long, default_value = "true")]
    pub streams: bool,
    
    /// Include metadata
    #[arg(long, default_value = "true")]
    pub metadata: bool,

    /// Output format (json, yaml, text)
    #[arg(long, default_value = "text")]
    pub format: String,

    /// Show stream information
    #[arg(long)]
    pub show_streams: bool,

    /// Show keyframe information
    #[arg(long)]
    pub show_keyframes: bool,
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

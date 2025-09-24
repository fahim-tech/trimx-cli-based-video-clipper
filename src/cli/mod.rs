//! CLI module for TrimX
//!
//! This module handles command-line argument parsing and command execution.

use anyhow::Result;
use clap::{Parser, Subcommand};

pub mod args;
pub mod commands;

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
    Clip(args::ClipArgs),
    /// Inspect video file information
    Inspect(args::InspectArgs),
    /// Verify a clipped segment
    Verify(args::VerifyArgs),
}

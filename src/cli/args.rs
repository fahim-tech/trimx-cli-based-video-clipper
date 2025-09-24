//! Command-line argument definitions

use clap::Args;

/// Arguments for the clip command
#[derive(Args, Debug)]
pub struct ClipArgs {
    /// Input video file path
    #[arg(short, long)]
    pub input: String,

    /// Start time (HH:MM:SS.ms, MM:SS.ms, or seconds)
    #[arg(short, long)]
    pub start: String,

    /// End time (HH:MM:SS.ms, MM:SS.ms, or seconds)
    #[arg(short, long)]
    pub end: String,

    /// Output file path (default: auto-generated)
    #[arg(short, long)]
    pub output: Option<String>,

    /// Clipping strategy
    #[arg(long, default_value = "auto")]
    pub mode: String,

    /// Remove audio streams
    #[arg(long)]
    pub no_audio: bool,

    /// Remove subtitle streams
    #[arg(long)]
    pub no_subs: bool,

    /// Output container format
    #[arg(long, default_value = "same")]
    pub container: String,

    /// Video codec
    #[arg(long, default_value = "h264")]
    pub codec: String,

    /// Constant Rate Factor (0-51)
    #[arg(long, default_value = "18")]
    pub crf: u8,

    /// Encoding preset
    #[arg(long, default_value = "medium")]
    pub preset: String,
}

/// Arguments for the inspect command
#[derive(Args, Debug)]
pub struct InspectArgs {
    /// Input video file path
    #[arg(short, long)]
    pub input: String,

    /// Output in JSON format
    #[arg(long)]
    pub json: bool,
}

/// Arguments for the verify command
#[derive(Args, Debug)]
pub struct VerifyArgs {
    /// Input video file path
    #[arg(short, long)]
    pub input: String,

    /// Expected start time
    #[arg(short, long)]
    pub start: String,

    /// Expected end time
    #[arg(short, long)]
    pub end: String,
}

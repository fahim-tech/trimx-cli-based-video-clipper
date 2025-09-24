//! Output file writing and verification module

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod writer;
pub mod verifier;

/// Output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Output file path
    pub path: String,
    /// Container format
    pub container: String,
    /// Overwrite policy
    pub overwrite: OverwritePolicy,
    /// Enable faststart for MP4
    pub faststart: bool,
}

/// Overwrite policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverwritePolicy {
    /// Prompt user before overwriting
    Prompt,
    /// Always overwrite
    Always,
    /// Never overwrite
    Never,
}

/// Output verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Verification passed
    pub success: bool,
    /// Expected start time
    pub expected_start: f64,
    /// Actual start time
    pub actual_start: f64,
    /// Expected end time
    pub expected_end: f64,
    /// Actual end time
    pub actual_end: f64,
    /// Duration accuracy
    pub duration_accuracy: f64,
    /// Stream count verification
    pub stream_count_match: bool,
    /// Error message if verification failed
    pub error: Option<String>,
}

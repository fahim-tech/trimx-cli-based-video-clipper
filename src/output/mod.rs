//! Output file writing and verification module

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
    /// Overall verification score
    pub overall_score: f64,
    /// Individual verification checks
    pub checks: Vec<VerificationCheck>,
    /// Error message
    pub error_message: Option<String>,
}

/// Individual verification check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCheck {
    /// Type of check
    pub check_type: String,
    /// Check details
    pub details: String,
    /// Check passed
    pub success: bool,
    /// Check score
    pub score: f64,
    /// Check weight
    pub weight: f64,
    /// Error message if check failed
    pub error_message: Option<String>,
}

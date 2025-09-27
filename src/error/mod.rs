//! Error handling module for TrimX

use thiserror::Error;

/// Main error type for TrimX operations
#[derive(Error, Debug)]
pub enum TrimXError {
    /// Input file not found or inaccessible
    #[error("Input file not found: {path}")]
    InputFileNotFound { path: String },

    /// Invalid time format
    #[error("Invalid time format: {time}. Expected HH:MM:SS.ms, MM:SS.ms, or seconds")]
    InvalidTimeFormat { time: String },

    /// Time range validation error
    #[error("Invalid time range: start ({start}) must be less than end ({end})")]
    InvalidTimeRange { start: String, end: String },

    /// FFmpeg initialization error
    #[error("Failed to initialize FFmpeg: {message}")]
    FFmpegInitError { message: String },

    /// Media probe error
    #[error("Failed to probe media file: {message}")]
    ProbeError { message: String },

    /// Clipping operation error
    #[error("Clipping operation failed: {message}")]
    ClippingError { message: String },

    /// Output file write error
    #[error("Failed to write output file: {message}")]
    OutputError { message: String },

    /// Stream processing error
    #[error("Stream processing error: {message}")]
    StreamError { message: String },

    /// Verification error
    #[error("Verification failed: {message}")]
    VerificationError { message: String },

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// FFmpeg error
    #[error("FFmpeg error: {0}")]
    FFmpegError(#[from] ffmpeg_next::Error),
}

/// Result type alias for TrimX operations
pub type TrimXResult<T> = std::result::Result<T, TrimXError>;

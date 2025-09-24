// Domain errors - Error types for the domain layer

use std::fmt;

/// Domain-specific error types
#[derive(Debug, Clone)]
pub enum DomainError {
    /// Invalid arguments provided
    BadArgs(String),
    /// File not found
    FileNotFound(String),
    /// Invalid file format
    InvalidFormat(String),
    /// Codec not supported
    UnsupportedCodec(String),
    /// Invalid time range
    InvalidTimeRange(String),
    /// Insufficient permissions
    PermissionDenied(String),
    /// Resource not available
    ResourceUnavailable(String),
    /// Validation failed
    ValidationFailed(String),
    /// Processing error
    ProcessingError(String),
    /// Internal error
    InternalError(String),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::BadArgs(msg) => write!(f, "Bad arguments: {}", msg),
            DomainError::FileNotFound(msg) => write!(f, "File not found: {}", msg),
            DomainError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            DomainError::UnsupportedCodec(msg) => write!(f, "Unsupported codec: {}", msg),
            DomainError::InvalidTimeRange(msg) => write!(f, "Invalid time range: {}", msg),
            DomainError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            DomainError::ResourceUnavailable(msg) => write!(f, "Resource unavailable: {}", msg),
            DomainError::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
            DomainError::ProcessingError(msg) => write!(f, "Processing error: {}", msg),
            DomainError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for DomainError {}

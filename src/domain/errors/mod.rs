// Domain errors - Domain-specific error types

use std::fmt;

/// Domain-level error types
#[derive(Debug, Clone)]
pub enum DomainError {
    /// Invalid command-line arguments or parameters
    BadArgs(String),
    /// Cut range exceeds media duration or is invalid
    OutOfRange(String),
    /// Media file inspection failed
    ProbeFail(String),
    /// Cannot create valid execution plan
    PlanUnsupported(String),
    /// Execution engine failed
    ExecFail(String),
    /// File system operations failed
    FsFail(String),
    /// Output verification failed
    VerifyFail(String),
    /// Feature not yet implemented
    NotImplemented,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::BadArgs(msg) => write!(f, "Invalid arguments: {}", msg),
            DomainError::OutOfRange(msg) => write!(f, "Range error: {}", msg),
            DomainError::ProbeFail(msg) => write!(f, "Probe failed: {}", msg),
            DomainError::PlanUnsupported(msg) => write!(f, "Plan unsupported: {}", msg),
            DomainError::ExecFail(msg) => write!(f, "Execution failed: {}", msg),
            DomainError::FsFail(msg) => write!(f, "File system error: {}", msg),
            DomainError::VerifyFail(msg) => write!(f, "Verification failed: {}", msg),
            DomainError::NotImplemented => write!(f, "Feature not implemented"),
        }
    }
}

impl std::error::Error for DomainError {}

/// Exit codes for automation
#[derive(Debug, Clone, Copy)]
pub enum ExitCode {
    Success = 0,
    InvalidArgs = 2,
    ProbeFailure = 3,
    ExecutionFailure = 4,
    FilesystemFailure = 5,
}

impl From<DomainError> for ExitCode {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::BadArgs(_) => ExitCode::InvalidArgs,
            DomainError::OutOfRange(_) => ExitCode::InvalidArgs,
            DomainError::ProbeFail(_) => ExitCode::ProbeFailure,
            DomainError::PlanUnsupported(_) => ExitCode::ExecutionFailure,
            DomainError::ExecFail(_) => ExitCode::ExecutionFailure,
            DomainError::FsFail(_) => ExitCode::FilesystemFailure,
            DomainError::VerifyFail(_) => ExitCode::ExecutionFailure,
            DomainError::NotImplemented => ExitCode::ExecutionFailure,
        }
    }
}

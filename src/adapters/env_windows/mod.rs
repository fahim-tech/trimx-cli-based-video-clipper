//! Windows environment adapter
//!
//! This module provides Windows-specific environment variable handling.

use crate::domain::errors::DomainError;

/// Windows environment adapter
pub struct EnvWindowsAdapter;

impl EnvWindowsAdapter {
    /// Create new Windows environment adapter
    pub fn new() -> Result<Self, DomainError> {
        Ok(Self)
    }

    /// Get environment variable
    pub fn get_env(&self, key: &str) -> Result<Option<String>, DomainError> {
        match std::env::var(key) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(e) => Err(DomainError::InternalError(format!(
                "Failed to get environment variable {}: {}",
                key, e
            ))),
        }
    }

    /// Set environment variable
    pub fn set_env(&self, key: &str, value: &str) -> Result<(), DomainError> {
        std::env::set_var(key, value);
        Ok(())
    }
}

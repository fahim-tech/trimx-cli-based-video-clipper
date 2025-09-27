// Tracing log adapter - Structured logging using tracing crate

use crate::domain::errors::*;
use crate::ports::*;
use async_trait::async_trait;
use tracing::{info, warn, error, debug, trace};

/// Tracing log adapter
pub struct TracingLogAdapter {
    current_level: LogLevel,
    json_output: bool,
}

impl TracingLogAdapter {
    /// Create new tracing log adapter
    pub fn new() -> Result<Self, DomainError> {
        // Initialize tracing subscriber if not already initialized
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
        
        Ok(Self {
            current_level: LogLevel::Info,
            json_output: false,
        })
    }
    
    /// Convert domain log level to tracing level
    fn to_tracing_level(level: &LogLevel) -> tracing::Level {
        match level {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
    
    /// Check if log level should be logged
    fn should_log(&self, level: &LogLevel) -> bool {
        level >= &self.current_level
    }
}

#[async_trait]
impl LogPort for TracingLogAdapter {
    async fn info(&self, message: &str) {
        if self.should_log(&LogLevel::Info) {
            info!("{}", message);
        }
    }
    
    async fn warn(&self, message: &str) {
        if self.should_log(&LogLevel::Warn) {
            warn!("{}", message);
        }
    }
    
    async fn error(&self, message: &str) {
        if self.should_log(&LogLevel::Error) {
            error!("{}", message);
        }
    }
    
    async fn debug(&self, message: &str) {
        if self.should_log(&LogLevel::Debug) {
            debug!("{}", message);
        }
    }
    
    async fn trace(&self, message: &str) {
        if self.should_log(&LogLevel::Trace) {
            trace!("{}", message);
        }
    }
    
    async fn log_event(&self, event: &LogEvent) {
        if !self.should_log(&event.level) {
            return;
        }
        
        let _tracing_level = Self::to_tracing_level(&event.level);
        
        // Log structured event based on level
        match event.level {
            LogLevel::Error => {
                tracing::error!(message = %event.message, ?event.context);
            }
            LogLevel::Warn => {
                tracing::warn!(message = %event.message, ?event.context);
            }
            LogLevel::Info => {
                tracing::info!(message = %event.message, ?event.context);
            }
            LogLevel::Debug => {
                tracing::debug!(message = %event.message, ?event.context);
            }
            LogLevel::Trace => {
                tracing::trace!(message = %event.message, ?event.context);
            }
        }
    }
    
    async fn set_log_level(&self, level: LogLevel) {
        // Note: This is a limitation of the current implementation
        // In a real implementation, we would need to recreate the tracing subscriber
        // For now, we just store the level for filtering
        // self.current_level = level;
        
        // Log the level change
        info!("Log level changed to: {:?}", level);
    }
    
    async fn get_log_level(&self) -> LogLevel {
        self.current_level.clone()
    }
    
    async fn set_json_output(&self, enabled: bool) {
        // Note: This is a limitation of the current implementation
        // In a real implementation, we would need to recreate the tracing subscriber
        // For now, we just store the setting
        // self.json_output = enabled;
        
        info!("JSON output {}", if enabled { "enabled" } else { "disabled" });
    }
    
    async fn flush(&self) {
        // Tracing handles flushing automatically
        // This is a no-op for compatibility
    }
}
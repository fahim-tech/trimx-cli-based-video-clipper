//! Advanced logging configuration and output formatting

use serde::{Deserialize, Serialize};
use std::io::{self, Write};

/// Logging configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Global log level
    pub level: LogLevel,
    /// Output format
    pub format: LogFormat,
    /// Output destination
    pub output: LogOutput,
    /// Include timestamps
    pub timestamps: bool,
    /// Include target module information
    pub target: bool,
    /// Include thread information
    pub thread: bool,
    /// Include line numbers
    pub line_numbers: bool,
    /// Use colored output (if supported)
    pub colored: bool,
    /// JSON output formatting options
    pub json_options: JsonOptions,
    /// Progress reporting configuration
    pub progress: ProgressConfig,
}

/// Log level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    /// Error messages only
    Error,
    /// Warnings and errors
    Warn,
    /// General information
    Info,
    /// Debug information
    Debug,
    /// Very verbose debug information
    Trace,
}

/// Log output format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    /// Human-readable text format
    Pretty,
    /// Compact text format
    Compact,
    /// JSON format for structured logging
    Json,
    /// Custom format string
    Custom(String),
}

/// Log output destination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogOutput {
    /// Standard output
    Stdout,
    /// Standard error
    Stderr,
    /// File output
    File(String),
    /// Both stdout and file
    Both(String),
}

/// JSON formatting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonOptions {
    /// Pretty-print JSON output
    pub pretty: bool,
    /// Include span information
    pub spans: bool,
    /// Include field information
    pub fields: bool,
    /// Custom field mapping
    pub field_map: std::collections::HashMap<String, String>,
}

/// Progress reporting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressConfig {
    /// Enable progress bars
    pub enabled: bool,
    /// Progress bar style
    pub style: ProgressStyle,
    /// Update interval (milliseconds)
    pub update_interval: u64,
    /// Show ETA (estimated time of arrival)
    pub show_eta: bool,
    /// Show throughput information
    pub show_throughput: bool,
}

/// Progress bar style
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressStyle {
    /// Simple ASCII progress bar
    Simple,
    /// Unicode progress bar with emojis
    Unicode,
    /// Minimal progress indicator
    Minimal,
    /// Custom progress template
    Custom(String),
}

/// Logging system manager
pub struct LoggingSystem {
    config: LoggingConfig,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Pretty,
            output: LogOutput::Stderr,
            timestamps: true,
            target: false,
            thread: false,
            line_numbers: false,
            colored: true,
            json_options: JsonOptions {
                pretty: false,
                spans: true,
                fields: true,
                field_map: std::collections::HashMap::new(),
            },
            progress: ProgressConfig {
                enabled: true,
                style: ProgressStyle::Unicode,
                update_interval: 100,
                show_eta: true,
                show_throughput: true,
            },
        }
    }
}

impl LoggingSystem {
    /// Create a new logging system with configuration
    pub fn new(config: LoggingConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(LoggingConfig::default())
    }
}

impl Default for LoggingSystem {
    fn default() -> Self {
        Self::new(LoggingConfig::default())
    }
}

impl LoggingSystem {
    /// Initialize the logging system (simplified)
    pub fn initialize(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Simple initialization using tracing_subscriber's basic features
        let filter = match self.config.level {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        };

        std::env::set_var("RUST_LOG", filter);
        tracing_subscriber::fmt::init();

        tracing::info!(
            "Logging system initialized with level: {:?}",
            self.config.level
        );
        Ok(())
    }

    /// Create progress reporter
    pub fn create_progress_reporter(&self) -> ProgressReporter {
        ProgressReporter::new(self.config.progress.clone())
    }

    /// Log system information
    pub fn log_system_info(&self) {
        tracing::info!("=== TrimX Video Clipper ===");
        tracing::info!("Version: {}", env!("CARGO_PKG_VERSION"));
        tracing::info!(
            "Build: {} {}",
            option_env!("BUILD_DATE").unwrap_or("unknown"),
            option_env!("GIT_HASH").unwrap_or("unknown")
        );

        #[cfg(target_os = "macos")]
        tracing::info!("Platform: macOS");
        #[cfg(target_os = "linux")]
        tracing::info!("Platform: Linux");
        #[cfg(target_os = "windows")]
        tracing::info!("Platform: Windows");

        tracing::info!("Logging level: {:?}", self.config.level);
        tracing::info!("Output format: {:?}", self.config.format);
    }
}

/// Multi-writer that writes to multiple destinations
#[allow(dead_code)]
struct MultiWriter {
    writers: Vec<Box<dyn Write + Send + Sync>>,
}

impl MultiWriter {
    #[allow(dead_code)]
    fn new(writers: Vec<Box<dyn Write + Send + Sync>>) -> Self {
        Self { writers }
    }
}

impl Write for MultiWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for writer in &mut self.writers {
            writer.write_all(buf)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        for writer in &mut self.writers {
            writer.flush()?;
        }
        Ok(())
    }
}

/// Progress reporter for long-running operations
pub struct ProgressReporter {
    config: ProgressConfig,
    current_operation: Option<String>,
    start_time: Option<std::time::Instant>,
}

impl ProgressReporter {
    /// Create a new progress reporter
    pub fn new(config: ProgressConfig) -> Self {
        Self {
            config,
            current_operation: None,
            start_time: None,
        }
    }

    /// Start a new operation
    pub fn start_operation(&mut self, operation: String) {
        self.current_operation = Some(operation.clone());
        self.start_time = Some(std::time::Instant::now());

        if self.config.enabled {
            tracing::info!("🚀 Starting: {}", operation);
        }
    }

    /// Update progress
    pub fn update_progress(&self, progress: f64, description: Option<String>) {
        if !self.config.enabled {
            return;
        }

        let progress_pct = (progress * 100.0).clamp(0.0, 100.0);

        let mut message = match &self.config.style {
            ProgressStyle::Simple => {
                format!(
                    "[{:>3.0}%] {}",
                    progress_pct,
                    description.unwrap_or_else(|| "Processing...".to_string())
                )
            }
            ProgressStyle::Unicode => {
                let bar_length = 20;
                let filled = (progress * bar_length as f64) as usize;
                let bar = "█".repeat(filled) + &"░".repeat(bar_length - filled);
                format!(
                    "🔄 [{}] {:>3.0}% {}",
                    bar,
                    progress_pct,
                    description.unwrap_or_else(|| "Processing...".to_string())
                )
            }
            ProgressStyle::Minimal => {
                format!("{:>3.0}%", progress_pct)
            }
            ProgressStyle::Custom(template) => template
                .replace("{percent}", &format!("{:.0}", progress_pct))
                .replace("{description}", &description.unwrap_or_default()),
        };

        // Add ETA if enabled and we have timing information
        if self.config.show_eta && progress > 0.0 {
            if let Some(start_time) = self.start_time {
                let elapsed = start_time.elapsed().as_secs_f64();
                let total_time = elapsed / progress;
                let eta = total_time - elapsed;

                if eta > 0.0 {
                    message.push_str(&format!(" (ETA: {:.0}s)", eta));
                }
            }
        }

        tracing::info!("{}", message);
    }

    /// Complete the current operation
    pub fn complete_operation(&mut self, success: bool) {
        if let Some(operation) = &self.current_operation {
            let icon = if success { "✅" } else { "❌" };
            let status = if success { "completed" } else { "failed" };

            let message = if let Some(start_time) = self.start_time {
                let elapsed = start_time.elapsed().as_secs_f64();
                format!("{} {} {} in {:.2}s", icon, operation, status, elapsed)
            } else {
                format!("{} {} {}", icon, operation, status)
            };

            tracing::info!("{}", message);
        }

        self.current_operation = None;
        self.start_time = None;
    }

    /// Report an error during operation
    pub fn report_error(&self, error: &str) {
        tracing::error!("❌ Error: {}", error);
    }

    /// Report a warning during operation
    pub fn report_warning(&self, warning: &str) {
        tracing::warn!("⚠️  Warning: {}", warning);
    }
}

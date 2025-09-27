// Adapters - External system implementations

pub mod env_windows;
pub mod exec_ffmpeg;
pub mod exec_libav;
pub mod fs_windows;
pub mod probe_ffprobe;
pub mod probe_libav;
pub mod toml_config;
pub mod tracing_log;

// Re-export adapters
pub use exec_libav::MockExecutionAdapter;
pub use fs_windows::FsWindowsAdapter;
pub use probe_libav::MockProbeAdapter;
pub use toml_config::TomlConfigAdapter;
pub use tracing_log::TracingLogAdapter;

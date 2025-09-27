// Adapters - External system implementations

pub mod probe_libav;
pub mod exec_libav;
pub mod fs_windows;
pub mod tracing_log;
pub mod toml_config;
pub mod env_windows;
pub mod probe_ffprobe;
pub mod exec_ffmpeg;

// Re-export adapters
pub use probe_libav::LibavProbeAdapter;
pub use exec_libav::LibavExecutionAdapter;
pub use fs_windows::FsWindowsAdapter;
pub use tracing_log::TracingLogAdapter;
pub use toml_config::TomlConfigAdapter;
pub use env_windows::EnvWindowsAdapter;
pub use probe_ffprobe::FFprobeAdapter;
pub use exec_ffmpeg::FFmpegAdapter;

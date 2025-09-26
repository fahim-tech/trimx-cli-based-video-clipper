//! Common utilities and helpers


pub mod container_validator;
pub mod logging;
pub mod memory_manager;
use std::time::Duration;

pub mod time;
pub mod path;

/// Utility functions for TrimX
pub struct Utils;

impl Utils {
    /// Create a new utils instance
    pub fn new() -> Self {
        Self
    }

    /// Format duration for display
    pub fn format_duration(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let milliseconds = duration.subsec_millis();

        if hours > 0 {
            format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, milliseconds)
        } else {
            format!("{:02}:{:02}.{:03}", minutes, seconds, milliseconds)
        }
    }

    /// Format file size for display
    pub fn format_file_size(size: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = size as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.2} {}", size, UNITS[unit_index])
        }
    }

    /// Calculate progress percentage
    pub fn calculate_progress(current: u64, total: u64) -> f32 {
        if total == 0 {
            0.0
        } else {
            (current as f32 / total as f32) * 100.0
        }
    }

    /// Estimate remaining time
    pub fn estimate_remaining_time(
        current: u64,
        total: u64,
        elapsed: Duration,
    ) -> Option<Duration> {
        if current == 0 || current >= total {
            return None;
        }

        let rate = current as f64 / elapsed.as_secs_f64();
        let remaining = (total - current) as f64 / rate;
        
        Some(Duration::from_secs_f64(remaining))
    }
}

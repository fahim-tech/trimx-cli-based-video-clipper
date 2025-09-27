//! Time parsing and formatting utilities

use crate::error::{TrimXError, TrimXResult};

/// Time parser for various time formats
pub struct TimeParser;

impl TimeParser {
    /// Create a new time parser
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }
}

impl Default for TimeParser {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeParser {
    /// Parse time string to seconds
    pub fn parse_time(&self, time_str: &str) -> TrimXResult<f64> {
        let time_str = time_str.trim();

        // Try parsing as seconds (float)
        if let Ok(seconds) = time_str.parse::<f64>() {
            return Ok(seconds);
        }

        // Try parsing as MM:SS.ms format
        if let Some(colon_pos) = time_str.find(':') {
            if let Some(dot_pos) = time_str.find('.') {
                // Format: MM:SS.ms
                if colon_pos < dot_pos {
                    return self.parse_mm_ss_ms(time_str);
                }
            } else {
                // Format: MM:SS
                return self.parse_mm_ss(time_str);
            }
        }

        // Try parsing as HH:MM:SS.ms format
        if time_str.matches(':').count() == 2 {
            return self.parse_hh_mm_ss_ms(time_str);
        }

        Err(TrimXError::InvalidTimeFormat {
            time: time_str.to_string(),
        })
    }

    /// Parse MM:SS format
    fn parse_mm_ss(&self, time_str: &str) -> TrimXResult<f64> {
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 2 {
            return Err(TrimXError::InvalidTimeFormat {
                time: time_str.to_string(),
            });
        }

        let minutes: f64 = parts[0]
            .parse()
            .map_err(|_| TrimXError::InvalidTimeFormat {
                time: time_str.to_string(),
            })?;

        let seconds: f64 = parts[1]
            .parse()
            .map_err(|_| TrimXError::InvalidTimeFormat {
                time: time_str.to_string(),
            })?;

        Ok(minutes * 60.0 + seconds)
    }

    /// Parse MM:SS.ms format
    fn parse_mm_ss_ms(&self, time_str: &str) -> TrimXResult<f64> {
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 2 {
            return Err(TrimXError::InvalidTimeFormat {
                time: time_str.to_string(),
            });
        }

        let minutes: f64 = parts[0]
            .parse()
            .map_err(|_| TrimXError::InvalidTimeFormat {
                time: time_str.to_string(),
            })?;

        let seconds_ms: f64 = parts[1]
            .parse()
            .map_err(|_| TrimXError::InvalidTimeFormat {
                time: time_str.to_string(),
            })?;

        Ok(minutes * 60.0 + seconds_ms)
    }

    /// Parse HH:MM:SS.ms format
    fn parse_hh_mm_ss_ms(&self, time_str: &str) -> TrimXResult<f64> {
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 3 {
            return Err(TrimXError::InvalidTimeFormat {
                time: time_str.to_string(),
            });
        }

        let hours: f64 = parts[0]
            .parse()
            .map_err(|_| TrimXError::InvalidTimeFormat {
                time: time_str.to_string(),
            })?;

        let minutes: f64 = parts[1]
            .parse()
            .map_err(|_| TrimXError::InvalidTimeFormat {
                time: time_str.to_string(),
            })?;

        let seconds_ms: f64 = parts[2]
            .parse()
            .map_err(|_| TrimXError::InvalidTimeFormat {
                time: time_str.to_string(),
            })?;

        Ok(hours * 3600.0 + minutes * 60.0 + seconds_ms)
    }

    /// Format seconds to HH:MM:SS.ms string
    pub fn format_time(&self, seconds: f64) -> String {
        let hours = (seconds / 3600.0) as u32;
        let minutes = ((seconds % 3600.0) / 60.0) as u32;
        let secs = (seconds % 60.0) as u32;
        let milliseconds = ((seconds % 1.0) * 1000.0) as u32;

        if hours > 0 {
            format!(
                "{:02}:{:02}:{:02}.{:03}",
                hours, minutes, secs, milliseconds
            )
        } else {
            format!("{:02}:{:02}.{:03}", minutes, secs, milliseconds)
        }
    }
}

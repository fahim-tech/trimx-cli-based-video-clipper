// Domain models - Core types and data structures

use std::time::Duration;
use std::fmt;
use crate::domain::errors::DomainError;

/// Time specification with precision - represents time in seconds with fractional precision
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct TimeSpec {
    pub seconds: f64,
}

impl TimeSpec {
    /// Create a new TimeSpec from seconds
    pub fn from_seconds(seconds: f64) -> Self {
        Self { seconds }
    }
    
    /// Create a new TimeSpec from hours, minutes, seconds, milliseconds
    pub fn from_components(hours: u32, minutes: u32, seconds: u32, milliseconds: u32) -> Self {
        let total_seconds = hours as f64 * 3600.0 + 
                           minutes as f64 * 60.0 + 
                           seconds as f64 + 
                           milliseconds as f64 / 1000.0;
        Self { seconds: total_seconds }
    }
    
    /// Convert to Duration
    pub fn to_duration(&self) -> Duration {
        Duration::from_secs_f64(self.seconds)
    }
    
    /// Convert from Duration
    pub fn from_duration(duration: Duration) -> Self {
        Self { seconds: duration.as_secs_f64() }
    }
    
    /// Parse time string in various formats
    pub fn parse(time_str: &str) -> Result<Self, DomainError> {
        let trimmed = time_str.trim();
        
        // Try parsing as seconds (float)
        if let Ok(seconds) = trimmed.parse::<f64>() {
            if seconds < 0.0 {
                return Err(DomainError::BadArgs("Time cannot be negative".to_string()));
            }
            return Ok(Self::from_seconds(seconds));
        }
        
        // Try parsing as HH:MM:SS.ms or MM:SS.ms
        let parts: Vec<&str> = trimmed.split(':').collect();
        if parts.len() == 2 {
            // MM:SS.ms format
            let minutes = parts[0].parse::<u32>()
                .map_err(|_| DomainError::BadArgs("Invalid minutes format".to_string()))?;
            let seconds_part = parts[1].parse::<f64>()
                .map_err(|_| DomainError::BadArgs("Invalid seconds format".to_string()))?;
            
            if seconds_part >= 60.0 {
                return Err(DomainError::BadArgs("Seconds must be less than 60".to_string()));
            }
            
            Ok(Self::from_seconds(minutes as f64 * 60.0 + seconds_part))
        } else if parts.len() == 3 {
            // HH:MM:SS.ms format
            let hours = parts[0].parse::<u32>()
                .map_err(|_| DomainError::BadArgs("Invalid hours format".to_string()))?;
            let minutes = parts[1].parse::<u32>()
                .map_err(|_| DomainError::BadArgs("Invalid minutes format".to_string()))?;
            let seconds_part = parts[2].parse::<f64>()
                .map_err(|_| DomainError::BadArgs("Invalid seconds format".to_string()))?;
            
            if minutes >= 60 {
                return Err(DomainError::BadArgs("Minutes must be less than 60".to_string()));
            }
            if seconds_part >= 60.0 {
                return Err(DomainError::BadArgs("Seconds must be less than 60".to_string()));
            }
            
            Ok(Self::from_seconds(hours as f64 * 3600.0 + minutes as f64 * 60.0 + seconds_part))
        } else {
            Err(DomainError::BadArgs(
                "Invalid time format. Supported formats: seconds (e.g., 123.45), MM:SS.ms (e.g., 2:30.5), HH:MM:SS.ms (e.g., 1:02:30.5)".to_string()
            ))
        }
    }
    
    /// Format as HH:MM:SS.ms
    pub fn format_hms(&self) -> String {
        let hours = (self.seconds / 3600.0) as u32;
        let minutes = ((self.seconds % 3600.0) / 60.0) as u32;
        let seconds = (self.seconds % 60.0) as u32;
        let milliseconds = ((self.seconds % 1.0) * 1000.0) as u32;
        
        if hours > 0 {
            format!("{}:{:02}:{:02}.{:03}", hours, minutes, seconds, milliseconds)
        } else {
            format!("{}:{:02}.{:03}", minutes, seconds, milliseconds)
        }
    }
}

impl fmt::Display for TimeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_hms())
    }
}

/// Timebase for timestamp calculations - represents rational number for timestamp conversion
#[derive(Debug, Clone, PartialEq)]
pub struct Timebase {
    pub num: i32,
    pub den: i32,
}

impl Timebase {
    /// Create a new timebase
    pub fn new(num: i32, den: i32) -> Result<Self, DomainError> {
        if den == 0 {
            return Err(DomainError::BadArgs("Timebase denominator cannot be zero".to_string()));
        }
        Ok(Self { num, den })
    }
    
    /// Convert to floating point seconds
    pub fn to_seconds(&self) -> f64 {
        self.num as f64 / self.den as f64
    }
    
    /// Rescale PTS from this timebase to target timebase
    pub fn rescale_pts(&self, pts: i64, target: &Timebase) -> i64 {
        if self.den == target.den && self.num == target.num {
            return pts;
        }
        
        // Convert to seconds and back to target timebase
        let seconds = pts as f64 * self.to_seconds();
        (seconds / target.to_seconds()) as i64
    }
    
    /// Convert PTS to seconds
    pub fn pts_to_seconds(&self, pts: i64) -> f64 {
        pts as f64 * self.to_seconds()
    }
    
    /// Convert seconds to PTS
    pub fn seconds_to_pts(&self, seconds: f64) -> i64 {
        (seconds / self.to_seconds()) as i64
    }
    
    /// Common timebases
    pub fn av_time_base() -> Self {
        Self { num: 1, den: 1000000 } // microseconds
    }
    
    pub fn frame_rate_30() -> Self {
        Self { num: 1, den: 30 }
    }
    
    pub fn frame_rate_25() -> Self {
        Self { num: 1, den: 25 }
    }
    
    pub fn frame_rate_24() -> Self {
        Self { num: 1001, den: 24000 } // 23.976 fps
    }
}

/// Video stream information
#[derive(Debug, Clone)]
pub struct VideoStreamInfo {
    pub index: usize,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
    pub bit_rate: Option<u64>,
    pub timebase: Timebase,
    pub pixel_format: Option<String>,
    pub color_space: Option<String>,
    pub rotation: Option<f32>,
    pub duration: Option<TimeSpec>,
}

impl VideoStreamInfo {
    /// Create new video stream info with validation
    pub fn new(
        index: usize,
        codec: String,
        width: u32,
        height: u32,
        frame_rate: f64,
        timebase: Timebase,
    ) -> Result<Self, DomainError> {
        if width == 0 || height == 0 {
            return Err(DomainError::BadArgs("Video dimensions cannot be zero".to_string()));
        }
        if frame_rate <= 0.0 {
            return Err(DomainError::BadArgs("Frame rate must be positive".to_string()));
        }
        
        Ok(Self {
            index,
            codec,
            width,
            height,
            frame_rate,
            bit_rate: None,
            timebase,
            pixel_format: None,
            color_space: None,
            rotation: None,
            duration: None,
        })
    }
    
    /// Get aspect ratio
    pub fn aspect_ratio(&self) -> f64 {
        self.width as f64 / self.height as f64
    }
    
    /// Check if codec supports copy mode
    pub fn supports_copy(&self) -> bool {
        matches!(self.codec.as_str(), "h264" | "hevc" | "vp9" | "av1")
    }
    
    /// Get frame duration in seconds
    pub fn frame_duration(&self) -> f64 {
        1.0 / self.frame_rate
    }
}

#[cfg(test)]
mod tests;
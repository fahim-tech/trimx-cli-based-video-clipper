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

#[cfg(test)]
mod tests;
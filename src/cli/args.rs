//! Command-line argument definitions

use clap::Args;

/// Arguments for the clip command
#[derive(Args, Debug)]
pub struct ClipArgs {
    /// Input video file path
    #[arg(short, long, value_parser = validate_input_file)]
    pub input: String,

    /// Start time (HH:MM:SS.ms, MM:SS.ms, or seconds)
    #[arg(short, long, value_parser = validate_time_format)]
    pub start: String,

    /// End time (HH:MM:SS.ms, MM:SS.ms, or seconds)
    #[arg(short, long, value_parser = validate_time_format)]
    pub end: String,

    /// Output file path (default: auto-generated)
    #[arg(short, long, value_parser = validate_output_path)]
    pub output: Option<String>,

    /// Clipping strategy
    #[arg(long, default_value = "auto", value_parser = validate_clipping_mode)]
    pub mode: String,

    /// Remove audio streams
    #[arg(long)]
    pub no_audio: bool,

    /// Remove subtitle streams
    #[arg(long)]
    pub no_subs: bool,

    /// Output container format
    #[arg(long, default_value = "same")]
    pub container: String,

    /// Video codec
    #[arg(long, default_value = "h264")]
    pub codec: String,

    /// Constant Rate Factor (0-51)
    #[arg(long, default_value = "18")]
    pub crf: u8,

    /// Encoding preset
    #[arg(long, default_value = "medium")]
    pub preset: String,

    /// Quality setting (0-51)
    #[arg(long)]
    pub quality: Option<u8>,

    /// Overwrite output file if it exists
    #[arg(long)]
    pub overwrite: bool,

    /// Number of threads to use
    #[arg(long)]
    pub threads: Option<usize>,
}

/// Arguments for the inspect command
#[derive(Args, Debug)]
pub struct InspectArgs {
    /// Input video file path
    #[arg(short, long)]
    pub input: String,

    /// Output in JSON format
    #[arg(long)]
    pub json: bool,

    /// Output format (json, yaml, text)
    #[arg(long, default_value = "text")]
    pub format: String,

    /// Show stream information
    #[arg(long)]
    pub show_streams: bool,

    /// Show keyframe information
    #[arg(long)]
    pub show_keyframes: bool,
}

/// Arguments for the verify command
#[derive(Args, Debug)]
pub struct VerifyArgs {
    /// Input video file path
    #[arg(short, long)]
    pub input: String,

    /// Expected start time
    #[arg(short, long)]
    pub start: String,

    /// Expected end time
    #[arg(short, long)]
    pub end: String,

    /// Output file path
    #[arg(short, long)]
    pub output: String,

    /// Clipping mode used
    #[arg(long, default_value = "auto")]
    pub mode: String,

    /// Tolerance in milliseconds
    #[arg(long, default_value = "100")]
    pub tolerance: u32,
}

/// Validate input file path for security
fn validate_input_file(path: &str) -> Result<String, String> {
    // Check for basic security issues
    if path.is_empty() {
        return Err("Input file path cannot be empty".to_string());
    }
    
    if path.len() > 32767 {
        return Err("Input file path too long".to_string());
    }
    
    // Check for path traversal
    if path.contains("..") || path.contains("./") || path.contains(".\\") {
        return Err("Invalid file path: path traversal not allowed".to_string());
    }
    
    // Check for invalid characters
    let invalid_chars = ['<', '>', ':', '"', '|', '?', '*', '\0'];
    if path.chars().any(|c| invalid_chars.contains(&c)) {
        return Err("Invalid file path: contains invalid characters".to_string());
    }
    
    // Check if file exists
    if !std::path::Path::new(path).exists() {
        return Err(format!("Input file does not exist: {}", path));
    }
    
    Ok(path.to_string())
}

/// Validate time format
fn validate_time_format(time: &str) -> Result<String, String> {
    if time.is_empty() {
        return Err("Time cannot be empty".to_string());
    }
    
    // Try to parse as seconds first
    if let Ok(seconds) = time.parse::<f64>() {
        if seconds < 0.0 {
            return Err("Time cannot be negative".to_string());
        }
        return Ok(time.to_string());
    }
    
    // Try to parse as HH:MM:SS.ms or MM:SS.ms
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() == 2 {
        // MM:SS.ms format
        let minutes = parts[0].parse::<u32>()
            .map_err(|_| "Invalid minutes format".to_string())?;
        let seconds_part = parts[1].parse::<f64>()
            .map_err(|_| "Invalid seconds format".to_string())?;
        
        if seconds_part >= 60.0 {
            return Err("Seconds must be less than 60".to_string());
        }
        
        Ok(time.to_string())
    } else if parts.len() == 3 {
        // HH:MM:SS.ms format
        let hours = parts[0].parse::<u32>()
            .map_err(|_| "Invalid hours format".to_string())?;
        let minutes = parts[1].parse::<u32>()
            .map_err(|_| "Invalid minutes format".to_string())?;
        let seconds_part = parts[2].parse::<f64>()
            .map_err(|_| "Invalid seconds format".to_string())?;
        
        if minutes >= 60 {
            return Err("Minutes must be less than 60".to_string());
        }
        if seconds_part >= 60.0 {
            return Err("Seconds must be less than 60".to_string());
        }
        
        Ok(time.to_string())
    } else {
        Err("Invalid time format. Supported formats: seconds (e.g., 123.45), MM:SS.ms (e.g., 2:30.5), HH:MM:SS.ms (e.g., 1:02:30.5)".to_string())
    }
}

/// Validate output file path for security
fn validate_output_path(path: &str) -> Result<String, String> {
    // Check for basic security issues
    if path.is_empty() {
        return Err("Output file path cannot be empty".to_string());
    }
    
    if path.len() > 32767 {
        return Err("Output file path too long".to_string());
    }
    
    // Check for path traversal
    if path.contains("..") || path.contains("./") || path.contains(".\\") {
        return Err("Invalid file path: path traversal not allowed".to_string());
    }
    
    // Check for invalid characters
    let invalid_chars = ['<', '>', ':', '"', '|', '?', '*', '\0'];
    if path.chars().any(|c| invalid_chars.contains(&c)) {
        return Err("Invalid file path: contains invalid characters".to_string());
    }
    
    // Check for reserved Windows names
    let path_obj = std::path::Path::new(path);
    if let Some(filename) = path_obj.file_name() {
        if let Some(filename_str) = filename.to_str() {
            let reserved_names = [
                "CON", "PRN", "AUX", "NUL",
                "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
                "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"
            ];
            
            let filename_upper = filename_str.to_uppercase();
            if reserved_names.contains(&filename_upper.as_str()) {
                return Err("Invalid file name: reserved Windows name".to_string());
            }
        }
    }
    
    Ok(path.to_string())
}

/// Validate clipping mode
fn validate_clipping_mode(mode: &str) -> Result<String, String> {
    let valid_modes = ["auto", "copy", "reencode", "hybrid"];
    let mode_lower = mode.to_lowercase();
    
    if valid_modes.contains(&mode_lower.as_str()) {
        Ok(mode.to_string())
    } else {
        Err(format!("Invalid clipping mode: {}. Valid modes: {}", mode, valid_modes.join(", ")))
    }
}

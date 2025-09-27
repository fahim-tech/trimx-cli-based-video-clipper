//! Container format validation and compatibility checking

use crate::error::{TrimXError, TrimXResult};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{debug, info};

/// Container format validator
pub struct ContainerValidator {
    /// Enable debug logging
    debug: bool,
    /// Codec compatibility matrix
    codec_compatibility: HashMap<String, HashSet<String>>,
    /// Supported input formats
    supported_inputs: HashSet<String>,
    /// Supported output formats
    supported_outputs: HashSet<String>,
}

/// Container format information
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    /// Format name
    pub format: String,
    /// Format long name/description
    pub description: String,
    /// File extensions associated with this format
    pub extensions: Vec<String>,
    /// Supported video codecs
    pub video_codecs: Vec<String>,
    /// Supported audio codecs
    pub audio_codecs: Vec<String>,
    /// Supports lossless stream copying
    pub supports_stream_copy: bool,
    /// Maximum streams supported
    pub max_streams: Option<u32>,
    /// Container capabilities
    pub capabilities: ContainerCapabilities,
}

/// Container capabilities
#[derive(Debug, Clone)]
pub struct ContainerCapabilities {
    /// Supports variable frame rate
    pub supports_vfr: bool,
    /// Supports chapters
    pub supports_chapters: bool,
    /// Supports metadata
    pub supports_metadata: bool,
    /// Supports subtitles
    pub supports_subtitles: bool,
    /// Supports seeking
    pub supports_seeking: bool,
    /// Supports streaming
    pub supports_streaming: bool,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Validation passed
    pub valid: bool,
    /// Input format information
    pub input_format: Option<ContainerInfo>,
    /// Output format information
    pub output_format: Option<ContainerInfo>,
    /// Compatibility warnings
    pub warnings: Vec<String>,
    /// Compatibility errors
    pub errors: Vec<String>,
    /// Recommended actions
    pub recommendations: Vec<String>,
}

impl ContainerValidator {
    /// Create a new container validator
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut validator = Self {
            debug: false,
            codec_compatibility: HashMap::new(),
            supported_inputs: HashSet::new(),
            supported_outputs: HashSet::new(),
        };

        validator.initialize_compatibility_matrix();
        validator
    }
}

impl Default for ContainerValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerValidator {
    /// Enable debug logging
    pub fn with_debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// Initialize codec compatibility matrix
    fn initialize_compatibility_matrix(&mut self) {
        // MP4/MOV container compatibility
        let mut mp4_codecs = HashSet::new();
        mp4_codecs.insert("h264".to_string());
        mp4_codecs.insert("h265".to_string());
        mp4_codecs.insert("hevc".to_string());
        mp4_codecs.insert("mpeg4".to_string());
        mp4_codecs.insert("aac".to_string());
        mp4_codecs.insert("mp3".to_string());
        mp4_codecs.insert("ac3".to_string());
        self.codec_compatibility
            .insert("mp4".to_string(), mp4_codecs.clone());
        self.codec_compatibility
            .insert("mov".to_string(), mp4_codecs.clone());

        // MKV container compatibility (very flexible)
        let mut mkv_codecs = HashSet::new();
        mkv_codecs.insert("h264".to_string());
        mkv_codecs.insert("h265".to_string());
        mkv_codecs.insert("hevc".to_string());
        mkv_codecs.insert("vp8".to_string());
        mkv_codecs.insert("vp9".to_string());
        mkv_codecs.insert("av1".to_string());
        mkv_codecs.insert("aac".to_string());
        mkv_codecs.insert("mp3".to_string());
        mkv_codecs.insert("vorbis".to_string());
        mkv_codecs.insert("opus".to_string());
        mkv_codecs.insert("flac".to_string());
        mkv_codecs.insert("ac3".to_string());
        mkv_codecs.insert("dts".to_string());
        self.codec_compatibility
            .insert("mkv".to_string(), mkv_codecs);

        // WebM container compatibility
        let mut webm_codecs = HashSet::new();
        webm_codecs.insert("vp8".to_string());
        webm_codecs.insert("vp9".to_string());
        webm_codecs.insert("av1".to_string());
        webm_codecs.insert("vorbis".to_string());
        webm_codecs.insert("opus".to_string());
        self.codec_compatibility
            .insert("webm".to_string(), webm_codecs);

        // AVI container compatibility
        let mut avi_codecs = HashSet::new();
        avi_codecs.insert("h264".to_string());
        avi_codecs.insert("mpeg4".to_string());
        avi_codecs.insert("xvid".to_string());
        avi_codecs.insert("mp3".to_string());
        avi_codecs.insert("ac3".to_string());
        avi_codecs.insert("pcm".to_string());
        self.codec_compatibility
            .insert("avi".to_string(), avi_codecs);

        // Initialize supported formats
        self.supported_inputs.insert("mp4".to_string());
        self.supported_inputs.insert("mov".to_string());
        self.supported_inputs.insert("mkv".to_string());
        self.supported_inputs.insert("webm".to_string());
        self.supported_inputs.insert("avi".to_string());
        self.supported_inputs.insert("m4v".to_string());
        self.supported_inputs.insert("3gp".to_string());
        self.supported_inputs.insert("ts".to_string());
        self.supported_inputs.insert("mts".to_string());

        self.supported_outputs.insert("mp4".to_string());
        self.supported_outputs.insert("mov".to_string());
        self.supported_outputs.insert("mkv".to_string());
        self.supported_outputs.insert("webm".to_string());
        self.supported_outputs.insert("avi".to_string());
    }

    /// Validate container format compatibility
    pub fn validate_formats(
        &self,
        input_path: &str,
        output_path: &str,
    ) -> TrimXResult<ValidationResult> {
        info!("Validating container format compatibility");
        debug!("Input: {}", input_path);
        debug!("Output: {}", output_path);

        let mut result = ValidationResult {
            valid: true,
            input_format: None,
            output_format: None,
            warnings: Vec::new(),
            errors: Vec::new(),
            recommendations: Vec::new(),
        };

        // Detect input format
        let input_format = self.detect_format_from_file(input_path)?;
        result.input_format = Some(input_format.clone());

        // Detect output format
        let output_format = self.detect_format_from_extension(output_path)?;
        result.output_format = Some(output_format.clone());

        // Check input format support
        if !self.supported_inputs.contains(&input_format.format) {
            result.errors.push(format!(
                "Input format '{}' is not supported",
                input_format.format
            ));
            result.valid = false;
        }

        // Check output format support
        if !self.supported_outputs.contains(&output_format.format) {
            result.errors.push(format!(
                "Output format '{}' is not supported",
                output_format.format
            ));
            result.valid = false;
        }

        // Check codec compatibility
        self.check_codec_compatibility(&input_format, &output_format, &mut result);

        // Check format-specific limitations
        self.check_format_limitations(&input_format, &output_format, &mut result);

        // Generate recommendations
        self.generate_recommendations(&input_format, &output_format, &mut result);

        info!(
            "Format validation complete: {} errors, {} warnings",
            result.errors.len(),
            result.warnings.len()
        );

        Ok(result)
    }

    /// Detect format from actual file
    fn detect_format_from_file(&self, file_path: &str) -> TrimXResult<ContainerInfo> {
        // Initialize FFmpeg
        ffmpeg_next::init().map_err(|e| TrimXError::ClippingError {
            message: format!("Failed to initialize FFmpeg: {}", e),
        })?;

        // Open file to detect format
        let input_ctx =
            ffmpeg_next::format::input(file_path).map_err(|e| TrimXError::ClippingError {
                message: format!("Failed to open input file: {}", e),
            })?;

        let format = input_ctx.format();
        let format_name = format.name();
        let format_desc = format.description();

        // Get video and audio codecs from streams
        let mut video_codecs = Vec::new();
        let mut audio_codecs = Vec::new();

        for stream in input_ctx.streams() {
            let codec_name = stream.parameters().id().name().to_string();

            match stream.parameters().medium() {
                ffmpeg_next::media::Type::Video => {
                    if !video_codecs.contains(&codec_name) {
                        video_codecs.push(codec_name);
                    }
                }
                ffmpeg_next::media::Type::Audio => {
                    if !audio_codecs.contains(&codec_name) {
                        audio_codecs.push(codec_name);
                    }
                }
                _ => {} // Skip other stream types
            }
        }

        Ok(ContainerInfo {
            format: format_name.to_string(),
            description: format_desc.to_string(),
            extensions: self.get_extensions_for_format(format_name),
            video_codecs,
            audio_codecs,
            supports_stream_copy: true, // Most formats support stream copy
            max_streams: None,          // Would need format-specific logic
            capabilities: self.get_format_capabilities(format_name),
        })
    }

    /// Detect format from file extension
    fn detect_format_from_extension(&self, file_path: &str) -> TrimXResult<ContainerInfo> {
        let path = Path::new(file_path);
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .ok_or_else(|| TrimXError::ClippingError {
                message: "Could not determine output format from file extension".to_string(),
            })?;

        let format_name = match extension.as_str() {
            "mp4" | "m4v" => "mp4",
            "mov" => "mov",
            "mkv" => "mkv",
            "webm" => "webm",
            "avi" => "avi",
            _ => {
                return Err(TrimXError::ClippingError {
                    message: format!("Unsupported output format: {}", extension),
                })
            }
        };

        let compatible_codecs = self
            .codec_compatibility
            .get(format_name)
            .cloned()
            .unwrap_or_else(HashSet::new);

        Ok(ContainerInfo {
            format: format_name.to_string(),
            description: self.get_format_description(format_name),
            extensions: vec![extension],
            video_codecs: compatible_codecs
                .iter()
                .filter(|codec| self.is_video_codec(codec))
                .cloned()
                .collect(),
            audio_codecs: compatible_codecs
                .iter()
                .filter(|codec| self.is_audio_codec(codec))
                .cloned()
                .collect(),
            supports_stream_copy: true,
            max_streams: None,
            capabilities: self.get_format_capabilities(format_name),
        })
    }

    /// Check codec compatibility between input and output
    fn check_codec_compatibility(
        &self,
        input: &ContainerInfo,
        output: &ContainerInfo,
        result: &mut ValidationResult,
    ) {
        let output_compatible_codecs = self
            .codec_compatibility
            .get(&output.format)
            .cloned()
            .unwrap_or_else(HashSet::new);

        // Check video codec compatibility
        for video_codec in &input.video_codecs {
            if !output_compatible_codecs.contains(video_codec) {
                result.warnings.push(format!(
                    "Video codec '{}' may not be compatible with {} container (will require re-encoding)", 
                    video_codec, output.format
                ));
            }
        }

        // Check audio codec compatibility
        for audio_codec in &input.audio_codecs {
            if !output_compatible_codecs.contains(audio_codec) {
                result.warnings.push(format!(
                    "Audio codec '{}' may not be compatible with {} container (will require re-encoding)", 
                    audio_codec, output.format
                ));
            }
        }
    }

    /// Check format-specific limitations
    fn check_format_limitations(
        &self,
        input: &ContainerInfo,
        output: &ContainerInfo,
        result: &mut ValidationResult,
    ) {
        // Check for lossy format conversions
        if input.format == "mkv" && output.format == "mp4" {
            result.warnings.push(
                "Converting from MKV to MP4 may lose some subtitle or metadata information"
                    .to_string(),
            );
        }

        if input.format == "avi" && (output.format == "mp4" || output.format == "mkv") {
            result.recommendations.push(
                "Consider using MP4 or MKV for better compatibility and features".to_string(),
            );
        }

        // Check for stream copy limitations
        if !output.supports_stream_copy {
            result.warnings.push(format!(
                "Output format '{}' may not support lossless stream copying",
                output.format
            ));
        }
    }

    /// Generate format recommendations
    fn generate_recommendations(
        &self,
        input: &ContainerInfo,
        output: &ContainerInfo,
        result: &mut ValidationResult,
    ) {
        // Recommend MP4 for broad compatibility
        if output.format != "mp4" && input.video_codecs.contains(&"h264".to_string()) {
            result.recommendations.push(
                "Consider using MP4 format for maximum compatibility across devices and players"
                    .to_string(),
            );
        }

        // Recommend MKV for advanced features
        if input.capabilities.supports_subtitles && !output.capabilities.supports_subtitles {
            result
                .recommendations
                .push("Consider using MKV format to preserve subtitle streams".to_string());
        }

        // Recommend keeping same format if compatible
        if input.format != output.format && self.supported_outputs.contains(&input.format) {
            result.recommendations.push(format!(
                "Consider keeping original {} format to avoid potential compatibility issues",
                input.format
            ));
        }
    }

    /// Helper functions
    fn get_extensions_for_format(&self, format: &str) -> Vec<String> {
        match format {
            "mp4" => vec!["mp4".to_string(), "m4v".to_string()],
            "mov" => vec!["mov".to_string()],
            "mkv" => vec!["mkv".to_string()],
            "webm" => vec!["webm".to_string()],
            "avi" => vec!["avi".to_string()],
            _ => vec![format.to_string()],
        }
    }

    fn get_format_description(&self, format: &str) -> String {
        match format {
            "mp4" => "MPEG-4 Part 14".to_string(),
            "mov" => "QuickTime Movie".to_string(),
            "mkv" => "Matroska Video".to_string(),
            "webm" => "WebM Video".to_string(),
            "avi" => "Audio Video Interleave".to_string(),
            _ => format.to_uppercase(),
        }
    }

    fn get_format_capabilities(&self, format: &str) -> ContainerCapabilities {
        match format {
            "mp4" | "mov" => ContainerCapabilities {
                supports_vfr: true,
                supports_chapters: true,
                supports_metadata: true,
                supports_subtitles: true,
                supports_seeking: true,
                supports_streaming: true,
            },
            "mkv" => ContainerCapabilities {
                supports_vfr: true,
                supports_chapters: true,
                supports_metadata: true,
                supports_subtitles: true,
                supports_seeking: true,
                supports_streaming: false,
            },
            "webm" => ContainerCapabilities {
                supports_vfr: true,
                supports_chapters: false,
                supports_metadata: true,
                supports_subtitles: true,
                supports_seeking: true,
                supports_streaming: true,
            },
            "avi" => ContainerCapabilities {
                supports_vfr: false,
                supports_chapters: false,
                supports_metadata: true,
                supports_subtitles: false,
                supports_seeking: true,
                supports_streaming: false,
            },
            _ => ContainerCapabilities {
                supports_vfr: false,
                supports_chapters: false,
                supports_metadata: false,
                supports_subtitles: false,
                supports_seeking: true,
                supports_streaming: false,
            },
        }
    }

    fn is_video_codec(&self, codec: &str) -> bool {
        matches!(
            codec,
            "h264" | "h265" | "hevc" | "vp8" | "vp9" | "av1" | "mpeg4" | "xvid"
        )
    }

    fn is_audio_codec(&self, codec: &str) -> bool {
        matches!(
            codec,
            "aac" | "mp3" | "ac3" | "dts" | "vorbis" | "opus" | "flac" | "pcm"
        )
    }
}

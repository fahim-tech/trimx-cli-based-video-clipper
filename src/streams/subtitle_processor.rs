//! Subtitle stream processing and preservation

use std::path::Path;
use tracing::{info, debug, warn};
use crate::error::{TrimXError, TrimXResult};

/// Subtitle stream information (placeholder)
#[derive(Debug, Clone)]
pub struct SubtitleStreamInfo {
    pub index: usize,
    pub codec: String,
    pub language: Option<String>,
    pub forced: bool,
    pub default: bool,
    pub title: Option<String>,
}

/// Subtitle processing configuration
#[derive(Debug, Clone)]
pub struct SubtitleConfig {
    /// Include all subtitle streams
    pub include_all: bool,
    /// Select specific streams by index
    pub selected_streams: Option<Vec<usize>>,
    /// Language preference order
    pub language_preference: Vec<String>,
    /// Include forced subtitles only
    pub forced_only: bool,
    /// Convert subtitle format
    pub target_format: Option<SubtitleFormat>,
    /// Extract subtitles to separate files
    pub extract_to_files: bool,
    /// Output directory for extracted subtitles
    pub output_directory: Option<String>,
}

/// Supported subtitle formats
#[derive(Debug, Clone, PartialEq)]
pub enum SubtitleFormat {
    /// SubRip (.srt)
    Srt,
    /// WebVTT (.vtt)
    WebVtt,
    /// Advanced SubStation Alpha (.ass)
    Ass,
    /// SubStation Alpha (.ssa)
    Ssa,
    /// DVD Video Object (.vob)
    VobSub,
    /// Presentation Graphic Stream (.sup)
    Pgs,
    /// Teletext
    Teletext,
    /// Closed Captions
    ClosedCaptions,
}

/// Subtitle stream mapping
#[derive(Debug, Clone)]
pub struct SubtitleMapping {
    /// Input stream index
    pub input_index: usize,
    /// Output stream index
    pub output_index: usize,
    /// Stream information
    pub stream_info: SubtitleStreamInfo,
    /// Processing mode
    pub processing_mode: SubtitleProcessingMode,
    /// Quality score for selection
    pub quality_score: f64,
}

/// Subtitle processing modes
#[derive(Debug, Clone)]
pub enum SubtitleProcessingMode {
    /// Copy subtitle stream as-is
    Copy,
    /// Convert to different format
    Convert { 
        target_format: SubtitleFormat,
        time_offset: f64,
    },
    /// Extract to external file
    Extract {
        output_path: String,
        format: SubtitleFormat,
    },
    /// Burn into video (not supported in stream copy)
    Burn,
}

/// Subtitle processor
pub struct SubtitleProcessor {
    /// Enable debug logging
    debug: bool,
}

impl Default for SubtitleConfig {
    fn default() -> Self {
        Self {
            include_all: false,
            selected_streams: None,
            language_preference: vec!["en".to_string(), "eng".to_string()],
            forced_only: false,
            target_format: None,
            extract_to_files: false,
            output_directory: None,
        }
    }
}

impl SubtitleProcessor {
    /// Create a new subtitle processor
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self { debug: false }
    }

    /// Enable debug logging
    pub fn with_debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// Create subtitle stream mappings
    pub fn create_mappings(
        &self,
        subtitle_streams: &[SubtitleStreamInfo],
        config: &SubtitleConfig,
    ) -> TrimXResult<Vec<SubtitleMapping>> {
        if subtitle_streams.is_empty() {
            info!("No subtitle streams found in input");
            return Ok(Vec::new());
        }

        debug!("Creating subtitle mappings for {} streams", subtitle_streams.len());

        let mappings = if config.include_all {
            self.map_all_streams(subtitle_streams, config)?
        } else if let Some(ref selected_indices) = config.selected_streams {
            self.map_selected_streams(subtitle_streams, selected_indices, config)?
        } else {
            self.auto_select_streams(subtitle_streams, config)?
        };

        info!("Created {} subtitle stream mappings", mappings.len());
        Ok(mappings)
    }

    /// Map all subtitle streams
    fn map_all_streams(
        &self,
        streams: &[SubtitleStreamInfo],
        config: &SubtitleConfig,
    ) -> TrimXResult<Vec<SubtitleMapping>> {
        let mut mappings = Vec::new();

        for (input_index, stream) in streams.iter().enumerate() {
            if config.forced_only && !stream.forced {
                continue;
            }

            let mapping = self.create_mapping(stream, input_index, mappings.len(), config)?;
            mappings.push(mapping);
        }

        Ok(mappings)
    }

    /// Map selected streams by index
    fn map_selected_streams(
        &self,
        streams: &[SubtitleStreamInfo],
        selected_indices: &[usize],
        config: &SubtitleConfig,
    ) -> TrimXResult<Vec<SubtitleMapping>> {
        let mut mappings = Vec::new();

        for &input_index in selected_indices {
            if let Some(stream) = streams.get(input_index) {
                if config.forced_only && !stream.forced {
                    warn!("Stream {} is not forced, skipping due to forced_only setting", input_index);
                    continue;
                }

                let mapping = self.create_mapping(stream, input_index, mappings.len(), config)?;
                mappings.push(mapping);
            } else {
                warn!("Subtitle stream index {} not found, skipping", input_index);
            }
        }

        Ok(mappings)
    }

    /// Auto-select best streams
    fn auto_select_streams(
        &self,
        streams: &[SubtitleStreamInfo],
        config: &SubtitleConfig,
    ) -> TrimXResult<Vec<SubtitleMapping>> {
        // Score each stream
        let mut scored_streams: Vec<(usize, &SubtitleStreamInfo, f64)> = streams
            .iter()
            .enumerate()
            .map(|(index, stream)| {
                let score = self.calculate_quality_score(stream, config);
                (index, stream, score)
            })
            .collect();

        // Sort by score (descending)
        scored_streams.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

        let mut mappings = Vec::new();
        let mut languages_seen = std::collections::HashSet::new();

        for (input_index, stream, _score) in scored_streams {
            if config.forced_only && !stream.forced {
                continue;
            }

            let language = stream.language.as_deref().unwrap_or("unknown");

            // Include first stream of each language
            if !languages_seen.contains(language) || mappings.is_empty() {
                let mapping = self.create_mapping(stream, input_index, mappings.len(), config)?;
                mappings.push(mapping);
                languages_seen.insert(language.to_string());

                // Limit to reasonable number of streams
                if mappings.len() >= 5 {
                    break;
                }
            }
        }

        Ok(mappings)
    }

    /// Create a single subtitle mapping
    fn create_mapping(
        &self,
        stream: &SubtitleStreamInfo,
        input_index: usize,
        output_index: usize,
        config: &SubtitleConfig,
    ) -> TrimXResult<SubtitleMapping> {
        let processing_mode = self.determine_processing_mode(stream, config, output_index)?;
        let quality_score = self.calculate_quality_score(stream, config);

        Ok(SubtitleMapping {
            input_index,
            output_index,
            stream_info: stream.clone(),
            processing_mode,
            quality_score,
        })
    }

    /// Calculate quality score for stream selection
    fn calculate_quality_score(&self, stream: &SubtitleStreamInfo, config: &SubtitleConfig) -> f64 {
        let mut score = 0.0;

        // Codec quality score
        score += match stream.codec.as_str() {
            "subrip" | "srt" => 90.0,          // Text-based, widely supported
            "webvtt" | "vtt" => 85.0,          // Modern web standard
            "ass" | "ssa" => 80.0,             // Advanced styling
            "dvd_subtitle" | "vobsub" => 70.0,  // Image-based
            "pgs" => 75.0,                     // Blu-ray subtitles
            "teletext" => 60.0,                // Legacy
            "mov_text" => 85.0,                // MP4 text subtitles
            _ => 50.0,                         // Unknown/other
        };

        // Language preference score
        if let Some(ref language) = stream.language {
            let lang_lower = language.to_lowercase();
            for (index, preferred) in config.language_preference.iter().enumerate() {
                if lang_lower.contains(&preferred.to_lowercase()) {
                    score += 50.0 - (index as f64 * 10.0);
                    break;
                }
            }
        } else {
            // Penalize streams without language info
            score -= 10.0;
        }

        // Forced subtitle bonus
        if stream.forced {
            score += 20.0;
        }

        // Default stream bonus
        if stream.default {
            score += 15.0;
        }

        // Title information bonus
        if stream.title.is_some() {
            score += 5.0;
        }

        score
    }

    /// Determine processing mode for a subtitle stream
    fn determine_processing_mode(
        &self,
        stream: &SubtitleStreamInfo,
        config: &SubtitleConfig,
        output_index: usize,
    ) -> TrimXResult<SubtitleProcessingMode> {
        // Check if extraction to files is requested
        if config.extract_to_files {
            let output_path = self.generate_output_path(stream, config, output_index)?;
            let target_format = config.target_format.clone()
                .unwrap_or_else(|| self.detect_best_format(&stream.codec));

            return Ok(SubtitleProcessingMode::Extract {
                output_path,
                format: target_format,
            });
        }

        // Check if format conversion is requested
        if let Some(ref target_format) = config.target_format {
            if &self.codec_to_format(&stream.codec) != target_format {
                return Ok(SubtitleProcessingMode::Convert {
                    target_format: target_format.clone(),
                    time_offset: 0.0, // Would be calculated based on cut times
                });
            }
        }

        // Default to copy
        Ok(SubtitleProcessingMode::Copy)
    }

    /// Generate output path for extracted subtitles
    fn generate_output_path(
        &self,
        stream: &SubtitleStreamInfo,
        config: &SubtitleConfig,
        output_index: usize,
    ) -> TrimXResult<String> {
        let output_dir = config.output_directory.as_deref().unwrap_or(".");
        
        let language = stream.language.as_deref().unwrap_or("unknown");
        let forced_suffix = if stream.forced { ".forced" } else { "" };
        
        let extension = match config.target_format.as_ref().unwrap_or(&SubtitleFormat::Srt) {
            SubtitleFormat::Srt => "srt",
            SubtitleFormat::WebVtt => "vtt",
            SubtitleFormat::Ass => "ass",
            SubtitleFormat::Ssa => "ssa",
            SubtitleFormat::VobSub => "sub",
            SubtitleFormat::Pgs => "sup",
            SubtitleFormat::Teletext => "txt",
            SubtitleFormat::ClosedCaptions => "scc",
        };

        let filename = format!("subtitles_{}.{}{}.{}", output_index, language, forced_suffix, extension);
        let output_path = Path::new(output_dir).join(filename);
        
        Ok(output_path.to_string_lossy().to_string())
    }

    /// Convert codec name to subtitle format
    fn codec_to_format(&self, codec: &str) -> SubtitleFormat {
        match codec {
            "subrip" | "srt" => SubtitleFormat::Srt,
            "webvtt" | "vtt" => SubtitleFormat::WebVtt,
            "ass" => SubtitleFormat::Ass,
            "ssa" => SubtitleFormat::Ssa,
            "dvd_subtitle" | "vobsub" => SubtitleFormat::VobSub,
            "pgs" => SubtitleFormat::Pgs,
            "teletext" => SubtitleFormat::Teletext,
            "mov_text" => SubtitleFormat::Srt, // Convert to SRT for compatibility
            _ => SubtitleFormat::Srt, // Default fallback
        }
    }

    /// Detect best output format for a codec
    fn detect_best_format(&self, codec: &str) -> SubtitleFormat {
        match codec {
            "dvd_subtitle" | "vobsub" => SubtitleFormat::Srt, // Convert bitmap to text
            "pgs" => SubtitleFormat::Srt, // Convert bitmap to text
            "teletext" => SubtitleFormat::Srt, // Convert to standard format
            _ => self.codec_to_format(codec), // Keep original format
        }
    }

    /// Validate subtitle configuration
    pub fn validate_config(&self, config: &SubtitleConfig) -> TrimXResult<()> {
        // Check for conflicting options
        if config.include_all && config.selected_streams.is_some() {
            return Err(TrimXError::ClippingError {
                message: "Cannot specify both include_all and selected_streams for subtitles".to_string()
            });
        }

        // Validate selected stream indices
        if let Some(ref indices) = config.selected_streams {
            if indices.is_empty() {
                return Err(TrimXError::ClippingError {
                    message: "Selected subtitle streams list cannot be empty".to_string()
                });
            }
        }

        // Validate output directory
        if config.extract_to_files {
            if let Some(ref output_dir) = config.output_directory {
                let path = Path::new(output_dir);
                if !path.exists() {
                    return Err(TrimXError::ClippingError {
                        message: format!("Output directory does not exist: {}", output_dir)
                    });
                }
                if !path.is_dir() {
                    return Err(TrimXError::ClippingError {
                        message: format!("Output path is not a directory: {}", output_dir)
                    });
                }
            }
        }

        Ok(())
    }

    /// Get supported subtitle formats for container
    pub fn get_supported_formats(&self, container_format: &str) -> Vec<SubtitleFormat> {
        match container_format.to_lowercase().as_str() {
            "mp4" | "m4v" | "mov" => vec![
                SubtitleFormat::Srt,
                // MP4 has limited subtitle support
            ],
            "mkv" => vec![
                SubtitleFormat::Srt,
                SubtitleFormat::Ass,
                SubtitleFormat::Ssa,
                SubtitleFormat::VobSub,
                SubtitleFormat::Pgs,
                SubtitleFormat::WebVtt,
            ],
            "webm" => vec![
                SubtitleFormat::WebVtt,
            ],
            "avi" => vec![
                SubtitleFormat::Srt, // External subtitles only
            ],
            _ => vec![SubtitleFormat::Srt], // Default fallback
        }
    }

    /// Process subtitle streams during clipping
    pub fn process_streams(
        &self,
        mappings: &[SubtitleMapping],
        start_time: f64,
        _end_time: f64,
    ) -> TrimXResult<()> {
        info!("Processing {} subtitle streams", mappings.len());

        for mapping in mappings {
            match &mapping.processing_mode {
                SubtitleProcessingMode::Copy => {
                    debug!("Copying subtitle stream {}", mapping.input_index);
                    // Stream copy - handled by FFmpeg
                }
                SubtitleProcessingMode::Convert { target_format, time_offset } => {
                    debug!("Converting subtitle stream {} to {:?} with offset {:.3}s", 
                           mapping.input_index, target_format, time_offset);
                    // Format conversion - would implement with FFmpeg filters
                }
                SubtitleProcessingMode::Extract { output_path, format } => {
                    info!("Extracting subtitle stream {} to {} as {:?}", 
                          mapping.input_index, output_path, format);
                    // Subtitle extraction - would implement with FFmpeg
                }
                SubtitleProcessingMode::Burn => {
                    warn!("Subtitle burning not supported in stream copy mode for stream {}", 
                          mapping.input_index);
                    // Burning requires re-encoding
                }
            }

            // Adjust timing for extracted subtitles
            if matches!(mapping.processing_mode, SubtitleProcessingMode::Extract { .. }) {
                self.adjust_subtitle_timing(mapping, start_time)?;
            }
        }

        Ok(())
    }

    /// Adjust subtitle timing for extracted files
    fn adjust_subtitle_timing(&self, mapping: &SubtitleMapping, start_time: f64) -> TrimXResult<()> {
        debug!("Adjusting timing for subtitle stream {} by -{:.3}s", 
               mapping.input_index, start_time);
        
        // In a full implementation, this would:
        // 1. Parse subtitle file
        // 2. Adjust all timestamps by subtracting start_time
        // 3. Write adjusted subtitle file
        
        // For now, this is a placeholder
        Ok(())
    }

    /// Generate processing report
    pub fn generate_report(&self, mappings: &[SubtitleMapping]) -> String {
        let mut report = String::new();
        
        report.push_str("Subtitle Stream Processing Report:\n");
        report.push_str(&format!("  Total mappings: {}\n", mappings.len()));
        
        for (index, mapping) in mappings.iter().enumerate() {
            report.push_str(&format!("\n  Stream {}:\n", index));
            report.push_str(&format!("    Input Index: {}\n", mapping.input_index));
            report.push_str(&format!("    Codec: {}\n", mapping.stream_info.codec));
            
            if let Some(ref language) = mapping.stream_info.language {
                report.push_str(&format!("    Language: {}\n", language));
            }
            
            if mapping.stream_info.forced {
                report.push_str("    Type: Forced\n");
            }
            
            if mapping.stream_info.default {
                report.push_str("    Default: Yes\n");
            }
            
            if let Some(ref title) = mapping.stream_info.title {
                report.push_str(&format!("    Title: {}\n", title));
            }
            
            report.push_str(&format!("    Processing: {:?}\n", mapping.processing_mode));
            report.push_str(&format!("    Quality Score: {:.1}\n", mapping.quality_score));
        }
        
        report
    }
}

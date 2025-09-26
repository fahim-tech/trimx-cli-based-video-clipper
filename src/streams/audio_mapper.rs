//! Audio stream mapping and selection logic

use tracing::{info, debug, warn};
use crate::error::{TrimXError, TrimXResult};
// Note: In production, would import from domain model
// use crate::domain::model::{AudioStreamInfo, MediaInfo};

/// Placeholder for AudioStreamInfo
#[derive(Debug, Clone)]
pub struct AudioStreamInfo {
    pub index: usize,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub bit_rate: Option<u64>,
    pub language: Option<String>,
}

/// Placeholder for MediaInfo
#[derive(Debug, Clone)]  
pub struct MediaInfo {
    pub audio_streams: Vec<AudioStreamInfo>,
}

/// Audio stream mapping configuration
#[derive(Debug, Clone)]
pub struct AudioMappingConfig {
    /// Select specific audio streams by index
    pub selected_streams: Option<Vec<usize>>,
    /// Select streams by language preference
    pub language_preference: Vec<String>,
    /// Select best quality stream automatically
    pub auto_select_best: bool,
    /// Include all audio streams
    pub include_all: bool,
    /// Audio codec override for re-encoding
    pub target_codec: Option<String>,
    /// Audio bitrate override
    pub target_bitrate: Option<u64>,
    /// Audio channel configuration
    pub channel_config: Option<AudioChannelConfig>,
}

/// Audio channel configuration
#[derive(Debug, Clone)]
pub struct AudioChannelConfig {
    /// Target channel count
    pub channels: u32,
    /// Channel layout (e.g., "stereo", "5.1", "7.1")
    pub layout: String,
    /// Downmix strategy for multi-channel sources
    pub downmix_strategy: DownmixStrategy,
}

/// Downmix strategy for multi-channel audio
#[derive(Debug, Clone)]
pub enum DownmixStrategy {
    /// Simple downmix (basic channel mapping)
    Simple,
    /// Dolby Pro Logic II downmix
    ProLogicII,
    /// Center spread (spread center channel to L/R)
    CenterSpread,
    /// Custom downmix matrix
    Custom(Vec<Vec<f32>>),
}

/// Audio stream mapping result
#[derive(Debug, Clone)]
pub struct AudioMapping {
    /// Input stream index
    pub input_index: usize,
    /// Output stream index
    pub output_index: usize,
    /// Stream information
    pub stream_info: AudioStreamInfo,
    /// Processing mode
    pub processing_mode: AudioProcessingMode,
    /// Quality score (higher is better)
    pub quality_score: f64,
}

/// Audio processing mode
#[derive(Debug, Clone)]
pub enum AudioProcessingMode {
    /// Copy stream without re-encoding
    Copy,
    /// Re-encode with same parameters
    Reencode,
    /// Re-encode with different parameters
    Transcode {
        target_codec: String,
        target_bitrate: Option<u64>,
        target_channels: Option<u32>,
        target_sample_rate: Option<u32>,
    },
}

/// Audio stream mapper
pub struct AudioMapper {
    /// Enable debug logging
    debug: bool,
}

impl Default for AudioMappingConfig {
    fn default() -> Self {
        Self {
            selected_streams: None,
            language_preference: vec!["en".to_string(), "eng".to_string()],
            auto_select_best: true,
            include_all: false,
            target_codec: None,
            target_bitrate: None,
            channel_config: None,
        }
    }
}

impl AudioMapper {
    /// Create a new audio mapper
    pub fn new() -> Self {
        Self { debug: false }
    }

    /// Enable debug logging
    pub fn with_debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// Create audio stream mappings
    pub fn create_mappings(
        &self,
        media_info: &MediaInfo,
        config: &AudioMappingConfig,
    ) -> TrimXResult<Vec<AudioMapping>> {
        if media_info.audio_streams.is_empty() {
            info!("No audio streams found in input");
            return Ok(Vec::new());
        }

        debug!("Creating audio mappings for {} streams", media_info.audio_streams.len());

        let mut mappings = Vec::new();

        if config.include_all {
            // Include all audio streams
            mappings = self.map_all_streams(&media_info.audio_streams, config)?;
        } else if let Some(ref selected_indices) = config.selected_streams {
            // Map specific streams
            mappings = self.map_selected_streams(&media_info.audio_streams, selected_indices, config)?;
        } else if config.auto_select_best {
            // Auto-select best streams
            mappings = self.auto_select_streams(&media_info.audio_streams, config)?;
        } else {
            // Default: select first stream
            if let Some(first_stream) = media_info.audio_streams.first() {
                mappings.push(self.create_mapping(first_stream, 0, 0, config)?);
            }
        }

        info!("Created {} audio stream mappings", mappings.len());

        // Sort mappings by quality score (descending)
        mappings.sort_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap());

        Ok(mappings)
    }

    /// Map all audio streams
    fn map_all_streams(
        &self,
        streams: &[AudioStreamInfo],
        config: &AudioMappingConfig,
    ) -> TrimXResult<Vec<AudioMapping>> {
        let mut mappings = Vec::new();

        for (input_index, stream) in streams.iter().enumerate() {
            let mapping = self.create_mapping(stream, input_index, mappings.len(), config)?;
            mappings.push(mapping);
        }

        Ok(mappings)
    }

    /// Map selected streams by index
    fn map_selected_streams(
        &self,
        streams: &[AudioStreamInfo],
        selected_indices: &[usize],
        config: &AudioMappingConfig,
    ) -> TrimXResult<Vec<AudioMapping>> {
        let mut mappings = Vec::new();

        for &input_index in selected_indices {
            if let Some(stream) = streams.get(input_index) {
                let mapping = self.create_mapping(stream, input_index, mappings.len(), config)?;
                mappings.push(mapping);
            } else {
                warn!("Audio stream index {} not found, skipping", input_index);
            }
        }

        Ok(mappings)
    }

    /// Auto-select best streams based on quality and language
    fn auto_select_streams(
        &self,
        streams: &[AudioStreamInfo],
        config: &AudioMappingConfig,
    ) -> TrimXResult<Vec<AudioMapping>> {
        // Score each stream
        let mut scored_streams: Vec<(usize, &AudioStreamInfo, f64)> = streams
            .iter()
            .enumerate()
            .map(|(index, stream)| {
                let score = self.calculate_quality_score(stream, config);
                (index, stream, score)
            })
            .collect();

        // Sort by score (descending)
        scored_streams.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

        // Select best streams by language groups
        let mut mappings = Vec::new();
        let mut languages_seen = std::collections::HashSet::new();

        for (input_index, stream, score) in scored_streams {
            let language = stream.language.as_deref().unwrap_or("unknown");

            // Include first stream of each language, or if no language preference
            if config.language_preference.is_empty() 
                || !languages_seen.contains(language) 
                || mappings.is_empty() {

                let mapping = AudioMapping {
                    input_index,
                    output_index: mappings.len(),
                    stream_info: stream.clone(),
                    processing_mode: self.determine_processing_mode(stream, config),
                    quality_score: score,
                };

                mappings.push(mapping);
                languages_seen.insert(language.to_string());

                // Limit to reasonable number of streams
                if mappings.len() >= 3 {
                    break;
                }
            }
        }

        Ok(mappings)
    }

    /// Create a single mapping
    fn create_mapping(
        &self,
        stream: &AudioStreamInfo,
        input_index: usize,
        output_index: usize,
        config: &AudioMappingConfig,
    ) -> TrimXResult<AudioMapping> {
        let processing_mode = self.determine_processing_mode(stream, config);
        let quality_score = self.calculate_quality_score(stream, config);

        Ok(AudioMapping {
            input_index,
            output_index,
            stream_info: stream.clone(),
            processing_mode,
            quality_score,
        })
    }

    /// Calculate quality score for stream selection
    fn calculate_quality_score(&self, stream: &AudioStreamInfo, config: &AudioMappingConfig) -> f64 {
        let mut score = 0.0;

        // Codec quality score
        score += match stream.codec.as_str() {
            "flac" | "alac" => 100.0,      // Lossless
            "aac" => 90.0,                 // High quality lossy
            "vorbis" | "opus" => 85.0,     // Good quality lossy
            "mp3" => 70.0,                 // Standard quality
            "ac3" => 60.0,                 // Surround sound
            "dts" => 80.0,                 // High quality surround
            _ => 50.0,                     // Unknown/other
        };

        // Bit rate score
        if let Some(bitrate) = stream.bit_rate {
            score += match bitrate {
                br if br >= 320000 => 20.0,    // High bitrate
                br if br >= 192000 => 15.0,    // Good bitrate
                br if br >= 128000 => 10.0,    // Acceptable bitrate
                _ => 5.0,                      // Low bitrate
            };
        }

        // Channel count score
        score += match stream.channels {
            8 | 7 => 15.0,     // 7.1 surround
            6 => 12.0,         // 5.1 surround
            4 => 8.0,          // Quadraphonic
            2 => 10.0,         // Stereo (most compatible)
            1 => 5.0,          // Mono
            _ => 3.0,          // Other
        };

        // Sample rate score
        score += match stream.sample_rate {
            96000 | 192000 => 10.0,    // High sample rate
            48000 => 8.0,              // Standard professional
            44100 => 7.0,              // CD quality
            _ => 5.0,                  // Other
        };

        // Language preference score
        if let Some(ref language) = stream.language {
            let lang_lower = language.to_lowercase();
            for (index, preferred) in config.language_preference.iter().enumerate() {
                if lang_lower.contains(&preferred.to_lowercase()) {
                    score += 30.0 - (index as f64 * 5.0); // Higher score for preferred languages
                    break;
                }
            }
        }

        score
    }

    /// Determine processing mode for a stream
    fn determine_processing_mode(&self, stream: &AudioStreamInfo, config: &AudioMappingConfig) -> AudioProcessingMode {
        // Check if transcoding is requested
        if let Some(ref target_codec) = config.target_codec {
            if target_codec != &stream.codec {
                return AudioProcessingMode::Transcode {
                    target_codec: target_codec.clone(),
                    target_bitrate: config.target_bitrate,
                    target_channels: config.channel_config.as_ref().map(|c| c.channels),
                    target_sample_rate: None, // Keep original sample rate
                };
            }
        }

        // Check if channel configuration change is needed
        if let Some(ref channel_config) = config.channel_config {
            if channel_config.channels != stream.channels {
                return AudioProcessingMode::Transcode {
                    target_codec: stream.codec.clone(), // Keep same codec
                    target_bitrate: config.target_bitrate,
                    target_channels: Some(channel_config.channels),
                    target_sample_rate: None,
                };
            }
        }

        // Check if bitrate change is needed
        if let Some(target_bitrate) = config.target_bitrate {
            if let Some(current_bitrate) = stream.bit_rate {
                if target_bitrate != current_bitrate {
                    return AudioProcessingMode::Reencode;
                }
            }
        }

        // Default to copy if no changes needed
        AudioProcessingMode::Copy
    }

    /// Validate audio mapping configuration
    pub fn validate_config(&self, config: &AudioMappingConfig) -> TrimXResult<()> {
        // Check for conflicting options
        if config.include_all && config.selected_streams.is_some() {
            return Err(TrimXError::ClippingError {
                message: "Cannot specify both include_all and selected_streams".to_string()
            });
        }

        // Validate selected stream indices
        if let Some(ref indices) = config.selected_streams {
            if indices.is_empty() {
                return Err(TrimXError::ClippingError {
                    message: "Selected streams list cannot be empty".to_string()
                });
            }
        }

        // Validate channel configuration
        if let Some(ref channel_config) = config.channel_config {
            if channel_config.channels == 0 || channel_config.channels > 8 {
                return Err(TrimXError::ClippingError {
                    message: format!("Invalid channel count: {}", channel_config.channels)
                });
            }
        }

        Ok(())
    }

    /// Get supported audio codecs for container format
    pub fn get_supported_codecs(&self, container_format: &str) -> Vec<String> {
        match container_format.to_lowercase().as_str() {
            "mp4" | "m4v" | "mov" => vec![
                "aac".to_string(),
                "mp3".to_string(),
                "ac3".to_string(),
            ],
            "mkv" => vec![
                "aac".to_string(),
                "mp3".to_string(),
                "vorbis".to_string(),
                "opus".to_string(),
                "flac".to_string(),
                "ac3".to_string(),
                "dts".to_string(),
            ],
            "webm" => vec![
                "vorbis".to_string(),
                "opus".to_string(),
            ],
            "avi" => vec![
                "mp3".to_string(),
                "ac3".to_string(),
                "pcm".to_string(),
            ],
            _ => vec!["aac".to_string()], // Default fallback
        }
    }

    /// Generate mapping summary report
    pub fn generate_report(&self, mappings: &[AudioMapping]) -> String {
        let mut report = String::new();
        
        report.push_str("Audio Stream Mapping Report:\n");
        report.push_str(&format!("  Total mappings: {}\n", mappings.len()));
        
        for (index, mapping) in mappings.iter().enumerate() {
            report.push_str(&format!("\n  Stream {}:\n", index));
            report.push_str(&format!("    Input Index: {}\n", mapping.input_index));
            report.push_str(&format!("    Codec: {}\n", mapping.stream_info.codec));
            report.push_str(&format!("    Channels: {}\n", mapping.stream_info.channels));
            report.push_str(&format!("    Sample Rate: {} Hz\n", mapping.stream_info.sample_rate));
            
            if let Some(bitrate) = mapping.stream_info.bit_rate {
                report.push_str(&format!("    Bitrate: {} bps\n", bitrate));
            }
            
            if let Some(ref language) = mapping.stream_info.language {
                report.push_str(&format!("    Language: {}\n", language));
            }
            
            report.push_str(&format!("    Processing: {:?}\n", mapping.processing_mode));
            report.push_str(&format!("    Quality Score: {:.1}\n", mapping.quality_score));
        }
        
        report
    }
}

//! Stream mapping utilities

use crate::streams::{StreamMapping, VideoStreamMapping, AudioStreamMapping, SubtitleStreamMapping};
use crate::streams::{VideoProcessingMode, AudioProcessingMode, SubtitleProcessingMode};
use crate::probe::MediaInfo;
use crate::error::{TrimXError, TrimXResult};

/// Stream mapper for creating stream mappings
pub struct StreamMapper;

impl StreamMapper {
    /// Create a new stream mapper
    pub fn new() -> Self {
        Self
    }

    /// Create stream mapping from media info
    pub fn create_mapping(
        &self,
        media_info: &MediaInfo,
        no_audio: bool,
        no_subs: bool,
    ) -> TrimXResult<StreamMapping> {
        let mut mapping = StreamMapping {
            video: None,
            audio: vec![],
            subtitles: vec![],
        };

        // Map video stream (take the first one)
        if let Some(video_stream) = media_info.video_streams.first() {
            mapping.video = Some(VideoStreamMapping {
                input_index: video_stream.index,
                output_index: 0,
                mode: VideoProcessingMode::Copy, // Default to copy
            });
        }

        // Map audio streams
        if !no_audio {
            for (i, audio_stream) in media_info.audio_streams.iter().enumerate() {
                mapping.audio.push(AudioStreamMapping {
                    input_index: audio_stream.index,
                    output_index: i,
                    mode: AudioProcessingMode::Copy, // Default to copy
                });
            }
        }

        // Map subtitle streams
        if !no_subs {
            for (i, sub_stream) in media_info.subtitle_streams.iter().enumerate() {
                mapping.subtitles.push(SubtitleStreamMapping {
                    input_index: sub_stream.index,
                    output_index: i,
                    mode: SubtitleProcessingMode::Copy, // Default to copy
                });
            }
        }

        Ok(mapping)
    }

    /// Validate stream mapping
    pub fn validate_mapping(&self, mapping: &StreamMapping) -> TrimXResult<()> {
        // Check for duplicate output indices
        let mut output_indices = std::collections::HashSet::new();

        if let Some(video) = &mapping.video {
            if !output_indices.insert(video.output_index) {
                return Err(TrimXError::StreamError {
                    message: "Duplicate output stream index".to_string(),
                });
            }
        }

        for audio in &mapping.audio {
            if !output_indices.insert(audio.output_index) {
                return Err(TrimXError::StreamError {
                    message: "Duplicate output stream index".to_string(),
                });
            }
        }

        for subtitle in &mapping.subtitles {
            if !output_indices.insert(subtitle.output_index) {
                return Err(TrimXError::StreamError {
                    message: "Duplicate output stream index".to_string(),
                });
            }
        }

        Ok(())
    }
}

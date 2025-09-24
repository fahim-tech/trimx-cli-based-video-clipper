// Inspect interactor - Orchestrates media file inspection use case

use crate::domain::model::*;
use crate::domain::errors::*;
use crate::ports::*;

/// Interactor for media file inspection use case
pub struct InspectInteractor {
    probe_port: Box<dyn ProbePort>,
    fs_port: Box<dyn FsPort>,
    log_port: Box<dyn LogPort>,
}

impl InspectInteractor {
    /// Create new inspect interactor with injected ports
    pub fn new(
        probe_port: Box<dyn ProbePort>,
        fs_port: Box<dyn FsPort>,
        log_port: Box<dyn LogPort>,
    ) -> Self {
        Self {
            probe_port,
            fs_port,
            log_port,
        }
    }
    
    /// Execute media file inspection
    pub async fn execute(&self, request: InspectRequest) -> Result<InspectResponse, DomainError> {
        // Log start of operation
        self.log_port.info(&format!("Starting media file inspection for: {}", request.input_file));
        
        // Validate input file
        if !self.fs_port.file_exists(&request.input_file).await? {
            return Err(DomainError::FsFail(format!("Input file does not exist: {}", request.input_file)));
        }
        
        // Get file metadata
        let file_metadata = self.fs_port.get_file_metadata(&request.input_file).await?;
        
        // Probe media file
        let media_info = self.probe_port.probe_media(&request.input_file).await?;
        self.log_port.info(&format!("Media file probed successfully: {} streams", media_info.total_streams()));
        
        // Get additional stream information if requested
        let mut detailed_streams = Vec::new();
        if request.include_streams {
            detailed_streams = self.get_detailed_stream_info(&request.input_file, &media_info).await?;
        }
        
        // Log completion
        self.log_port.info("Media file inspection completed successfully");
        
        Ok(InspectResponse {
            media_info,
            file_metadata,
            detailed_streams,
            format_supported: true, // Would be determined by probe_port
        })
    }
    
    /// Get detailed stream information
    async fn get_detailed_stream_info(
        &self, 
        file_path: &str, 
        media_info: &MediaInfo
    ) -> Result<Vec<DetailedStreamInfo>, DomainError> {
        let mut detailed_streams = Vec::new();
        
        // Get video stream details
        for (index, _) in media_info.video_streams.iter().enumerate() {
            match self.probe_port.get_video_stream_info(file_path, index).await {
                Ok(video_stream) => {
                    detailed_streams.push(DetailedStreamInfo::Video(video_stream));
                },
                Err(e) => {
                    self.log_port.warn(&format!("Failed to get video stream {} details: {}", index, e));
                }
            }
        }
        
        // Get audio stream details
        for (index, _) in media_info.audio_streams.iter().enumerate() {
            match self.probe_port.get_audio_stream_info(file_path, index).await {
                Ok(audio_stream) => {
                    detailed_streams.push(DetailedStreamInfo::Audio(audio_stream));
                },
                Err(e) => {
                    self.log_port.warn(&format!("Failed to get audio stream {} details: {}", index, e));
                }
            }
        }
        
        // Get subtitle stream details
        for (index, _) in media_info.subtitle_streams.iter().enumerate() {
            match self.probe_port.get_subtitle_stream_info(file_path, index).await {
                Ok(subtitle_stream) => {
                    detailed_streams.push(DetailedStreamInfo::Subtitle(subtitle_stream));
                },
                Err(e) => {
                    self.log_port.warn(&format!("Failed to get subtitle stream {} details: {}", index, e));
                }
            }
        }
        
        Ok(detailed_streams)
    }
    
    /// Generate human-readable report
    pub fn generate_report(&self, response: &InspectResponse) -> String {
        let mut report = String::new();
        
        // File information
        report.push_str("=== Media File Information ===\n");
        report.push_str(&format!("File: {}\n", response.media_info.format));
        report.push_str(&format!("Duration: {}\n", response.media_info.duration));
        report.push_str(&format!("File Size: {} bytes ({:.2} MB)\n", 
            response.file_metadata.size, 
            response.file_metadata.size as f64 / 1_048_576.0));
        
        if let Some(bit_rate) = response.media_info.bit_rate {
            report.push_str(&format!("Bit Rate: {} bps ({:.2} Mbps)\n", 
                bit_rate, bit_rate as f64 / 1_000_000.0));
        }
        
        // Stream information
        report.push_str("\n=== Stream Information ===\n");
        report.push_str(&format!("Total Streams: {}\n", response.media_info.total_streams()));
        report.push_str(&format!("Video Streams: {}\n", response.media_info.video_streams.len()));
        report.push_str(&format!("Audio Streams: {}\n", response.media_info.audio_streams.len()));
        report.push_str(&format!("Subtitle Streams: {}\n", response.media_info.subtitle_streams.len()));
        
        // Detailed stream information
        for (index, stream) in response.detailed_streams.iter().enumerate() {
            report.push_str(&format!("\n--- Stream {} ---\n", index));
            match stream {
                DetailedStreamInfo::Video(video) => {
                    report.push_str(&format!("Type: Video\n"));
                    report.push_str(&format!("Codec: {}\n", video.codec));
                    report.push_str(&format!("Resolution: {}x{}\n", video.width, video.height));
                    report.push_str(&format!("Frame Rate: {:.2} fps\n", video.frame_rate));
                    if let Some(pixel_format) = &video.pixel_format {
                        report.push_str(&format!("Pixel Format: {}\n", pixel_format));
                    }
                    if let Some(color_space) = &video.color_space {
                        report.push_str(&format!("Color Space: {}\n", color_space));
                    }
                    if let Some(rotation) = video.rotation {
                        report.push_str(&format!("Rotation: {}°\n", rotation));
                    }
                },
                DetailedStreamInfo::Audio(audio) => {
                    report.push_str(&format!("Type: Audio\n"));
                    report.push_str(&format!("Codec: {}\n", audio.codec));
                    report.push_str(&format!("Sample Rate: {} Hz\n", audio.sample_rate));
                    report.push_str(&format!("Channels: {}\n", audio.channels));
                    if let Some(language) = &audio.language {
                        report.push_str(&format!("Language: {}\n", language));
                    }
                },
                DetailedStreamInfo::Subtitle(subtitle) => {
                    report.push_str(&format!("Type: Subtitle\n"));
                    report.push_str(&format!("Codec: {}\n", subtitle.codec));
                    if let Some(language) = &subtitle.language {
                        report.push_str(&format!("Language: {}\n", language));
                    }
                    report.push_str(&format!("Forced: {}\n", subtitle.forced));
                    report.push_str(&format!("Default: {}\n", subtitle.default));
                }
            }
        }
        
        report
    }
}

/// Request for media file inspection
#[derive(Debug, Clone)]
pub struct InspectRequest {
    pub input_file: String,
    pub include_streams: bool,
    pub include_metadata: bool,
}

impl InspectRequest {
    /// Create new inspect request
    pub fn new(input_file: String) -> Self {
        Self {
            input_file,
            include_streams: true,
            include_metadata: true,
        }
    }
    
    /// Create new inspect request with options
    pub fn with_options(
        input_file: String,
        include_streams: bool,
        include_metadata: bool,
    ) -> Self {
        Self {
            input_file,
            include_streams,
            include_metadata,
        }
    }
}

/// Response from media file inspection
#[derive(Debug, Clone)]
pub struct InspectResponse {
    pub media_info: MediaInfo,
    pub file_metadata: crate::ports::FileMetadata,
    pub detailed_streams: Vec<DetailedStreamInfo>,
    pub format_supported: bool,
}

/// Detailed stream information for inspection
#[derive(Debug, Clone)]
pub enum DetailedStreamInfo {
    Video(VideoStreamInfo),
    Audio(AudioStreamInfo),
    Subtitle(SubtitleStreamInfo),
}
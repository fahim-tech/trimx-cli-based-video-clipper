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
        
        // Additional stream information could be gathered here if needed
        
        // Log completion
        self.log_port.info("Media file inspection completed successfully");
        
        Ok(InspectResponse::success(media_info))
    }
    
}

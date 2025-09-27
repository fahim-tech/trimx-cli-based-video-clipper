use std::sync::Arc;

use crate::adapters::{
    FsWindowsAdapter, MockExecutionAdapter, MockProbeAdapter, TomlConfigAdapter, TracingLogAdapter,
};
use crate::app::{
    clip_interactor::ClipInteractor, inspect_interactor::InspectInteractor,
    verify_interactor::VerifyInteractor,
};
use crate::domain::errors::DomainError;
use crate::ports::{ConfigPort, ExecutePort, FsPort, LogPort, ProbePort};

pub trait AppContainer: Send + Sync {
    fn clip_interactor(&self) -> Arc<ClipInteractor>;
    fn inspect_interactor(&self) -> Arc<InspectInteractor>;
    fn verify_interactor(&self) -> Arc<VerifyInteractor>;
}

pub struct DefaultAppContainer {
    clip_interactor: Arc<ClipInteractor>,
    inspect_interactor: Arc<InspectInteractor>,
    verify_interactor: Arc<VerifyInteractor>,
}

impl DefaultAppContainer {
    pub fn new() -> Result<Self, DomainError> {
        let probe_port = Arc::new(MockProbeAdapter::new()?);
        let execute_port = Arc::new(MockExecutionAdapter::new()?);
        let fs_port = Arc::new(FsWindowsAdapter::new()?);
        let config_port = Arc::new(TomlConfigAdapter::new()?);
        let log_port = Arc::new(TracingLogAdapter::new()?);

        let clip_interactor = Arc::new(ClipInteractor::new(
            Arc::clone(&probe_port) as Arc<dyn ProbePort>,
            Arc::clone(&execute_port) as Arc<dyn ExecutePort>,
            Arc::clone(&fs_port) as Arc<dyn FsPort>,
            Arc::clone(&config_port) as Arc<dyn ConfigPort>,
            Arc::clone(&log_port) as Arc<dyn LogPort>,
        ));

        let inspect_interactor = Arc::new(InspectInteractor::new(
            Arc::clone(&probe_port) as Arc<dyn ProbePort>,
            Arc::clone(&fs_port) as Arc<dyn FsPort>,
            Arc::clone(&log_port) as Arc<dyn LogPort>,
        ));

        let verify_interactor = Arc::new(VerifyInteractor::new(
            Arc::clone(&probe_port) as Arc<dyn ProbePort>,
            Arc::clone(&fs_port) as Arc<dyn FsPort>,
            Arc::clone(&log_port) as Arc<dyn LogPort>,
        ));

        Ok(Self {
            clip_interactor,
            inspect_interactor,
            verify_interactor,
        })
    }
}

impl AppContainer for DefaultAppContainer {
    fn clip_interactor(&self) -> Arc<ClipInteractor> {
        Arc::clone(&self.clip_interactor)
    }

    fn inspect_interactor(&self) -> Arc<InspectInteractor> {
        Arc::clone(&self.inspect_interactor)
    }

    fn verify_interactor(&self) -> Arc<VerifyInteractor> {
        Arc::clone(&self.verify_interactor)
    }
}

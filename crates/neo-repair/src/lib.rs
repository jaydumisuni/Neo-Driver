//! Phase 21 Repair & Windows Features.
//!
//! Public callers may inspect current Windows servicing state and prepare typed
//! transaction-bound operations. Machine mutation remains capability-gated and
//! is intended to be issued only by the trusted MCP/RPC service path.

mod error;
#[cfg(any(windows, test))]
mod executor;
#[cfg(any(windows, test))]
mod host;
#[cfg(any(windows, test))]
mod inspection;
mod model;
mod operation;
#[cfg(any(windows, test))]
mod parse;
#[cfg(any(windows, test))]
mod plan;
#[cfg(any(windows, test))]
pub mod rpc;
#[cfg(any(windows, test))]
mod session_store;

pub use error::RepairError;
#[cfg(any(windows, test))]
pub use executor::{RepairExecutionSession, RepairExecutorCapability};
pub use model::{
    BoundedCommandEvidence, ComponentStoreObservation, ComponentStoreState, FeatureDesiredState,
    RepairHealthInspectionReport, RepairInspectionReport, RepairTarget, SupportedWindowsFeature,
    SystemFileObservation, SystemFileState, WindowsFeatureObservation, WindowsFeatureState,
    WindowsFeaturesInspectionReport, MAX_REPAIR_EVIDENCE_BYTES,
};
pub use operation::{RepairBaseline, RepairOperation};
#[cfg(any(windows, test))]
pub use plan::RepairExecutionPlan;

#[cfg(windows)]
pub fn inspect_windows_repair_health() -> Result<RepairHealthInspectionReport, RepairError> {
    let host = host::WindowsRepairHost::new()?;
    inspection::inspect_repair_health_with_host(&host)
}

#[cfg(not(windows))]
pub fn inspect_windows_repair_health() -> Result<RepairHealthInspectionReport, RepairError> {
    Err(RepairError::UnsupportedPlatform)
}

#[cfg(windows)]
pub fn inspect_windows_features() -> Result<WindowsFeaturesInspectionReport, RepairError> {
    let host = host::WindowsRepairHost::new()?;
    inspection::inspect_features_with_host(&host)
}

#[cfg(not(windows))]
pub fn inspect_windows_features() -> Result<WindowsFeaturesInspectionReport, RepairError> {
    Err(RepairError::UnsupportedPlatform)
}

#[cfg(windows)]
pub fn inspect_windows() -> Result<RepairInspectionReport, RepairError> {
    let host = host::WindowsRepairHost::new()?;
    inspection::inspect_with_host(&host)
}

#[cfg(not(windows))]
pub fn inspect_windows() -> Result<RepairInspectionReport, RepairError> {
    Err(RepairError::UnsupportedPlatform)
}

#[cfg(windows)]
pub fn prepare_windows_operation(
    operation: RepairOperation,
    mission_id: impl Into<String>,
) -> Result<RepairExecutionSession, RepairError> {
    let host = host::WindowsRepairHost::new()?;
    RepairExecutionSession::prepare_with_host(operation, mission_id, &host)
}

#[cfg(test)]
mod review_tests;

//! Phase 21 Repair & Windows Features.
//!
//! Public callers may inspect current Windows servicing state and prepare typed
//! transaction-bound operations. Machine mutation remains capability-gated and
//! is intended to be issued only by the trusted MCP/RPC service path.

mod error;
mod executor;
mod host;
mod inspection;
mod model;
mod operation;
mod parse;
mod plan;
mod session_store;

pub use error::RepairError;
pub use executor::{RepairExecutionSession, RepairExecutorCapability};
pub use model::{
    BoundedCommandEvidence, ComponentStoreObservation, ComponentStoreState, FeatureDesiredState,
    RepairInspectionReport, RepairTarget, SupportedWindowsFeature, SystemFileObservation,
    SystemFileState, WindowsFeatureObservation, WindowsFeatureState, MAX_REPAIR_EVIDENCE_BYTES,
};
pub use operation::{RepairBaseline, RepairOperation};
pub use plan::RepairExecutionPlan;

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

#[cfg(not(windows))]
pub fn prepare_windows_operation(
    _operation: RepairOperation,
    _mission_id: impl Into<String>,
) -> Result<RepairExecutionSession, RepairError> {
    Err(RepairError::UnsupportedPlatform)
}

#[cfg(test)]
mod review_tests;

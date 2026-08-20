//! Phase 22 Driver Store / PnP repair assessment foundation.
//!
//! This crate is deliberately read-only. It consumes the exact device and Driver Store
//! evidence already owned by `neo-device` and `neo-driverstore`, then derives a bounded
//! repair candidate. It does not issue driver installation, device re-enumeration,
//! Driver Store deletion, or any other machine-changing authority.

mod assessment;
mod error;
mod model;

pub use error::DriverRepairError;
pub use model::{
    DriverRepairAssessment, DriverRepairAssessmentReport, DriverRepairDeviceEvidence,
    DriverRepairEvidence, DriverRepairRoute, DriverRepairState, PnpStatusEvidence,
};

#[cfg(windows)]
pub fn inspect_windows_driver_repair() -> Result<DriverRepairAssessmentReport, DriverRepairError> {
    use neo_driverstore::WindowsDriverHost;

    let host = WindowsDriverHost;
    assessment::capture_and_assess_with_host(&host)
}

#[cfg(not(windows))]
pub fn inspect_windows_driver_repair() -> Result<DriverRepairAssessmentReport, DriverRepairError> {
    Err(DriverRepairError::UnsupportedPlatform)
}

pub fn assess_driver_repair_evidence(
    evidence: DriverRepairEvidence,
) -> Result<DriverRepairAssessmentReport, DriverRepairError> {
    assessment::assess(evidence)
}

#[cfg(test)]
mod tests;

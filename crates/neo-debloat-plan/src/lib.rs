//! Phase 15 exact AppX mutation-plan and rollback-readiness boundary.
//!
//! Phase 15 is still read-only. It composes the proven Phase 14 presence evidence with a native
//! PackageManager exact-identity inventory, then prepares one current-user Debloat transaction
//! only when the exact package and every direct dependency have matching provisioned staged
//! identities suitable for a future RegisterPackageByFullName rollback path. No removal,
//! registration, deprovisioning, provisioning, capability issuance, CLI write command, plugin,
//! or MCP/RPC debloat authority exists in this crate.

mod error;
mod model;
mod plan;
#[cfg(target_os = "windows")]
mod windows;

pub use error::DebloatPlanError;
pub use model::{
    DebloatPreparedStep, DebloatPreparedTransaction, DebloatRestoreRoute, ExactAppxInventory,
    ExactPackageDependency, ExactPackageIdentity,
};
pub use plan::prepare_debloat_transaction_from_evidence;

use neo_debloat::{DebloatCatalogue, DebloatProfile};

pub fn scan_windows_exact_appx_inventory() -> Result<ExactAppxInventory, DebloatPlanError> {
    #[cfg(target_os = "windows")]
    {
        windows::scan_native_inventory()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(DebloatPlanError::UnsupportedPlatform)
    }
}

pub fn prepare_windows_debloat_transaction(
    catalogue: &DebloatCatalogue,
    profile: DebloatProfile,
    selected_ids: &[String],
    mission_id: impl Into<String>,
) -> Result<DebloatPreparedTransaction, DebloatPlanError> {
    #[cfg(target_os = "windows")]
    {
        let phase14 = neo_debloat_probe::scan_current_debloat_evidence(catalogue)?;
        let exact = windows::scan_native_inventory()?;
        prepare_debloat_transaction_from_evidence(
            catalogue,
            &phase14.evidence,
            &exact,
            profile,
            selected_ids,
            mission_id,
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (catalogue, profile, selected_ids, mission_id.into());
        Err(DebloatPlanError::UnsupportedPlatform)
    }
}

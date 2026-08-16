//! Phase 17 completed-removal history and post-success restore-readiness boundary.
//!
//! Phase 17 does not mutate AppX state. It turns a successfully completed Phase 16 current-user
//! removal into a versioned, fingerprinted history receipt that retains the validated completed
//! Phase 4 checkpoint and original exact main/dependency identities. Later, Neo may re-probe the
//! current native AppX inventory and prepare a fresh inverse Phase 4 transaction only if the exact
//! local staged main/dependency restore route remains valid and no conflicting current-user version
//! has appeared. Post-success restore execution remains separately gated.

mod error;
mod model;
mod plan;

pub use error::DebloatHistoryError;
pub use model::{
    DebloatRemovalReceipt, DebloatRestorePreparedStep, DebloatRestorePreparedTransaction,
    HistoryRestoreRoute, DEBLOAT_REMOVAL_RECEIPT_SCHEMA_VERSION,
};
pub use plan::prepare_restore_from_inventory;

use model::appx_target;
use neo_debloat_executor::DebloatExecutionSession;
#[cfg(target_os = "windows")]
use neo_debloat_plan::scan_windows_exact_appx_inventory;
use neo_debloat_plan::{DebloatRestoreRoute, ExactPackageDependency, ExactPackageIdentity};
use neo_transaction::{CapturedValue, TransactionStage};
#[cfg(test)]
use neo_transaction::TransactionCheckpoint;

pub fn receipt_from_completed_execution(
    session: &DebloatExecutionSession,
) -> Result<DebloatRemovalReceipt, DebloatHistoryError> {
    if session.stage() != TransactionStage::Complete {
        return Err(DebloatHistoryError::IncompleteRemoval(format!(
            "Phase 16 session is {:?}, not Complete",
            session.stage()
        )));
    }
    let transaction = session.plan().transaction();
    if session.checkpoint().plan_fingerprint() != transaction.fingerprint()?
        || session.checkpoint().plan().fingerprint()? != transaction.fingerprint()?
    {
        return Err(DebloatHistoryError::IncompleteRemoval(
            "Phase 16 execution/checkpoint transaction fingerprint continuity failed".to_string(),
        ));
    }
    let step = session.plan().step();
    let baseline = session.checkpoint().baseline().ok_or_else(|| {
        DebloatHistoryError::IncompleteRemoval("completed Phase 16 baseline is missing".to_string())
    })?;

    let main_target = appx_target(step.package_full_name());
    let main_json = match baseline.get(&main_target) {
        Some(CapturedValue::Present(value)) => value,
        _ => {
            return Err(DebloatHistoryError::IncompleteRemoval(
                "completed Phase 16 main baseline is not Present".to_string(),
            ))
        }
    };
    let main: ExactPackageIdentity = serde_json::from_str(main_json)?;
    if !main
        .full_name
        .eq_ignore_ascii_case(step.package_full_name())
        || !main
            .family_name
            .eq_ignore_ascii_case(step.package_family_name())
    {
        return Err(DebloatHistoryError::IncompleteRemoval(
            "completed Phase 16 main baseline differs from execution identity".to_string(),
        ));
    }

    let mut dependencies = Vec::with_capacity(step.dependency_full_names().len());
    for dependency_full_name in step.dependency_full_names() {
        let target = appx_target(dependency_full_name);
        let json = match baseline.get(&target) {
            Some(CapturedValue::Present(value)) => value,
            _ => {
                return Err(DebloatHistoryError::IncompleteRemoval(format!(
                    "completed Phase 16 dependency baseline {dependency_full_name} is not Present"
                )))
            }
        };
        let dependency: ExactPackageDependency = serde_json::from_str(json)?;
        if !dependency
            .full_name
            .eq_ignore_ascii_case(dependency_full_name)
        {
            return Err(DebloatHistoryError::IncompleteRemoval(format!(
                "completed Phase 16 dependency baseline differs from {dependency_full_name}"
            )));
        }
        dependencies.push(dependency);
    }

    let restore = match step.restore() {
        DebloatRestoreRoute::RegisterByFullNameFromProvisioned {
            package_full_name,
            package_family_name,
            dependency_full_names,
        } => HistoryRestoreRoute::new(
            package_full_name.clone(),
            package_family_name.clone(),
            dependency_full_names.clone(),
        ),
    };

    DebloatRemovalReceipt::new(
        transaction.transaction_id().to_string(),
        transaction.fingerprint()?,
        transaction.mission_id().to_string(),
        step.debloat_id().to_string(),
        step.package_id().to_string(),
        main,
        dependencies,
        restore,
        session.checkpoint().clone(),
    )
}

pub fn prepare_windows_restore_from_receipt(
    receipt: &DebloatRemovalReceipt,
    mission_id: impl Into<String>,
) -> Result<DebloatRestorePreparedTransaction, DebloatHistoryError> {
    #[cfg(target_os = "windows")]
    {
        let inventory = scan_windows_exact_appx_inventory()?;
        prepare_restore_from_inventory(receipt, &inventory, mission_id)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (receipt, mission_id.into());
        Err(DebloatHistoryError::UnsupportedPlatform)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) fn receipt_from_completed_checkpoint_for_tests(
    debloat_id: &str,
    package_id: &str,
    main: ExactPackageIdentity,
    dependencies: Vec<ExactPackageDependency>,
    checkpoint: TransactionCheckpoint,
) -> Result<DebloatRemovalReceipt, DebloatHistoryError> {
    if checkpoint.stage() != TransactionStage::Complete {
        return Err(DebloatHistoryError::IncompleteRemoval(
            "synthetic source checkpoint is not Complete".to_string(),
        ));
    }
    let transaction = checkpoint.plan();
    DebloatRemovalReceipt::new(
        transaction.transaction_id().to_string(),
        transaction.fingerprint()?,
        transaction.mission_id().to_string(),
        debloat_id.to_string(),
        package_id.to_string(),
        main.clone(),
        dependencies.clone(),
        HistoryRestoreRoute::new(
            main.full_name,
            main.family_name,
            dependencies
                .iter()
                .map(|dependency| dependency.full_name.clone())
                .collect(),
        ),
        checkpoint,
    )
}

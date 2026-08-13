use crate::model::{canonical_arch, runtime_baseline_value};
use crate::{RuntimeExecutionOperation, RuntimeExecutionPlan, RuntimeExecutorError};
use neo_catalogue::{Catalogue, PackageKind};
use neo_core::{ActionKind, EvidenceVerdict};
use neo_runtime::{
    assess_runtime_profile, RuntimeComponent, RuntimeInventory, RuntimePolicy, RuntimeProfile,
    RuntimeState,
};
use neo_transaction::{
    BaselineSnapshot, CapturedState, CapturedValue, RollbackPlan, TransactionAction,
    TransactionPlan, VerificationPredicate,
};
use neo_vault::{Sha256Digest, VaultLayout, VaultMode, VaultSegment, VaultStore};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PreparedRuntimeExecution {
    pub plan: RuntimeExecutionPlan,
    pub transaction_plan: TransactionPlan,
    pub baseline: BaselineSnapshot,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_runtime_execution(
    mission_id: impl Into<String>,
    transaction_id: impl Into<String>,
    profile: RuntimeProfile,
    component: RuntimeComponent,
    inventory: &RuntimeInventory,
    catalogue: &Catalogue,
    policy: &RuntimePolicy,
    application_root: PathBuf,
    vault_mode: VaultMode,
) -> Result<PreparedRuntimeExecution, RuntimeExecutorError> {
    let mission_id = mission_id.into();
    let transaction_id = transaction_id.into();
    if mission_id.trim().is_empty() {
        return Err(RuntimeExecutorError::MissingMissionId);
    }
    if transaction_id.trim().is_empty() {
        return Err(RuntimeExecutorError::MissingTransactionId);
    }

    inventory
        .validate()
        .map_err(|error| RuntimeExecutorError::Assessment(error.to_string()))?;
    catalogue
        .validate()
        .map_err(|error| RuntimeExecutorError::Catalogue(error.to_string()))?;
    let assessment = assess_runtime_profile(profile, inventory, catalogue, policy)
        .map_err(|error| RuntimeExecutorError::Assessment(error.to_string()))?;
    let recommendation = assessment
        .recommendations
        .iter()
        .find(|recommendation| recommendation.component == component)
        .ok_or(RuntimeExecutorError::MissingRecommendation(component))?;
    if recommendation.verdict != EvidenceVerdict::Certified {
        return Err(RuntimeExecutorError::RecommendationNotCertified { component });
    }
    let action = recommendation
        .action
        .clone()
        .ok_or(RuntimeExecutorError::MissingCertifiedAction { component })?;
    let package_id = recommendation
        .package_id
        .as_deref()
        .ok_or(RuntimeExecutorError::MissingPackageId { component })?;
    let package = catalogue
        .packages
        .iter()
        .find(|package| package.package_id == package_id)
        .ok_or_else(|| RuntimeExecutorError::PackageNotFound(package_id.to_string()))?;
    if package.kind != PackageKind::Runtime {
        return Err(RuntimeExecutorError::PackageNotRuntime(
            package.package_id.clone(),
        ));
    }
    if !package.dependencies.is_empty() || !package.conflicts.is_empty() {
        return Err(RuntimeExecutorError::DependencyClosureRequired(
            package.package_id.clone(),
        ));
    }
    if package.security.changes_boot_or_security_state() {
        return Err(RuntimeExecutorError::SecurityMutationBlocked(
            package.package_id.clone(),
        ));
    }
    let execution = package
        .runtime_execution
        .clone()
        .ok_or_else(|| RuntimeExecutorError::MissingExecutionSpec(package.package_id.clone()))?;
    execution
        .validate()
        .map_err(|error| RuntimeExecutorError::Catalogue(error.to_string()))?;

    let baseline = inventory
        .observations
        .iter()
        .find(|observation| observation.component == component)
        .cloned()
        .ok_or(RuntimeExecutorError::MissingObservation { component })?;
    let operation = match baseline.state {
        RuntimeState::Missing => RuntimeExecutionOperation::Install,
        RuntimeState::Broken | RuntimeState::Partial => RuntimeExecutionOperation::Repair,
        other => {
            return Err(RuntimeExecutorError::OperationStateMismatch {
                operation: "runtime mutation",
                state: other,
            })
        }
    };
    let expected_action_kind = match operation {
        RuntimeExecutionOperation::Install => ActionKind::RuntimeInstall,
        RuntimeExecutionOperation::Repair => ActionKind::RuntimeRepair,
    };
    if action.kind != expected_action_kind {
        return Err(RuntimeExecutorError::ActionMismatch(format!(
            "Phase 6 returned {:?} for {:?}",
            action.kind, operation
        )));
    }
    if operation == RuntimeExecutionOperation::Repair && execution.repair_args.is_none() {
        return Err(RuntimeExecutorError::MissingRepairArguments(
            package.package_id.clone(),
        ));
    }

    let package_id = VaultSegment::new(&package.package_id)?;
    let package_version = VaultSegment::new(&package.version)?;
    let package_sha256 = Sha256Digest::new(&package.provenance.sha256)?;
    let architecture = canonical_arch(&inventory.architecture).ok_or_else(|| {
        RuntimeExecutorError::InvalidPlan(format!(
            "unsupported host architecture {}",
            inventory.architecture
        ))
    })?;

    let plan = RuntimeExecutionPlan {
        mission_id,
        transaction_id,
        profile,
        component,
        operation,
        package_kind: package.kind,
        package_id,
        package_version,
        package_sha256,
        package_dependencies: package.dependencies.clone(),
        package_conflicts: package.conflicts.clone(),
        package_security: package.security.clone(),
        execution,
        vault_mode,
        application_root,
        windows_build: inventory.windows_build,
        architecture,
        baseline,
        action,
    };
    plan.validate()?;

    let layout = VaultLayout::new(plan.vault_mode, plan.application_root.clone())?;
    let store = VaultStore::new(layout);
    let payload = plan.payload_path()?;
    store.verify_pack(&payload, &plan.package_sha256)?;

    let transaction_plan = transaction_plan_for(&plan)?;
    let baseline = BaselineSnapshot::for_plan(
        &transaction_plan,
        vec![CapturedState {
            target: plan.state_target(),
            value: CapturedValue::Present(runtime_baseline_value(&plan.baseline)),
        }],
    )?;

    Ok(PreparedRuntimeExecution {
        plan,
        transaction_plan,
        baseline,
    })
}

pub(crate) fn transaction_plan_for(
    plan: &RuntimeExecutionPlan,
) -> Result<TransactionPlan, RuntimeExecutorError> {
    plan.validate()?;
    let target = plan.state_target();
    let postcondition = VerificationPredicate {
        id: format!("verify.{}", plan.action.id),
        target: target.clone(),
        expectation: plan.verification_expectation(),
        required: true,
    };
    Ok(TransactionPlan::new(
        plan.transaction_id.clone(),
        1,
        plan.mission_id.clone(),
        vec![TransactionAction {
            action: plan.action.clone(),
            snapshot_targets: vec![target],
            postconditions: vec![postcondition],
            rollback: RollbackPlan::Irreversible {
                reason: "Phase 8 has no proven generic runtime uninstall/restoration path; exact baseline is captured for history and drift detection only.".to_string(),
            },
        }],
    )?)
}

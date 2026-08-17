use crate::DebloatRestoreExecutionError;
use neo_core::{ActionKind, EvidenceVerdict, RebootRequirement, RecommendationState, RiskLevel};
use neo_debloat_history::{
    DebloatRestorePreparedStep, DebloatRestorePreparedTransaction, HistoryRestoreRoute,
};
use neo_debloat_plan::{ExactPackageDependency, ExactPackageIdentity};
use neo_transaction::{TransactionCheckpoint, TransactionPlan, TransactionStage};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DebloatRestoreExecutionStep {
    debloat_id: String,
    package_id: String,
    main: ExactPackageIdentity,
    dependencies: Vec<ExactPackageDependency>,
    package_full_name: String,
    package_family_name: String,
    dependency_full_names: Vec<String>,
}

impl DebloatRestoreExecutionStep {
    pub(crate) fn from_prepared(
        step: &DebloatRestorePreparedStep,
    ) -> Result<Self, DebloatRestoreExecutionError> {
        let main = step.main().clone();
        let dependencies = step.dependencies().to_vec();
        validate_restore_route(step.restore(), &main, &dependencies)?;
        let execution = Self {
            debloat_id: step.debloat_id().to_string(),
            package_id: step.package_id().to_string(),
            package_full_name: step.restore().package_full_name().to_string(),
            package_family_name: step.restore().package_family_name().to_string(),
            dependency_full_names: step.restore().dependency_full_names().to_vec(),
            main,
            dependencies,
        };
        execution.validate()?;
        Ok(execution)
    }

    fn validate(&self) -> Result<(), DebloatRestoreExecutionError> {
        if self.debloat_id.trim().is_empty()
            || self.package_id.trim().is_empty()
            || self.package_full_name.trim().is_empty()
            || self.package_family_name.trim().is_empty()
        {
            return Err(DebloatRestoreExecutionError::InvalidPreparedState(
                "prepared restore step contains an empty identity".to_string(),
            ));
        }
        self.main.validate()?;
        for dependency in &self.dependencies {
            dependency.validate()?;
        }
        if !self
            .package_full_name
            .eq_ignore_ascii_case(&self.main.full_name)
            || !self
                .package_family_name
                .eq_ignore_ascii_case(&self.main.family_name)
            || self.dependencies.len() != self.dependency_full_names.len()
            || !self
                .dependency_full_names
                .iter()
                .zip(&self.dependencies)
                .all(|(left, right)| left.eq_ignore_ascii_case(&right.full_name))
            || self.main.dependencies != self.dependencies
        {
            return Err(DebloatRestoreExecutionError::InvalidPreparedState(
                "prepared restore route does not match exact main/dependency identities"
                    .to_string(),
            ));
        }
        Ok(())
    }

    pub fn debloat_id(&self) -> &str {
        &self.debloat_id
    }

    pub fn action_id(&self) -> String {
        format!("restore:{}", self.debloat_id)
    }

    pub fn package_id(&self) -> &str {
        &self.package_id
    }

    pub fn main(&self) -> &ExactPackageIdentity {
        &self.main
    }

    pub fn dependencies(&self) -> &[ExactPackageDependency] {
        &self.dependencies
    }

    pub fn package_full_name(&self) -> &str {
        &self.package_full_name
    }

    pub fn package_family_name(&self) -> &str {
        &self.package_family_name
    }

    pub fn dependency_full_names(&self) -> &[String] {
        &self.dependency_full_names
    }

    #[cfg(test)]
    pub(crate) fn for_tests(
        debloat_id: impl Into<String>,
        package_id: impl Into<String>,
        main: ExactPackageIdentity,
        dependencies: Vec<ExactPackageDependency>,
    ) -> Self {
        Self {
            debloat_id: debloat_id.into(),
            package_id: package_id.into(),
            package_full_name: main.full_name.clone(),
            package_family_name: main.family_name.clone(),
            dependency_full_names: dependencies
                .iter()
                .map(|dependency| dependency.full_name.clone())
                .collect(),
            main,
            dependencies,
        }
    }
}

fn validate_restore_route(
    route: &HistoryRestoreRoute,
    main: &ExactPackageIdentity,
    dependencies: &[ExactPackageDependency],
) -> Result<(), DebloatRestoreExecutionError> {
    if !route
        .package_full_name()
        .eq_ignore_ascii_case(&main.full_name)
        || !route
            .package_family_name()
            .eq_ignore_ascii_case(&main.family_name)
        || route.dependency_full_names().len() != dependencies.len()
        || !route
            .dependency_full_names()
            .iter()
            .zip(dependencies)
            .all(|(left, right)| left.eq_ignore_ascii_case(&right.full_name))
    {
        return Err(DebloatRestoreExecutionError::InvalidPreparedState(
            "Phase 17 restore route differs from the captured exact identities".to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct DebloatRestoreExecutionPlan {
    receipt_fingerprint: String,
    step: DebloatRestoreExecutionStep,
    transaction: TransactionPlan,
}

impl DebloatRestoreExecutionPlan {
    pub(crate) fn from_prepared(
        prepared: &DebloatRestorePreparedTransaction,
    ) -> Result<Self, DebloatRestoreExecutionError> {
        if prepared.machine_changes() {
            return Err(DebloatRestoreExecutionError::InvalidPreparedState(
                "Phase 17 prepared state cannot already claim machine mutation".to_string(),
            ));
        }
        prepared.transaction().validate()?;
        if prepared.transaction().revision() != 1
            || !prepared
                .transaction()
                .transaction_id()
                .ends_with(":phase17-debloat-restore-current-user")
        {
            return Err(DebloatRestoreExecutionError::InvalidPreparedState(
                "Phase 18 accepts only the frozen Phase 17 inverse current-user transaction shape"
                    .to_string(),
            ));
        }
        if prepared.transaction().actions().len() != 1 {
            return Err(DebloatRestoreExecutionError::InvalidPreparedState(
                "Phase 18 accepts exactly one Phase 17 restore action".to_string(),
            ));
        }
        if prepared.checkpoint().stage() != TransactionStage::BaselineCaptured {
            return Err(DebloatRestoreExecutionError::InvalidPreparedState(
                "Phase 17 restore checkpoint must be BaselineCaptured".to_string(),
            ));
        }
        let transaction_fingerprint = prepared.transaction().fingerprint()?;
        if prepared.checkpoint().plan_fingerprint() != transaction_fingerprint
            || prepared.checkpoint().plan().fingerprint()? != transaction_fingerprint
        {
            return Err(DebloatRestoreExecutionError::InvalidPreparedState(
                "Phase 17 checkpoint/transaction fingerprint continuity failed".to_string(),
            ));
        }

        let step = DebloatRestoreExecutionStep::from_prepared(prepared.step())?;
        let action = &prepared.transaction().actions()[0].action;
        if action.id != step.action_id()
            || action.kind != ActionKind::Debloat
            || action.risk != RiskLevel::Low
            || action.recommendation != RecommendationState::Repair
            || action.verdict != EvidenceVerdict::Certified
            || action.selected_by_default
            || !action.requires_confirmation
            || action.requires_admin
            || action.reboot != RebootRequirement::None
            || !action.rollback_available
        {
            return Err(DebloatRestoreExecutionError::InvalidPreparedState(
                "Phase 17 restore action is broader than the frozen explicit low-risk reversible authority"
                    .to_string(),
            ));
        }

        let receipt_fingerprint = prepared.receipt_fingerprint().to_string();
        if receipt_fingerprint.trim().is_empty() {
            return Err(DebloatRestoreExecutionError::InvalidPreparedState(
                "Phase 17 receipt fingerprint is empty".to_string(),
            ));
        }
        require_action_evidence(action, "phase17_receipt_fingerprint", &receipt_fingerprint)?;
        require_action_evidence(
            action,
            "restore_package_full_name",
            step.package_full_name(),
        )?;
        require_action_evidence(
            action,
            "restore_dependency_count",
            &step.dependencies().len().to_string(),
        )?;

        validate_restore_time_baseline(prepared.checkpoint(), &step)?;

        Ok(Self {
            receipt_fingerprint,
            step,
            transaction: prepared.transaction().clone(),
        })
    }

    pub fn receipt_fingerprint(&self) -> &str {
        &self.receipt_fingerprint
    }

    pub fn step(&self) -> &DebloatRestoreExecutionStep {
        &self.step
    }

    pub fn transaction(&self) -> &TransactionPlan {
        &self.transaction
    }

    #[cfg(test)]
    pub(crate) fn for_tests(
        receipt_fingerprint: impl Into<String>,
        step: DebloatRestoreExecutionStep,
        transaction: TransactionPlan,
    ) -> Self {
        Self {
            receipt_fingerprint: receipt_fingerprint.into(),
            step,
            transaction,
        }
    }
}

fn require_action_evidence(
    action: &neo_core::PlannedAction,
    key: &str,
    expected: &str,
) -> Result<(), DebloatRestoreExecutionError> {
    let matches = action
        .evidence
        .iter()
        .filter(|item| item.key == key)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [item] if item.value == expected => Ok(()),
        [item] => Err(DebloatRestoreExecutionError::InvalidPreparedState(format!(
            "Phase 17 action evidence {key} differs from prepared state: {}",
            item.value
        ))),
        [] => Err(DebloatRestoreExecutionError::InvalidPreparedState(format!(
            "Phase 17 action evidence {key} is missing"
        ))),
        _ => Err(DebloatRestoreExecutionError::InvalidPreparedState(format!(
            "Phase 17 action evidence {key} is duplicated"
        ))),
    }
}

fn validate_restore_time_baseline(
    checkpoint: &TransactionCheckpoint,
    step: &DebloatRestoreExecutionStep,
) -> Result<(), DebloatRestoreExecutionError> {
    let baseline = checkpoint.baseline().ok_or_else(|| {
        DebloatRestoreExecutionError::InvalidPreparedState(
            "Phase 17 restore-time baseline is missing".to_string(),
        )
    })?;
    let main_target = appx_target(step.package_full_name());
    match baseline.get(&main_target) {
        Some(neo_transaction::CapturedValue::Absent) => {}
        _ => {
            return Err(DebloatRestoreExecutionError::InvalidPreparedState(
                "Phase 17 restore-time main baseline must be Absent".to_string(),
            ))
        }
    }
    for dependency in step.dependencies() {
        let target = appx_target(&dependency.full_name);
        match baseline.get(&target) {
            Some(neo_transaction::CapturedValue::Absent) => {}
            Some(neo_transaction::CapturedValue::Present(value)) => {
                let captured: ExactPackageDependency = serde_json::from_str(value)?;
                if captured != *dependency {
                    return Err(DebloatRestoreExecutionError::InvalidPreparedState(format!(
                        "Phase 17 restore-time dependency baseline differs from {}",
                        dependency.full_name
                    )));
                }
            }
            _ => {
                return Err(DebloatRestoreExecutionError::InvalidPreparedState(format!(
                    "Phase 17 restore-time dependency baseline for {} is unavailable or missing",
                    dependency.full_name
                )))
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct DebloatRestoreExecutionSession {
    pub(crate) plan: DebloatRestoreExecutionPlan,
    pub(crate) checkpoint: TransactionCheckpoint,
}

impl DebloatRestoreExecutionSession {
    pub(crate) fn from_prepared(
        prepared: &DebloatRestorePreparedTransaction,
    ) -> Result<Self, DebloatRestoreExecutionError> {
        let plan = DebloatRestoreExecutionPlan::from_prepared(prepared)?;
        let checkpoint = prepared.checkpoint().clone();
        if checkpoint.plan_fingerprint() != plan.transaction.fingerprint()? {
            return Err(DebloatRestoreExecutionError::InvalidPreparedState(
                "Phase 18 execution-session fingerprint mismatch".to_string(),
            ));
        }
        Ok(Self { plan, checkpoint })
    }

    pub fn plan(&self) -> &DebloatRestoreExecutionPlan {
        &self.plan
    }

    pub fn checkpoint(&self) -> &TransactionCheckpoint {
        &self.checkpoint
    }

    pub fn stage(&self) -> TransactionStage {
        self.checkpoint.stage()
    }

    #[cfg(test)]
    pub(crate) fn for_tests(
        plan: DebloatRestoreExecutionPlan,
        checkpoint: TransactionCheckpoint,
    ) -> Self {
        Self { plan, checkpoint }
    }
}

#[derive(Debug)]
pub struct DebloatRestoreExecutorCapability {
    _private: (),
}

impl DebloatRestoreExecutorCapability {
    #[cfg(windows)]
    pub(crate) fn for_rpc() -> Self {
        Self { _private: () }
    }

    #[cfg(test)]
    pub(crate) fn for_tests() -> Self {
        Self { _private: () }
    }
}

pub(crate) fn appx_target(full_name: &str) -> neo_transaction::StateTarget {
    neo_transaction::StateTarget {
        kind: neo_transaction::StateTargetKind::AppxPackage,
        key: format!("current_user:{full_name}"),
    }
}

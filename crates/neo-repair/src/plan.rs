use crate::error::RepairError;
use crate::model::{
    ComponentStoreObservation, ComponentStoreState, SupportedWindowsFeature, SystemFileObservation,
    SystemFileState, WindowsFeatureObservation, WindowsFeatureState,
};
use crate::operation::{RepairBaseline, RepairOperation};
use neo_core::{
    ActionKind, EvidenceItem, EvidenceVerdict, PlannedAction, RebootRequirement,
    RecommendationState, RiskLevel,
};
use neo_transaction::{
    CapturedState, CapturedValue, RollbackPlan, StateTarget, StateTargetKind, TransactionAction,
    TransactionCheckpoint, TransactionPlan, VerificationExpectation, VerificationPredicate,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairExecutionPlan {
    operation: RepairOperation,
    baseline: RepairBaseline,
    transaction: TransactionPlan,
}

impl RepairExecutionPlan {
    pub(crate) fn from_component_store(
        observation: &ComponentStoreObservation,
        mission_id: impl Into<String>,
    ) -> Result<Self, RepairError> {
        match observation.state {
            ComponentStoreState::Healthy => {
                return Err(RepairError::NothingToRepair(
                    "component store is healthy".to_string(),
                ))
            }
            ComponentStoreState::Repairable => {}
            ComponentStoreState::Unrepairable => {
                return Err(RepairError::StateUnavailable(
                    "component store is not repairable by DISM RestoreHealth".to_string(),
                ))
            }
            ComponentStoreState::Unavailable => return unavailable(&observation.detail),
        }
        Self::new(
            RepairOperation::RestoreComponentStore,
            RepairBaseline::ComponentStore(observation.state),
            mission_id,
        )
    }

    pub(crate) fn from_system_files(
        observation: &SystemFileObservation,
        mission_id: impl Into<String>,
    ) -> Result<Self, RepairError> {
        match observation.state {
            SystemFileState::Healthy => {
                return Err(RepairError::NothingToRepair(
                    "protected system files are healthy".to_string(),
                ))
            }
            SystemFileState::IntegrityViolations => {}
            SystemFileState::Unavailable => return unavailable(&observation.detail),
        }
        Self::new(
            RepairOperation::RepairSystemFiles,
            RepairBaseline::SystemFiles(observation.state),
            mission_id,
        )
    }

    pub(crate) fn from_feature(
        observation: &WindowsFeatureObservation,
        desired: crate::model::FeatureDesiredState,
        mission_id: impl Into<String>,
    ) -> Result<Self, RepairError> {
        if observation.state == WindowsFeatureState::Unavailable {
            return unavailable(&observation.detail);
        }
        if !observation.state.is_stable() {
            return Err(RepairError::FeatureNotReversible(format!(
                "{} baseline is {:?}",
                observation.feature.id(),
                observation.state
            )));
        }
        if observation.state == desired.target_state() {
            return Err(RepairError::NothingToChange(observation.feature.id().to_string()));
        }
        Self::new(
            RepairOperation::SetWindowsFeature {
                feature: observation.feature,
                desired,
            },
            RepairBaseline::WindowsFeature {
                feature: observation.feature,
                state: observation.state,
            },
            mission_id,
        )
    }

    fn new(
        operation: RepairOperation,
        baseline: RepairBaseline,
        mission_id: impl Into<String>,
    ) -> Result<Self, RepairError> {
        let mission_id = mission_id.into();
        if mission_id.trim().is_empty() {
            return Err(RepairError::InvalidRequest(
                "mission id must not be empty".to_string(),
            ));
        }
        let transaction = transaction_for(operation, baseline, &mission_id)?;
        let plan = Self {
            operation,
            baseline,
            transaction,
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn operation(&self) -> RepairOperation {
        self.operation
    }

    pub fn baseline(&self) -> RepairBaseline {
        self.baseline
    }

    pub fn transaction(&self) -> &TransactionPlan {
        &self.transaction
    }

    pub fn action_id(&self) -> String {
        self.operation.action_id()
    }

    pub fn validate(&self) -> Result<(), RepairError> {
        self.transaction.validate()?;
        let expected = transaction_for(
            self.operation,
            self.baseline,
            self.transaction.mission_id(),
        )?;
        if expected.fingerprint()? != self.transaction.fingerprint()? {
            return Err(RepairError::InvalidRequest(
                "Phase 21 transaction does not match operation/baseline authority".to_string(),
            ));
        }
        Ok(())
    }

    pub(crate) fn checkpoint(&self) -> Result<TransactionCheckpoint, RepairError> {
        self.validate()?;
        let mut checkpoint = TransactionCheckpoint::new(self.transaction.clone())?;
        checkpoint.capture_baseline(vec![CapturedState {
            target: target_for(self.operation),
            value: CapturedValue::Present(self.baseline.transaction_value().to_string()),
        }])?;
        Ok(checkpoint)
    }
}

fn transaction_for(
    operation: RepairOperation,
    baseline: RepairBaseline,
    mission_id: &str,
) -> Result<TransactionPlan, RepairError> {
    validate_operation_baseline(operation, baseline)?;
    let action_id = operation.action_id();
    let target = target_for(operation);
    let postcondition = VerificationPredicate {
        id: format!("verify:{action_id}"),
        target: target.clone(),
        expectation: VerificationExpectation::Equals(target_value(operation).to_string()),
        required: true,
    };
    let (kind, risk, title, rationale, rollback_available, rollback, recommendation) =
        match operation {
            RepairOperation::RestoreComponentStore => (
                ActionKind::Repair,
                RiskLevel::Normal,
                "Repair Windows component store".to_string(),
                "DISM reports repairable component-store corruption; use the fixed RestoreHealth route and re-check health afterward.".to_string(),
                false,
                RollbackPlan::Irreversible {
                    reason: "Windows component-store repair has no deterministic inverse; Neo must verify the post-repair health state instead of claiming rollback.".to_string(),
                },
                RecommendationState::Repair,
            ),
            RepairOperation::RepairSystemFiles => (
                ActionKind::Repair,
                RiskLevel::Normal,
                "Repair protected Windows system files".to_string(),
                "SFC reports protected-system-file integrity violations; run the fixed scannow repair route and re-verify afterward.".to_string(),
                false,
                RollbackPlan::Irreversible {
                    reason: "SFC repair has no deterministic inverse; Neo must verify the post-repair protected-file state instead of claiming rollback.".to_string(),
                },
                RecommendationState::Repair,
            ),
            RepairOperation::SetWindowsFeature { feature, desired } => (
                ActionKind::WindowsFeature,
                feature.risk(),
                format!(
                    "{} {}",
                    match desired {
                        crate::model::FeatureDesiredState::Enabled => "Enable",
                        crate::model::FeatureDesiredState::Disabled => "Disable",
                    },
                    feature.title()
                ),
                format!(
                    "Change the fixed Windows feature {} from captured {:?} to {:?}; no feature payload removal is permitted.",
                    feature.dism_name(),
                    baseline,
                    desired.target_state()
                ),
                true,
                RollbackPlan::Reversible {
                    restore_targets: vec![target.clone()],
                    verification: vec![VerificationPredicate {
                        id: format!("rollback:{action_id}"),
                        target: target.clone(),
                        expectation: VerificationExpectation::MatchesBaseline,
                        required: true,
                    }],
                },
                RecommendationState::OptionalComponent,
            ),
        };

    let planned = PlannedAction {
        id: action_id.clone(),
        title,
        kind,
        risk,
        recommendation,
        verdict: EvidenceVerdict::Certified,
        rationale,
        selected_by_default: false,
        requires_confirmation: true,
        requires_admin: true,
        reboot: RebootRequirement::Possible,
        rollback_available,
        evidence: vec![
            EvidenceItem::new("phase21_operation", action_id.clone(), "neo-repair"),
            EvidenceItem::new(
                "captured_baseline",
                baseline.transaction_value(),
                "neo-repair",
            ),
        ],
        warnings: match rollback_available {
            true => vec!["A Windows feature transition may require reboot before final verification.".to_string()],
            false => vec!["This Windows repair action is irreversible; completion requires fresh post-repair diagnosis.".to_string()],
        },
    };
    TransactionPlan::new(
        format!("{mission_id}:phase21-repair-windows-features"),
        1,
        mission_id,
        vec![TransactionAction {
            action: planned,
            snapshot_targets: vec![target],
            postconditions: vec![postcondition],
            rollback,
        }],
    )
    .map_err(RepairError::from)
}

fn validate_operation_baseline(
    operation: RepairOperation,
    baseline: RepairBaseline,
) -> Result<(), RepairError> {
    let valid = match (operation, baseline) {
        (
            RepairOperation::RestoreComponentStore,
            RepairBaseline::ComponentStore(ComponentStoreState::Repairable),
        ) => true,
        (
            RepairOperation::RepairSystemFiles,
            RepairBaseline::SystemFiles(SystemFileState::IntegrityViolations),
        ) => true,
        (
            RepairOperation::SetWindowsFeature { feature, desired },
            RepairBaseline::WindowsFeature {
                feature: baseline_feature,
                state,
            },
        ) => {
            feature == baseline_feature && state.is_stable() && state != desired.target_state()
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(RepairError::InvalidRequest(
            "operation does not match a frozen Phase 21 actionable baseline".to_string(),
        ))
    }
}

pub(crate) fn target_for(operation: RepairOperation) -> StateTarget {
    match operation {
        RepairOperation::RestoreComponentStore => StateTarget {
            kind: StateTargetKind::Other,
            key: "phase21:component_store".to_string(),
        },
        RepairOperation::RepairSystemFiles => StateTarget {
            kind: StateTargetKind::Other,
            key: "phase21:system_files".to_string(),
        },
        RepairOperation::SetWindowsFeature { feature, .. } => StateTarget {
            kind: StateTargetKind::WindowsFeature,
            key: feature.dism_name().to_string(),
        },
    }
}

pub(crate) fn target_value(operation: RepairOperation) -> &'static str {
    match operation {
        RepairOperation::RestoreComponentStore | RepairOperation::RepairSystemFiles => "healthy",
        RepairOperation::SetWindowsFeature { desired, .. } => {
            desired.target_state().as_transaction_value()
        }
    }
}

fn unavailable<T>(detail: &str) -> Result<T, RepairError> {
    if detail.to_ascii_lowercase().contains("elevated") {
        Err(RepairError::ElevationRequired)
    } else {
        Err(RepairError::StateUnavailable(detail.to_string()))
    }
}

pub(crate) fn feature_baseline_state(
    baseline: RepairBaseline,
) -> Option<(SupportedWindowsFeature, WindowsFeatureState)> {
    match baseline {
        RepairBaseline::WindowsFeature { feature, state } => Some((feature, state)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::BoundedCommandEvidence;

    fn empty_evidence() -> BoundedCommandEvidence {
        BoundedCommandEvidence {
            program: "fake".to_string(),
            args: Vec::new(),
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            start_error: None,
            stdout_truncated: false,
            stderr_truncated: false,
        }
    }

    #[test]
    fn repair_transactions_are_irreversible() {
        let plan = RepairExecutionPlan::from_component_store(
            &ComponentStoreObservation {
                state: ComponentStoreState::Repairable,
                detail: "repairable".to_string(),
                evidence: empty_evidence(),
            },
            "mission",
        )
        .unwrap();
        assert!(matches!(
            &plan.transaction().actions()[0].rollback,
            RollbackPlan::Irreversible { .. }
        ));
        assert!(!plan.transaction().actions()[0].action.rollback_available);
    }

    #[test]
    fn feature_transactions_restore_exact_baseline() {
        let feature = SupportedWindowsFeature::DirectPlay;
        let plan = RepairExecutionPlan::from_feature(
            &WindowsFeatureObservation {
                feature,
                state: WindowsFeatureState::Disabled,
                detail: "disabled".to_string(),
                evidence: empty_evidence(),
            },
            crate::model::FeatureDesiredState::Enabled,
            "mission",
        )
        .unwrap();
        assert!(matches!(
            &plan.transaction().actions()[0].rollback,
            RollbackPlan::Reversible { .. }
        ));
        assert_eq!(
            plan.checkpoint().unwrap().baseline().unwrap().get(&target_for(plan.operation())),
            Some(&CapturedValue::Present("disabled".to_string()))
        );
    }

    #[test]
    fn removed_feature_cannot_be_promoted_to_reversible_mutation() {
        let result = RepairExecutionPlan::from_feature(
            &WindowsFeatureObservation {
                feature: SupportedWindowsFeature::NetFx3,
                state: WindowsFeatureState::Removed,
                detail: "removed".to_string(),
                evidence: empty_evidence(),
            },
            crate::model::FeatureDesiredState::Enabled,
            "mission",
        );
        assert!(matches!(result, Err(RepairError::FeatureNotReversible(_))));
    }
}

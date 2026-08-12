use crate::error::TransactionError;
use crate::state::{
    ensure_unique_predicates, ensure_unique_targets, set_of_targets, StateTarget,
    VerificationExpectation, VerificationPredicate,
};
use neo_core::{EvidenceVerdict, PlannedAction, RebootRequirement, RecommendationState, RiskLevel};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum RollbackPlan {
    Reversible {
        restore_targets: Vec<StateTarget>,
        verification: Vec<VerificationPredicate>,
    },
    Irreversible {
        reason: String,
    },
}

impl RollbackPlan {
    pub(crate) fn is_reversible(&self) -> bool {
        matches!(self, Self::Reversible { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionAction {
    pub action: PlannedAction,
    #[serde(default)]
    pub snapshot_targets: Vec<StateTarget>,
    pub postconditions: Vec<VerificationPredicate>,
    pub rollback: RollbackPlan,
}

impl TransactionAction {
    fn validate(&self) -> Result<(), TransactionError> {
        self.action.validate()?;
        if !self.action.kind.mutates_machine() {
            return Err(TransactionError::NonMutatingTransactionAction(
                self.action.id.clone(),
            ));
        }
        if self.action.verdict == EvidenceVerdict::Rejected {
            return Err(TransactionError::RejectedAction(self.action.id.clone()));
        }
        if self.postconditions.is_empty() {
            return Err(TransactionError::MissingPostconditions(
                self.action.id.clone(),
            ));
        }

        ensure_unique_targets(&self.snapshot_targets)?;
        for target in &self.snapshot_targets {
            target.validate()?;
        }
        ensure_unique_predicates(&self.postconditions)?;

        match &self.rollback {
            RollbackPlan::Reversible {
                restore_targets,
                verification,
            } => {
                if !self.action.rollback_available {
                    return Err(TransactionError::RollbackContractMismatch(
                        self.action.id.clone(),
                    ));
                }
                if self.snapshot_targets.is_empty() || restore_targets.is_empty() {
                    return Err(TransactionError::MissingRollbackSnapshot(
                        self.action.id.clone(),
                    ));
                }
                ensure_unique_targets(restore_targets)?;
                if set_of_targets(restore_targets) != set_of_targets(&self.snapshot_targets) {
                    return Err(TransactionError::RollbackTargetMismatch(
                        self.action.id.clone(),
                    ));
                }
                if verification.is_empty() {
                    return Err(TransactionError::MissingRollbackVerification(
                        self.action.id.clone(),
                    ));
                }
                ensure_unique_predicates(verification)?;
                let verification_targets = verification
                    .iter()
                    .map(|predicate| predicate.target.clone())
                    .collect::<BTreeSet<_>>();
                if verification_targets != set_of_targets(restore_targets)
                    || verification.iter().any(|predicate| {
                        predicate.expectation != VerificationExpectation::MatchesBaseline
                    })
                {
                    return Err(TransactionError::InvalidRollbackVerification(
                        self.action.id.clone(),
                    ));
                }
            }
            RollbackPlan::Irreversible { reason } => {
                if self.action.rollback_available || reason.trim().is_empty() {
                    return Err(TransactionError::RollbackContractMismatch(
                        self.action.id.clone(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn needs_manual_override(&self) -> bool {
        self.action.verdict != EvidenceVerdict::Certified
            || matches!(
                self.action.recommendation,
                RecommendationState::Conflict
                    | RecommendationState::Unsupported
                    | RecommendationState::DoNotTouch
                    | RecommendationState::Unknown
            )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TransactionPlanWire")]
pub struct TransactionPlan {
    transaction_id: String,
    revision: u32,
    mission_id: String,
    actions: Vec<TransactionAction>,
}

#[derive(Debug, Deserialize)]
struct TransactionPlanWire {
    transaction_id: String,
    revision: u32,
    mission_id: String,
    actions: Vec<TransactionAction>,
}

impl TryFrom<TransactionPlanWire> for TransactionPlan {
    type Error = TransactionError;

    fn try_from(value: TransactionPlanWire) -> Result<Self, Self::Error> {
        Self::new(
            value.transaction_id,
            value.revision,
            value.mission_id,
            value.actions,
        )
    }
}

impl TransactionPlan {
    pub fn new(
        transaction_id: impl Into<String>,
        revision: u32,
        mission_id: impl Into<String>,
        actions: Vec<TransactionAction>,
    ) -> Result<Self, TransactionError> {
        let plan = Self {
            transaction_id: transaction_id.into(),
            revision,
            mission_id: mission_id.into(),
            actions,
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn from_json_str(input: &str) -> Result<Self, TransactionError> {
        let wire: TransactionPlanWire = serde_json::from_str(input)?;
        Self::try_from(wire)
    }

    pub fn transaction_id(&self) -> &str {
        &self.transaction_id
    }

    pub fn revision(&self) -> u32 {
        self.revision
    }

    pub fn mission_id(&self) -> &str {
        &self.mission_id
    }

    pub fn actions(&self) -> &[TransactionAction] {
        &self.actions
    }

    pub fn validate(&self) -> Result<(), TransactionError> {
        if self.transaction_id.trim().is_empty() {
            return Err(TransactionError::MissingTransactionId);
        }
        if self.revision == 0 {
            return Err(TransactionError::InvalidRevision);
        }
        if self.mission_id.trim().is_empty() {
            return Err(TransactionError::MissingMissionId);
        }
        if self.actions.is_empty() {
            return Err(TransactionError::EmptyTransactionPlan);
        }

        let mut action_ids = BTreeSet::new();
        let mut predicate_ids = BTreeSet::new();
        let mut snapshot_targets = BTreeSet::new();
        for transaction_action in &self.actions {
            transaction_action.validate()?;
            if !action_ids.insert(transaction_action.action.id.as_str()) {
                return Err(TransactionError::DuplicateTransactionAction(
                    transaction_action.action.id.clone(),
                ));
            }
            for target in &transaction_action.snapshot_targets {
                if !snapshot_targets.insert(target.clone()) {
                    return Err(TransactionError::OverlappingSnapshotTarget(
                        target.key.clone(),
                    ));
                }
            }
            for predicate in &transaction_action.postconditions {
                if !predicate_ids.insert(predicate.id.as_str()) {
                    return Err(TransactionError::DuplicatePredicateId(predicate.id.clone()));
                }
            }
            if let RollbackPlan::Reversible { verification, .. } = &transaction_action.rollback {
                for predicate in verification {
                    if !predicate_ids.insert(predicate.id.as_str()) {
                        return Err(TransactionError::DuplicatePredicateId(predicate.id.clone()));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn fingerprint(&self) -> Result<String, TransactionError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)?;
        let digest = Sha256::digest(bytes);
        let mut encoded = String::with_capacity(digest.len() * 2);
        for byte in digest {
            write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
        }
        Ok(encoded)
    }

    fn action_ids(&self) -> BTreeSet<String> {
        self.actions
            .iter()
            .map(|transaction_action| transaction_action.action.id.clone())
            .collect()
    }

    pub(crate) fn action_by_id(&self, action_id: &str) -> Option<&TransactionAction> {
        self.actions
            .iter()
            .find(|transaction_action| transaction_action.action.id == action_id)
    }

    pub(crate) fn required_snapshot_targets(&self) -> BTreeSet<StateTarget> {
        self.actions
            .iter()
            .flat_map(|transaction_action| transaction_action.snapshot_targets.iter().cloned())
            .collect()
    }

    pub(crate) fn required_rollback_targets(&self) -> BTreeSet<StateTarget> {
        self.actions
            .iter()
            .flat_map(|transaction_action| match &transaction_action.rollback {
                RollbackPlan::Reversible {
                    restore_targets, ..
                } => restore_targets.as_slice(),
                RollbackPlan::Irreversible { .. } => &[],
            })
            .cloned()
            .collect()
    }

    pub(crate) fn postconditions(&self) -> Vec<VerificationPredicate> {
        self.actions
            .iter()
            .flat_map(|transaction_action| transaction_action.postconditions.iter().cloned())
            .collect()
    }

    pub(crate) fn rollback_predicates_for(
        &self,
        action_ids: &BTreeSet<String>,
    ) -> Vec<VerificationPredicate> {
        self.actions
            .iter()
            .filter(|transaction_action| action_ids.contains(&transaction_action.action.id))
            .flat_map(|transaction_action| match &transaction_action.rollback {
                RollbackPlan::Reversible { verification, .. } => verification.as_slice(),
                RollbackPlan::Irreversible { .. } => &[],
            })
            .cloned()
            .collect()
    }

    pub(crate) fn requires_reboot(&self) -> bool {
        self.actions.iter().any(|transaction_action| {
            transaction_action.action.reboot == RebootRequirement::Required
        })
    }

    pub(crate) fn all_reversible(&self, action_ids: &BTreeSet<String>) -> bool {
        action_ids.iter().all(|action_id| {
            self.action_by_id(action_id)
                .is_some_and(|transaction_action| transaction_action.rollback.is_reversible())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionAcknowledgement {
    pub action_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionAuthorization {
    pub plan_fingerprint: String,
    pub approved_action_ids: Vec<String>,
    #[serde(default)]
    pub manual_override_action_ids: Vec<String>,
    #[serde(default)]
    pub high_risk_ack_action_ids: Vec<String>,
    #[serde(default)]
    pub irreversible_acknowledgements: Vec<ActionAcknowledgement>,
}

impl TransactionAuthorization {
    pub(crate) fn validate_for_plan(&self, plan: &TransactionPlan) -> Result<(), TransactionError> {
        if self.plan_fingerprint != plan.fingerprint()? {
            return Err(TransactionError::AuthorizationFingerprintMismatch);
        }
        let plan_ids = plan.action_ids();
        let approved = unique_id_set(&self.approved_action_ids)?;
        if approved != plan_ids {
            return Err(TransactionError::AuthorizationCoverageMismatch);
        }

        let manual_override = unique_id_set(&self.manual_override_action_ids)?;
        ensure_known_ids(&manual_override, &plan_ids)?;
        let required_manual = plan
            .actions
            .iter()
            .filter(|transaction_action| transaction_action.needs_manual_override())
            .map(|transaction_action| transaction_action.action.id.clone())
            .collect::<BTreeSet<_>>();
        if !required_manual.is_subset(&manual_override) {
            return Err(TransactionError::MissingManualOverride);
        }

        let high_risk = unique_id_set(&self.high_risk_ack_action_ids)?;
        ensure_known_ids(&high_risk, &plan_ids)?;
        let required_high_risk = plan
            .actions
            .iter()
            .filter(|transaction_action| transaction_action.action.risk >= RiskLevel::High)
            .map(|transaction_action| transaction_action.action.id.clone())
            .collect::<BTreeSet<_>>();
        if !required_high_risk.is_subset(&high_risk) {
            return Err(TransactionError::MissingHighRiskAcknowledgement);
        }

        let mut irreversible_ids = BTreeSet::new();
        for acknowledgement in &self.irreversible_acknowledgements {
            if acknowledgement.reason.trim().is_empty() {
                return Err(TransactionError::EmptyIrreversibleAcknowledgement(
                    acknowledgement.action_id.clone(),
                ));
            }
            if !irreversible_ids.insert(acknowledgement.action_id.clone()) {
                return Err(TransactionError::DuplicateAuthorizationId(
                    acknowledgement.action_id.clone(),
                ));
            }
        }
        ensure_known_ids(&irreversible_ids, &plan_ids)?;
        let required_irreversible = plan
            .actions
            .iter()
            .filter(|transaction_action| {
                matches!(
                    &transaction_action.rollback,
                    RollbackPlan::Irreversible { .. }
                )
            })
            .map(|transaction_action| transaction_action.action.id.clone())
            .collect::<BTreeSet<_>>();
        if required_irreversible != irreversible_ids {
            return Err(TransactionError::MissingIrreversibleAcknowledgement);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyOutcome {
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyRecord {
    pub action_id: String,
    pub outcome: ApplyOutcome,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackRecord {
    pub action_id: String,
    pub outcome: ApplyOutcome,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStage {
    Planned,
    BaselineCaptured,
    Authorized,
    Applying,
    AwaitingReboot,
    Verifying,
    RollingBack,
    Complete,
    RolledBack,
    Failed,
    Blocked,
}

fn unique_id_set(values: &[String]) -> Result<BTreeSet<String>, TransactionError> {
    let mut set = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() {
            return Err(TransactionError::EmptyAuthorizationId);
        }
        if !set.insert(value.clone()) {
            return Err(TransactionError::DuplicateAuthorizationId(value.clone()));
        }
    }
    Ok(set)
}

fn ensure_known_ids(
    actual: &BTreeSet<String>,
    known: &BTreeSet<String>,
) -> Result<(), TransactionError> {
    if !actual.is_subset(known) {
        return Err(TransactionError::UnknownAuthorizationId);
    }
    Ok(())
}

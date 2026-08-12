use crate::error::TransactionError;
use crate::plan::{
    ApplyOutcome, ApplyRecord, RollbackRecord, TransactionAuthorization, TransactionPlan,
    TransactionStage,
};
use crate::state::{
    BaselineSnapshot, CapturedState, Observation, StateTarget, VerificationPredicate,
    VerificationResult,
};
use crate::verification::{evaluate_predicates, required_results_pass};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebootCheckpoint {
    pub transaction_id: String,
    pub plan_fingerprint: String,
    pub expected_post_reboot: Vec<VerificationPredicate>,
    pub restoration_obligations: Vec<StateTarget>,
    pub resume_stage: TransactionStage,
}

impl RebootCheckpoint {
    pub(crate) fn for_apply_checkpoint(checkpoint: &TransactionCheckpoint) -> Self {
        Self {
            transaction_id: checkpoint.plan.transaction_id().to_string(),
            plan_fingerprint: checkpoint.plan_fingerprint.clone(),
            expected_post_reboot: checkpoint.plan.postconditions(),
            restoration_obligations: checkpoint
                .plan
                .required_rollback_targets()
                .into_iter()
                .collect(),
            resume_stage: TransactionStage::Verifying,
        }
    }

    pub(crate) fn for_rollback_checkpoint(checkpoint: &TransactionCheckpoint) -> Self {
        let changed = checkpoint.successful_applied_ids();
        Self {
            transaction_id: checkpoint.plan.transaction_id().to_string(),
            plan_fingerprint: checkpoint.plan_fingerprint.clone(),
            expected_post_reboot: checkpoint.plan.rollback_predicates_for(&changed),
            restoration_obligations: checkpoint
                .plan
                .rollback_targets_for(&changed)
                .into_iter()
                .collect(),
            resume_stage: TransactionStage::RolledBack,
        }
    }

    pub(crate) fn validate_for_checkpoint(
        &self,
        checkpoint: &TransactionCheckpoint,
    ) -> Result<(), TransactionError> {
        let expected = match self.resume_stage {
            TransactionStage::Verifying => Self::for_apply_checkpoint(checkpoint),
            TransactionStage::RolledBack => Self::for_rollback_checkpoint(checkpoint),
            _ => return Err(TransactionError::RebootCheckpointMismatch),
        };
        if self != &expected {
            return Err(TransactionError::RebootCheckpointMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionEvent {
    pub sequence: u64,
    pub stage: TransactionStage,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TransactionCheckpointWire")]
pub struct TransactionCheckpoint {
    pub(crate) plan: TransactionPlan,
    pub(crate) plan_fingerprint: String,
    pub(crate) stage: TransactionStage,
    pub(crate) baseline: Option<BaselineSnapshot>,
    pub(crate) authorization: Option<TransactionAuthorization>,
    #[serde(default)]
    pub(crate) apply_records: Vec<ApplyRecord>,
    #[serde(default)]
    pub(crate) resume_results: Vec<VerificationResult>,
    #[serde(default)]
    pub(crate) verification_results: Vec<VerificationResult>,
    #[serde(default)]
    pub(crate) rollback_records: Vec<RollbackRecord>,
    #[serde(default)]
    pub(crate) rollback_results: Vec<VerificationResult>,
    pub(crate) reboot_checkpoint: Option<RebootCheckpoint>,
    #[serde(default)]
    pub(crate) events: Vec<TransactionEvent>,
}

#[derive(Debug, Deserialize)]
struct TransactionCheckpointWire {
    plan: TransactionPlan,
    plan_fingerprint: String,
    stage: TransactionStage,
    baseline: Option<BaselineSnapshot>,
    authorization: Option<TransactionAuthorization>,
    #[serde(default)]
    apply_records: Vec<ApplyRecord>,
    #[serde(default)]
    resume_results: Vec<VerificationResult>,
    #[serde(default)]
    verification_results: Vec<VerificationResult>,
    #[serde(default)]
    rollback_records: Vec<RollbackRecord>,
    #[serde(default)]
    rollback_results: Vec<VerificationResult>,
    reboot_checkpoint: Option<RebootCheckpoint>,
    #[serde(default)]
    events: Vec<TransactionEvent>,
}

impl TryFrom<TransactionCheckpointWire> for TransactionCheckpoint {
    type Error = TransactionError;

    fn try_from(value: TransactionCheckpointWire) -> Result<Self, Self::Error> {
        let checkpoint = Self {
            plan: value.plan,
            plan_fingerprint: value.plan_fingerprint,
            stage: value.stage,
            baseline: value.baseline,
            authorization: value.authorization,
            apply_records: value.apply_records,
            resume_results: value.resume_results,
            verification_results: value.verification_results,
            rollback_records: value.rollback_records,
            rollback_results: value.rollback_results,
            reboot_checkpoint: value.reboot_checkpoint,
            events: value.events,
        };
        checkpoint.validate()?;
        Ok(checkpoint)
    }
}

impl TransactionCheckpoint {
    pub fn new(plan: TransactionPlan) -> Result<Self, TransactionError> {
        plan.validate()?;
        let fingerprint = plan.fingerprint()?;
        let mut checkpoint = Self {
            plan,
            plan_fingerprint: fingerprint,
            stage: TransactionStage::Planned,
            baseline: None,
            authorization: None,
            apply_records: Vec::new(),
            resume_results: Vec::new(),
            verification_results: Vec::new(),
            rollback_records: Vec::new(),
            rollback_results: Vec::new(),
            reboot_checkpoint: None,
            events: Vec::new(),
        };
        checkpoint.record_event("transaction checkpoint created");
        checkpoint.validate()?;
        Ok(checkpoint)
    }

    pub fn from_json_str(input: &str) -> Result<Self, TransactionError> {
        let wire: TransactionCheckpointWire = serde_json::from_str(input)?;
        Self::try_from(wire)
    }

    pub fn plan(&self) -> &TransactionPlan {
        &self.plan
    }

    pub fn plan_fingerprint(&self) -> &str {
        &self.plan_fingerprint
    }

    pub fn stage(&self) -> TransactionStage {
        self.stage
    }

    pub fn baseline(&self) -> Option<&BaselineSnapshot> {
        self.baseline.as_ref()
    }

    pub fn events(&self) -> &[TransactionEvent] {
        &self.events
    }

    pub fn capture_baseline(&mut self, states: Vec<CapturedState>) -> Result<(), TransactionError> {
        self.require_stage(TransactionStage::Planned)?;
        self.validate()?;
        let baseline = BaselineSnapshot::for_plan(&self.plan, states)?;
        self.baseline = Some(baseline);
        self.transition(
            TransactionStage::BaselineCaptured,
            "actual pre-state captured",
        );
        self.validate()
    }

    pub fn authorize(
        &mut self,
        authorization: TransactionAuthorization,
    ) -> Result<(), TransactionError> {
        self.require_stage(TransactionStage::BaselineCaptured)?;
        self.validate()?;
        authorization.validate_for_plan(&self.plan)?;
        self.authorization = Some(authorization);
        self.transition(TransactionStage::Authorized, "user authority recorded");
        self.validate()
    }

    pub fn begin_apply(&mut self) -> Result<(), TransactionError> {
        self.require_stage(TransactionStage::Authorized)?;
        self.validate()?;
        self.transition(TransactionStage::Applying, "apply phase opened");
        self.validate()
    }

    pub fn assert_action_pending(&self, action_id: &str) -> Result<(), TransactionError> {
        self.require_stage(TransactionStage::Applying)?;
        self.validate()?;
        if self.plan.action_by_id(action_id).is_none() {
            return Err(TransactionError::UnknownApplyAction(action_id.to_string()));
        }
        if self
            .apply_records
            .iter()
            .any(|record| record.action_id == action_id)
        {
            return Err(TransactionError::DuplicateApplyRecord(
                action_id.to_string(),
            ));
        }
        Ok(())
    }

    pub fn effective_apply_reboot_required(&self) -> bool {
        self.plan.requires_reboot()
            || self
                .apply_records
                .iter()
                .any(|record| record.outcome == ApplyOutcome::Success && record.reboot_required)
    }

    pub fn effective_rollback_reboot_required(&self) -> bool {
        self.rollback_records
            .iter()
            .any(|record| record.outcome == ApplyOutcome::Success && record.reboot_required)
    }

    pub fn record_apply_result(&mut self, record: ApplyRecord) -> Result<(), TransactionError> {
        self.require_stage(TransactionStage::Applying)?;
        self.validate()?;
        self.validate_apply_record(&record)?;
        self.apply_records.push(record.clone());
        self.record_event(&format!("apply result recorded for {}", record.action_id));

        if record.outcome == ApplyOutcome::Failure {
            let changed = self.successful_applied_ids();
            if !changed.is_empty() && self.plan.all_reversible(&changed) {
                self.transition(
                    TransactionStage::RollingBack,
                    "apply failure requires rollback of changed actions",
                );
            } else {
                self.transition(
                    TransactionStage::Failed,
                    "apply failure cannot be safely rolled back",
                );
            }
            return self.validate();
        }

        if self.apply_records.len() == self.plan.actions().len() {
            if self.effective_apply_reboot_required() {
                self.reboot_checkpoint = Some(RebootCheckpoint::for_apply_checkpoint(self));
                self.transition(
                    TransactionStage::AwaitingReboot,
                    "required reboot checkpoint created",
                );
            } else {
                self.transition(
                    TransactionStage::Verifying,
                    "apply records complete; verification required",
                );
            }
        }
        self.validate()
    }

    pub fn resume_after_reboot(
        &mut self,
        observations: Vec<Observation>,
    ) -> Result<(), TransactionError> {
        self.require_stage(TransactionStage::AwaitingReboot)?;
        self.validate()?;
        let reboot_checkpoint = self
            .reboot_checkpoint
            .as_ref()
            .ok_or(TransactionError::MissingRebootCheckpoint)?;
        reboot_checkpoint.validate_for_checkpoint(self)?;
        let baseline = self
            .baseline
            .as_ref()
            .ok_or(TransactionError::MissingBaseline)?;
        let results = evaluate_predicates(&reboot_checkpoint.expected_post_reboot, &observations)?;
        let passed = required_results_pass(&results, baseline);
        self.resume_results = results;
        if passed {
            self.transition(
                TransactionStage::Verifying,
                "post-reboot state proven; verification may continue",
            );
        } else {
            self.transition(
                TransactionStage::Blocked,
                "post-reboot state not proven; continuation blocked",
            );
        }
        self.validate()
    }

    pub fn reprobe_after_block(
        &mut self,
        observations: Vec<Observation>,
    ) -> Result<(), TransactionError> {
        self.require_stage(TransactionStage::Blocked)?;
        self.validate()?;
        let reboot_checkpoint = self
            .reboot_checkpoint
            .as_ref()
            .ok_or(TransactionError::MissingRebootCheckpoint)?;
        reboot_checkpoint.validate_for_checkpoint(self)?;
        let baseline = self
            .baseline
            .as_ref()
            .ok_or(TransactionError::MissingBaseline)?;
        let results = evaluate_predicates(&reboot_checkpoint.expected_post_reboot, &observations)?;
        let passed = required_results_pass(&results, baseline);
        self.resume_results = results;
        if passed {
            self.transition(
                TransactionStage::Verifying,
                "blocked post-reboot state re-proven; verification may continue",
            );
        } else {
            let changed = self.successful_applied_ids();
            if !changed.is_empty() && self.plan.all_reversible(&changed) {
                self.transition(
                    TransactionStage::RollingBack,
                    "blocked post-reboot state still unproven; rollback required",
                );
            } else {
                self.transition(
                    TransactionStage::Failed,
                    "blocked post-reboot state still unproven without complete rollback path",
                );
            }
        }
        self.validate()
    }

    pub fn verify_postconditions(
        &mut self,
        observations: Vec<Observation>,
    ) -> Result<(), TransactionError> {
        self.require_stage(TransactionStage::Verifying)?;
        self.validate()?;
        let baseline = self
            .baseline
            .as_ref()
            .ok_or(TransactionError::MissingBaseline)?;
        let predicates = self.plan.postconditions();
        let results = evaluate_predicates(&predicates, &observations)?;
        let passed = required_results_pass(&results, baseline);
        self.verification_results = results;
        if passed {
            self.transition(
                TransactionStage::Complete,
                "all required postconditions proven",
            );
        } else {
            let changed = self.successful_applied_ids();
            if !changed.is_empty() && self.plan.all_reversible(&changed) {
                self.transition(
                    TransactionStage::RollingBack,
                    "postcondition verification failed; rollback required",
                );
            } else {
                self.transition(
                    TransactionStage::Failed,
                    "postcondition verification failed without complete rollback path",
                );
            }
        }
        self.validate()
    }

    pub fn record_rollback_result(
        &mut self,
        record: RollbackRecord,
    ) -> Result<(), TransactionError> {
        self.require_stage(TransactionStage::RollingBack)?;
        self.validate()?;
        let changed = self.successful_applied_ids();
        if !changed.contains(&record.action_id) {
            return Err(TransactionError::UnknownRollbackAction(record.action_id));
        }
        let transaction_action = self
            .plan
            .action_by_id(&record.action_id)
            .ok_or_else(|| TransactionError::UnknownRollbackAction(record.action_id.clone()))?;
        if !transaction_action.rollback.is_reversible() {
            return Err(TransactionError::IrreversibleRollbackAttempt(
                record.action_id,
            ));
        }
        if self
            .rollback_records
            .iter()
            .any(|existing| existing.action_id == record.action_id)
        {
            return Err(TransactionError::DuplicateRollbackRecord(record.action_id));
        }
        if record.detail.trim().is_empty() {
            return Err(TransactionError::EmptyExecutionDetail(record.action_id));
        }
        self.rollback_records.push(record.clone());
        self.record_event(&format!(
            "rollback result recorded for {}",
            record.action_id
        ));
        if record.outcome == ApplyOutcome::Failure {
            self.transition(
                TransactionStage::Failed,
                "rollback application failed; recovery remains unresolved",
            );
            return self.validate();
        }

        let changed = self.successful_applied_ids();
        let all_rollback_records_complete = self.rollback_records.len() == changed.len()
            && self
                .rollback_records
                .iter()
                .all(|record| record.outcome == ApplyOutcome::Success);
        if all_rollback_records_complete && self.effective_rollback_reboot_required() {
            self.reboot_checkpoint = Some(RebootCheckpoint::for_rollback_checkpoint(self));
            self.transition(
                TransactionStage::AwaitingRollbackReboot,
                "rollback requires reboot before restoration can be verified",
            );
        }
        self.validate()
    }

    pub fn resume_after_rollback_reboot(
        &mut self,
        observations: Vec<Observation>,
    ) -> Result<(), TransactionError> {
        self.require_stage(TransactionStage::AwaitingRollbackReboot)?;
        self.validate()?;
        let reboot_checkpoint = self
            .reboot_checkpoint
            .as_ref()
            .ok_or(TransactionError::MissingRebootCheckpoint)?;
        reboot_checkpoint.validate_for_checkpoint(self)?;
        let baseline = self
            .baseline
            .as_ref()
            .ok_or(TransactionError::MissingBaseline)?;
        let results = evaluate_predicates(&reboot_checkpoint.expected_post_reboot, &observations)?;
        let passed = required_results_pass(&results, baseline);
        self.rollback_results = results;
        if passed {
            self.transition(
                TransactionStage::RolledBack,
                "post-reboot rollback restoration proven",
            );
        } else {
            self.transition(
                TransactionStage::Failed,
                "post-reboot rollback restoration not proven",
            );
        }
        self.validate()
    }

    pub fn verify_rollback(
        &mut self,
        observations: Vec<Observation>,
    ) -> Result<(), TransactionError> {
        self.require_stage(TransactionStage::RollingBack)?;
        self.validate()?;
        let changed = self.successful_applied_ids();
        self.require_successful_rollback_records(&changed)?;
        let baseline = self
            .baseline
            .as_ref()
            .ok_or(TransactionError::MissingBaseline)?;
        let predicates = self.plan.rollback_predicates_for(&changed);
        let results = evaluate_predicates(&predicates, &observations)?;
        let passed = required_results_pass(&results, baseline);
        self.rollback_results = results;
        if passed {
            self.transition(
                TransactionStage::RolledBack,
                "captured pre-state restoration proven",
            );
        } else {
            self.transition(
                TransactionStage::Failed,
                "rollback verification failed; recovery remains unresolved",
            );
        }
        self.validate()
    }
}

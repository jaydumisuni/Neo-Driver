use crate::checkpoint::{TransactionCheckpoint, TransactionEvent};
use crate::error::TransactionError;
use crate::plan::{ApplyOutcome, ApplyRecord, TransactionStage};
use crate::verification::{validate_record_ids, validate_result_set, validate_rollback_record_ids};
use std::collections::BTreeSet;

impl TransactionCheckpoint {
    pub fn validate(&self) -> Result<(), TransactionError> {
        self.plan.validate()?;
        if self.plan_fingerprint != self.plan.fingerprint()? {
            return Err(TransactionError::CheckpointFingerprintMismatch);
        }
        self.validate_event_log()?;
        validate_record_ids(&self.apply_records, &self.plan)?;
        validate_rollback_record_ids(&self.rollback_records, &self.plan)?;

        if let Some(baseline) = &self.baseline {
            baseline.validate_for_plan(&self.plan)?;
        }
        if let Some(authorization) = &self.authorization {
            authorization.validate_for_plan(&self.plan)?;
        }

        match self.stage {
            TransactionStage::Planned => {
                if self.baseline.is_some()
                    || self.authorization.is_some()
                    || !self.execution_artifacts_empty()
                {
                    return Err(TransactionError::StageInvariantViolation);
                }
            }
            TransactionStage::BaselineCaptured => {
                self.require_baseline_only()?;
                if !self.execution_artifacts_empty() {
                    return Err(TransactionError::StageInvariantViolation);
                }
            }
            TransactionStage::Authorized => {
                self.require_baseline_and_authorization()?;
                if !self.execution_artifacts_empty() {
                    return Err(TransactionError::StageInvariantViolation);
                }
            }
            TransactionStage::Applying => {
                self.require_baseline_and_authorization()?;
                if !self.resume_results.is_empty()
                    || !self.verification_results.is_empty()
                    || !self.rollback_records.is_empty()
                    || !self.rollback_results.is_empty()
                    || self.reboot_checkpoint.is_some()
                {
                    return Err(TransactionError::StageInvariantViolation);
                }
            }
            TransactionStage::AwaitingReboot => {
                self.require_baseline_and_authorization()?;
                self.require_all_apply_success()?;
                if !self.plan.requires_reboot() {
                    return Err(TransactionError::UnexpectedRebootCheckpoint);
                }
                self.reboot_checkpoint
                    .as_ref()
                    .ok_or(TransactionError::MissingRebootCheckpoint)?
                    .validate_for_checkpoint(self)?;
                if !self.resume_results.is_empty()
                    || !self.verification_results.is_empty()
                    || !self.rollback_records.is_empty()
                    || !self.rollback_results.is_empty()
                {
                    return Err(TransactionError::StageInvariantViolation);
                }
            }
            TransactionStage::Verifying => {
                self.require_baseline_and_authorization()?;
                self.require_all_apply_success()?;
                if self.plan.requires_reboot() {
                    self.validate_resume_results(true)?;
                } else if !self.resume_results.is_empty() || self.reboot_checkpoint.is_some() {
                    return Err(TransactionError::UnexpectedRebootCheckpoint);
                }
                if !self.verification_results.is_empty()
                    || !self.rollback_records.is_empty()
                    || !self.rollback_results.is_empty()
                {
                    return Err(TransactionError::StageInvariantViolation);
                }
            }
            TransactionStage::Complete => {
                self.require_baseline_and_authorization()?;
                self.require_all_apply_success()?;
                if self.plan.requires_reboot() {
                    self.validate_resume_results(true)?;
                }
                self.validate_postcondition_results(true)?;
                if !self.rollback_records.is_empty() || !self.rollback_results.is_empty() {
                    return Err(TransactionError::StageInvariantViolation);
                }
            }
            TransactionStage::RollingBack => {
                self.require_baseline_and_authorization()?;
                let changed = self.successful_applied_ids();
                if changed.is_empty() || !self.plan.all_reversible(&changed) {
                    return Err(TransactionError::StageInvariantViolation);
                }
                self.validate_rollback_records_for_changed(&changed)?;
                if !self.rollback_results.is_empty() {
                    return Err(TransactionError::StageInvariantViolation);
                }
            }
            TransactionStage::RolledBack => {
                self.require_baseline_and_authorization()?;
                let changed = self.successful_applied_ids();
                if changed.is_empty() || !self.plan.all_reversible(&changed) {
                    return Err(TransactionError::StageInvariantViolation);
                }
                self.require_successful_rollback_records(&changed)?;
                self.validate_rollback_results(&changed, true)?;
            }
            TransactionStage::Failed => {
                self.require_baseline_and_authorization()?;
            }
            TransactionStage::Blocked => {
                self.require_baseline_and_authorization()?;
                if !self.plan.requires_reboot() {
                    return Err(TransactionError::StageInvariantViolation);
                }
                self.validate_resume_results(false)?;
                if !self.verification_results.is_empty()
                    || !self.rollback_records.is_empty()
                    || !self.rollback_results.is_empty()
                {
                    return Err(TransactionError::StageInvariantViolation);
                }
            }
        }
        Ok(())
    }

    fn execution_artifacts_empty(&self) -> bool {
        self.apply_records.is_empty()
            && self.resume_results.is_empty()
            && self.verification_results.is_empty()
            && self.rollback_records.is_empty()
            && self.rollback_results.is_empty()
            && self.reboot_checkpoint.is_none()
    }

    fn validate_rollback_records_for_changed(
        &self,
        changed: &BTreeSet<String>,
    ) -> Result<(), TransactionError> {
        for record in &self.rollback_records {
            if !changed.contains(&record.action_id) || record.outcome != ApplyOutcome::Success {
                return Err(TransactionError::StageInvariantViolation);
            }
            let transaction_action = self
                .plan
                .action_by_id(&record.action_id)
                .ok_or_else(|| TransactionError::UnknownRollbackAction(record.action_id.clone()))?;
            if !transaction_action.rollback.is_reversible() {
                return Err(TransactionError::StageInvariantViolation);
            }
        }
        Ok(())
    }

    pub(crate) fn require_stage(&self, expected: TransactionStage) -> Result<(), TransactionError> {
        if self.stage != expected {
            return Err(TransactionError::InvalidStageTransition {
                expected,
                actual: self.stage,
            });
        }
        Ok(())
    }

    fn require_baseline_only(&self) -> Result<(), TransactionError> {
        if self.baseline.is_none() || self.authorization.is_some() || !self.apply_records.is_empty()
        {
            return Err(TransactionError::StageInvariantViolation);
        }
        Ok(())
    }

    fn require_baseline_and_authorization(&self) -> Result<(), TransactionError> {
        if self.baseline.is_none() || self.authorization.is_none() {
            return Err(TransactionError::StageInvariantViolation);
        }
        Ok(())
    }

    fn require_all_apply_success(&self) -> Result<(), TransactionError> {
        if self.apply_records.len() != self.plan.actions().len()
            || self
                .apply_records
                .iter()
                .any(|record| record.outcome != ApplyOutcome::Success)
        {
            return Err(TransactionError::IncompleteApplyProof);
        }
        Ok(())
    }

    pub(crate) fn validate_apply_record(
        &self,
        record: &ApplyRecord,
    ) -> Result<(), TransactionError> {
        if self.plan.action_by_id(&record.action_id).is_none() {
            return Err(TransactionError::UnknownApplyAction(
                record.action_id.clone(),
            ));
        }
        if self
            .apply_records
            .iter()
            .any(|existing| existing.action_id == record.action_id)
        {
            return Err(TransactionError::DuplicateApplyRecord(
                record.action_id.clone(),
            ));
        }
        if record.detail.trim().is_empty() {
            return Err(TransactionError::EmptyExecutionDetail(
                record.action_id.clone(),
            ));
        }
        Ok(())
    }

    pub(crate) fn successful_applied_ids(&self) -> BTreeSet<String> {
        self.apply_records
            .iter()
            .filter(|record| record.outcome == ApplyOutcome::Success)
            .map(|record| record.action_id.clone())
            .collect()
    }

    pub(crate) fn require_successful_rollback_records(
        &self,
        changed: &BTreeSet<String>,
    ) -> Result<(), TransactionError> {
        let record_ids = self
            .rollback_records
            .iter()
            .filter(|record| record.outcome == ApplyOutcome::Success)
            .map(|record| record.action_id.clone())
            .collect::<BTreeSet<_>>();
        if &record_ids != changed
            || self
                .rollback_records
                .iter()
                .any(|record| record.outcome != ApplyOutcome::Success)
        {
            return Err(TransactionError::IncompleteRollbackProof);
        }
        Ok(())
    }

    fn validate_resume_results(&self, require_pass: bool) -> Result<(), TransactionError> {
        let baseline = self
            .baseline
            .as_ref()
            .ok_or(TransactionError::MissingBaseline)?;
        let reboot_checkpoint = self
            .reboot_checkpoint
            .as_ref()
            .ok_or(TransactionError::MissingRebootCheckpoint)?;
        reboot_checkpoint.validate_for_checkpoint(self)?;
        validate_result_set(
            &self.resume_results,
            &reboot_checkpoint.expected_post_reboot,
            baseline,
            require_pass,
        )
    }

    fn validate_postcondition_results(&self, require_pass: bool) -> Result<(), TransactionError> {
        let baseline = self
            .baseline
            .as_ref()
            .ok_or(TransactionError::MissingBaseline)?;
        validate_result_set(
            &self.verification_results,
            &self.plan.postconditions(),
            baseline,
            require_pass,
        )
    }

    fn validate_rollback_results(
        &self,
        changed: &BTreeSet<String>,
        require_pass: bool,
    ) -> Result<(), TransactionError> {
        let baseline = self
            .baseline
            .as_ref()
            .ok_or(TransactionError::MissingBaseline)?;
        validate_result_set(
            &self.rollback_results,
            &self.plan.rollback_predicates_for(changed),
            baseline,
            require_pass,
        )
    }

    fn validate_event_log(&self) -> Result<(), TransactionError> {
        if self.events.is_empty() {
            return Err(TransactionError::MissingEventLog);
        }
        for (index, event) in self.events.iter().enumerate() {
            if event.sequence != (index + 1) as u64 || event.message.trim().is_empty() {
                return Err(TransactionError::InvalidEventLog);
            }
            if index == 0 {
                if event.stage != TransactionStage::Planned {
                    return Err(TransactionError::InvalidEventLog);
                }
            } else if !valid_event_transition(self.events[index - 1].stage, event.stage) {
                return Err(TransactionError::InvalidEventLog);
            }
        }
        if self.events.last().map(|event| event.stage) != Some(self.stage) {
            return Err(TransactionError::InvalidEventLog);
        }
        Ok(())
    }

    pub(crate) fn transition(&mut self, stage: TransactionStage, message: &str) {
        self.stage = stage;
        self.record_event(message);
    }

    pub(crate) fn record_event(&mut self, message: &str) {
        self.events.push(TransactionEvent {
            sequence: (self.events.len() + 1) as u64,
            stage: self.stage,
            message: message.to_string(),
        });
    }
}

fn valid_event_transition(from: TransactionStage, to: TransactionStage) -> bool {
    use TransactionStage::*;
    matches!(
        (from, to),
        (Planned, Planned)
            | (Planned, BaselineCaptured)
            | (BaselineCaptured, Authorized)
            | (Authorized, Applying)
            | (Applying, Applying)
            | (Applying, AwaitingReboot)
            | (Applying, Verifying)
            | (Applying, RollingBack)
            | (Applying, Failed)
            | (AwaitingReboot, Verifying)
            | (AwaitingReboot, Blocked)
            | (Blocked, Verifying)
            | (Blocked, RollingBack)
            | (Blocked, Failed)
            | (Verifying, Complete)
            | (Verifying, RollingBack)
            | (Verifying, Failed)
            | (RollingBack, RollingBack)
            | (RollingBack, RolledBack)
            | (RollingBack, Failed)
    )
}

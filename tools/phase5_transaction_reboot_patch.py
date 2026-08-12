#!/usr/bin/env python3
"""One-shot Phase 5 prerequisite: runtime apply/rollback reboot evidence."""
from pathlib import Path


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise SystemExit(f"expected one anchor in {path}: {old[:100]!r}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def main() -> None:
    plan = Path("crates/neo-transaction/src/plan.rs")
    replace_once(
        plan,
        '''    pub(crate) fn action_by_id(&self, action_id: &str) -> Option<&TransactionAction> {
''',
        '''    pub fn action_by_id(&self, action_id: &str) -> Option<&TransactionAction> {
''',
    )
    replace_once(
        plan,
        '''    pub(crate) fn rollback_predicates_for(
        &self,
        action_ids: &BTreeSet<String>,
    ) -> Vec<VerificationPredicate> {
''',
        '''    pub(crate) fn rollback_targets_for(
        &self,
        action_ids: &BTreeSet<String>,
    ) -> BTreeSet<StateTarget> {
        self.actions
            .iter()
            .filter(|transaction_action| action_ids.contains(&transaction_action.action.id))
            .flat_map(|transaction_action| match &transaction_action.rollback {
                RollbackPlan::Reversible { restore_targets, .. } => restore_targets.as_slice(),
                RollbackPlan::Irreversible { .. } => &[],
            })
            .cloned()
            .collect()
    }

    pub(crate) fn rollback_predicates_for(
        &self,
        action_ids: &BTreeSet<String>,
    ) -> Vec<VerificationPredicate> {
''',
    )
    replace_once(
        plan,
        '''pub struct ApplyRecord {
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
''',
        '''pub struct ApplyRecord {
    pub action_id: String,
    pub outcome: ApplyOutcome,
    pub detail: String,
    #[serde(default)]
    pub reboot_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackRecord {
    pub action_id: String,
    pub outcome: ApplyOutcome,
    pub detail: String,
    #[serde(default)]
    pub reboot_required: bool,
}
''',
    )
    replace_once(
        plan,
        '''    RollingBack,
    Complete,
''',
        '''    RollingBack,
    AwaitingRollbackReboot,
    Complete,
''',
    )

    checkpoint = Path("crates/neo-transaction/src/checkpoint.rs")
    replace_once(
        checkpoint,
        '''impl RebootCheckpoint {
    pub(crate) fn for_checkpoint(checkpoint: &TransactionCheckpoint) -> Self {
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

    pub(crate) fn validate_for_checkpoint(
        &self,
        checkpoint: &TransactionCheckpoint,
    ) -> Result<(), TransactionError> {
        let expected = Self::for_checkpoint(checkpoint);
        if self != &expected {
            return Err(TransactionError::RebootCheckpointMismatch);
        }
        Ok(())
    }
}
''',
        '''impl RebootCheckpoint {
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
''',
    )
    replace_once(
        checkpoint,
        '''    pub fn begin_apply(&mut self) -> Result<(), TransactionError> {
        self.require_stage(TransactionStage::Authorized)?;
        self.validate()?;
        self.transition(TransactionStage::Applying, "apply phase opened");
        self.validate()
    }

    pub fn record_apply_result(&mut self, record: ApplyRecord) -> Result<(), TransactionError> {
''',
        '''    pub fn begin_apply(&mut self) -> Result<(), TransactionError> {
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
            return Err(TransactionError::DuplicateApplyRecord(action_id.to_string()));
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
''',
    )
    replace_once(
        checkpoint,
        '''        if self.apply_records.len() == self.plan.actions().len() {
            if self.plan.requires_reboot() {
                self.reboot_checkpoint = Some(RebootCheckpoint::for_checkpoint(self));
''',
        '''        if self.apply_records.len() == self.plan.actions().len() {
            if self.effective_apply_reboot_required() {
                self.reboot_checkpoint = Some(RebootCheckpoint::for_apply_checkpoint(self));
''',
    )
    replace_once(
        checkpoint,
        '''        if record.outcome == ApplyOutcome::Failure {
            self.transition(
                TransactionStage::Failed,
                "rollback application failed; recovery remains unresolved",
            );
        }
        self.validate()
    }

    pub fn verify_rollback(
''',
        '''        if record.outcome == ApplyOutcome::Failure {
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
''',
    )

    invariants = Path("crates/neo-transaction/src/invariants.rs")
    replace_once(
        invariants,
        '''                if !self.plan.requires_reboot() {
                    return Err(TransactionError::UnexpectedRebootCheckpoint);
                }
''',
        '''                if !self.effective_apply_reboot_required() {
                    return Err(TransactionError::UnexpectedRebootCheckpoint);
                }
''',
    )
    replace_once(
        invariants,
        '''                if self.plan.requires_reboot() {
                    self.validate_resume_results(true)?;
''',
        '''                if self.effective_apply_reboot_required() {
                    self.validate_resume_results(true)?;
''',
    )
    replace_once(
        invariants,
        '''                if self.plan.requires_reboot() {
                    self.validate_resume_results(true)?;
''',
        '''                if self.effective_apply_reboot_required() {
                    self.validate_resume_results(true)?;
''',
    )
    replace_once(
        invariants,
        '''            TransactionStage::RolledBack => {
''',
        '''            TransactionStage::AwaitingRollbackReboot => {
                self.require_baseline_and_authorization()?;
                let changed = self.successful_applied_ids();
                if changed.is_empty()
                    || !self.plan.all_reversible(&changed)
                    || !self.effective_rollback_reboot_required()
                {
                    return Err(TransactionError::StageInvariantViolation);
                }
                self.require_successful_rollback_records(&changed)?;
                self.reboot_checkpoint
                    .as_ref()
                    .ok_or(TransactionError::MissingRebootCheckpoint)?
                    .validate_for_checkpoint(self)?;
                if !self.rollback_results.is_empty() {
                    return Err(TransactionError::StageInvariantViolation);
                }
            }
            TransactionStage::RolledBack => {
''',
    )
    replace_once(
        invariants,
        '''                self.require_successful_rollback_records(&changed)?;
                self.validate_rollback_results(&changed, true)?;
''',
        '''                self.require_successful_rollback_records(&changed)?;
                if self.effective_rollback_reboot_required() {
                    self.reboot_checkpoint
                        .as_ref()
                        .ok_or(TransactionError::MissingRebootCheckpoint)?
                        .validate_for_checkpoint(self)?;
                }
                self.validate_rollback_results(&changed, true)?;
''',
    )
    replace_once(
        invariants,
        '''                if !self.plan.requires_reboot() {
                    return Err(TransactionError::StageInvariantViolation);
                }
''',
        '''                if !self.effective_apply_reboot_required() {
                    return Err(TransactionError::StageInvariantViolation);
                }
''',
    )
    replace_once(
        invariants,
        '''            | (RollingBack, RolledBack)
            | (RollingBack, Failed)
''',
        '''            | (RollingBack, RolledBack)
            | (RollingBack, AwaitingRollbackReboot)
            | (RollingBack, Failed)
            | (AwaitingRollbackReboot, RolledBack)
            | (AwaitingRollbackReboot, Failed)
''',
    )

    tests = Path("crates/neo-transaction/src/tests.rs")
    text = tests.read_text(encoding="utf-8")
    text = text.replace(
        'detail: "future executor reported success".to_string(),\n        })',
        'detail: "future executor reported success".to_string(),\n            reboot_required: false,\n        })',
    )
    text = text.replace(
        'detail: "future executor restored captured value".to_string(),\n        })',
        'detail: "future executor restored captured value".to_string(),\n            reboot_required: false,\n        })',
    )
    marker = "\n#[test]\nfn required_reboot_must_be_proven_before_continuation() {"
    added = '''
#[test]
fn runtime_apply_reboot_escalates_possible_plan() {
    let mut checkpoint = authorized_checkpoint();
    checkpoint.begin_apply().unwrap();
    checkpoint
        .record_apply_result(ApplyRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "backend discovered reboot".to_string(),
            reboot_required: true,
        })
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::AwaitingReboot);
}

#[test]
fn rollback_runtime_reboot_waits_before_restoration_proof() {
    let mut checkpoint = authorized_checkpoint();
    checkpoint.begin_apply().unwrap();
    checkpoint
        .record_apply_result(ApplyRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "future executor reported success".to_string(),
            reboot_required: false,
        })
        .unwrap();
    checkpoint
        .verify_postconditions(vec![Observation {
            target: target(),
            value: ObservedValue::Present("broken".to_string()),
        }])
        .unwrap();
    checkpoint
        .record_rollback_result(RollbackRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "rollback backend requested reboot".to_string(),
            reboot_required: true,
        })
        .unwrap();
    assert_eq!(
        checkpoint.stage(),
        TransactionStage::AwaitingRollbackReboot
    );
    checkpoint
        .resume_after_rollback_reboot(vec![Observation {
            target: target(),
            value: ObservedValue::Present("0".to_string()),
        }])
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::RolledBack);
}
'''
    if text.count(marker) != 1:
        raise SystemExit("transaction tests insertion anchor mismatch")
    tests.write_text(text.replace(marker, added + marker, 1), encoding="utf-8")


if __name__ == "__main__":
    main()

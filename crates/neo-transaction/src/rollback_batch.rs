use crate::{
    ApplyOutcome, RebootCheckpoint, RollbackRecord, TransactionCheckpoint, TransactionError,
    TransactionEvent, TransactionStage,
};
use std::collections::BTreeSet;

impl TransactionCheckpoint {
    /// Record the complete rollback-attempt result set for every changed action at once.
    ///
    /// This preserves the legacy single-record API while allowing an executor to attempt every
    /// independent restoration before the transaction becomes terminal on any failed restore.
    pub fn record_rollback_results_batch(
        &mut self,
        records: Vec<RollbackRecord>,
    ) -> Result<(), TransactionError> {
        self.require_stage(TransactionStage::RollingBack)?;
        self.validate()?;
        if !self.rollback_records.is_empty() {
            return Err(TransactionError::IncompleteRollbackProof);
        }

        let changed = self.changed_action_ids();
        let mut record_ids = BTreeSet::new();
        for record in &records {
            if !changed.contains(&record.action_id) {
                return Err(TransactionError::UnknownRollbackAction(
                    record.action_id.clone(),
                ));
            }
            let transaction_action = self
                .plan
                .action_by_id(&record.action_id)
                .ok_or_else(|| TransactionError::UnknownRollbackAction(record.action_id.clone()))?;
            if !transaction_action.rollback.is_reversible() {
                return Err(TransactionError::IrreversibleRollbackAttempt(
                    record.action_id.clone(),
                ));
            }
            if !record_ids.insert(record.action_id.clone()) {
                return Err(TransactionError::DuplicateRollbackRecord(
                    record.action_id.clone(),
                ));
            }
            if record.detail.trim().is_empty() {
                return Err(TransactionError::EmptyExecutionDetail(
                    record.action_id.clone(),
                ));
            }
        }
        if record_ids != changed {
            return Err(TransactionError::IncompleteRollbackProof);
        }

        self.rollback_records = records;
        for action_id in self
            .rollback_records
            .iter()
            .map(|record| record.action_id.clone())
            .collect::<Vec<_>>()
        {
            self.events.push(TransactionEvent {
                sequence: self.events.len() as u64 + 1,
                stage: self.stage,
                message: format!("rollback result recorded for {action_id}"),
            });
        }

        if self
            .rollback_records
            .iter()
            .any(|record| record.outcome == ApplyOutcome::Failure)
        {
            self.stage = TransactionStage::Failed;
            self.events.push(TransactionEvent {
                sequence: self.events.len() as u64 + 1,
                stage: self.stage,
                message: "rollback application failed after all changed actions were attempted"
                    .to_string(),
            });
            return self.validate();
        }

        if self.effective_rollback_reboot_required() {
            self.reboot_checkpoint = Some(RebootCheckpoint::for_rollback_checkpoint(self));
            self.stage = TransactionStage::AwaitingRollbackReboot;
            self.events.push(TransactionEvent {
                sequence: self.events.len() as u64 + 1,
                stage: self.stage,
                message: "rollback requires reboot before restoration can be verified".to_string(),
            });
        }
        self.validate()
    }
}

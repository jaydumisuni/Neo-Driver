//! Transaction, verification, reboot/resume, and rollback contracts for Neo Driver.
//!
//! Phase 4 intentionally contains no Windows mutator. It proves the state machine
//! future executors must obey before they are allowed to change a machine.

mod checkpoint;
mod error;
mod invariants;
mod plan;
mod rollback_batch;
mod state;
mod verification;

pub use checkpoint::{RebootCheckpoint, TransactionCheckpoint, TransactionEvent};
pub use error::TransactionError;
pub use plan::{
    ActionAcknowledgement, ApplyOutcome, ApplyRecord, RollbackPlan, RollbackRecord,
    TransactionAction, TransactionAuthorization, TransactionPlan, TransactionStage,
};
pub use state::{
    BaselineSnapshot, CapturedState, CapturedValue, Observation, ObservedValue, StateTarget,
    StateTargetKind, VerificationExpectation, VerificationPredicate, VerificationResult,
    VerificationStatus,
};

#[cfg(test)]
mod rollback_batch_tests;
#[cfg(test)]
mod tests;

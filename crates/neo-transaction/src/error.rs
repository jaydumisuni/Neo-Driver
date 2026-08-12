use crate::plan::TransactionStage;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransactionError {
    #[error("transaction id is required")]
    MissingTransactionId,
    #[error("transaction revision must be greater than zero")]
    InvalidRevision,
    #[error("mission id is required")]
    MissingMissionId,
    #[error("transaction plan requires at least one action")]
    EmptyTransactionPlan,
    #[error("transaction action is not machine-changing: {0}")]
    NonMutatingTransactionAction(String),
    #[error("rejected action cannot enter a transaction: {0}")]
    RejectedAction(String),
    #[error("transaction action has no postconditions: {0}")]
    MissingPostconditions(String),
    #[error("duplicate transaction action id: {0}")]
    DuplicateTransactionAction(String),
    #[error("state target key is required")]
    EmptyStateTarget,
    #[error("duplicate state target: {0}")]
    DuplicateStateTarget(String),
    #[error("predicate id is required")]
    EmptyPredicateId,
    #[error("duplicate verification predicate id: {0}")]
    DuplicatePredicateId(String),
    #[error("rollback contract conflicts with PlannedAction metadata: {0}")]
    RollbackContractMismatch(String),
    #[error("reversible action has no captured rollback target: {0}")]
    MissingRollbackSnapshot(String),
    #[error("rollback targets do not exactly match snapshot targets: {0}")]
    RollbackTargetMismatch(String),
    #[error("reversible action has no rollback verification: {0}")]
    MissingRollbackVerification(String),
    #[error("rollback verification must prove captured baseline targets: {0}")]
    InvalidRollbackVerification(String),
    #[error("duplicate baseline target: {0}")]
    DuplicateBaselineTarget(String),
    #[error("state target is captured by more than one transaction action: {0}")]
    OverlappingSnapshotTarget(String),
    #[error("baseline snapshot does not exactly cover the transaction plan")]
    BaselineCoverageMismatch,
    #[error("rollback baseline unavailable for {target}: {reason}")]
    RollbackBaselineUnavailable { target: String, reason: String },
    #[error("authorization fingerprint does not match the exact plan")]
    AuthorizationFingerprintMismatch,
    #[error("authorization must approve exactly every action in the transaction")]
    AuthorizationCoverageMismatch,
    #[error("manual override acknowledgement is missing")]
    MissingManualOverride,
    #[error("HIGH/EXPERT risk acknowledgement is missing")]
    MissingHighRiskAcknowledgement,
    #[error("irreversible action acknowledgement is missing")]
    MissingIrreversibleAcknowledgement,
    #[error("irreversible acknowledgement has no reason for action: {0}")]
    EmptyIrreversibleAcknowledgement(String),
    #[error("authorization id is empty")]
    EmptyAuthorizationId,
    #[error("duplicate authorization id: {0}")]
    DuplicateAuthorizationId(String),
    #[error("authorization references an action outside the exact plan")]
    UnknownAuthorizationId,
    #[error("checkpoint fingerprint does not match its embedded plan")]
    CheckpointFingerprintMismatch,
    #[error("invalid transaction stage; expected {expected:?}, actual {actual:?}")]
    InvalidStageTransition {
        expected: TransactionStage,
        actual: TransactionStage,
    },
    #[error("checkpoint fields violate the transaction stage invariant")]
    StageInvariantViolation,
    #[error("transaction baseline is missing")]
    MissingBaseline,
    #[error("apply record references unknown action: {0}")]
    UnknownApplyAction(String),
    #[error("duplicate apply record for action: {0}")]
    DuplicateApplyRecord(String),
    #[error("execution record detail is required for action: {0}")]
    EmptyExecutionDetail(String),
    #[error("all apply records must prove success before this stage")]
    IncompleteApplyProof,
    #[error("required reboot checkpoint is missing")]
    MissingRebootCheckpoint,
    #[error("persisted reboot checkpoint does not match the exact plan")]
    RebootCheckpointMismatch,
    #[error("reboot checkpoint exists where no required reboot is declared")]
    UnexpectedRebootCheckpoint,
    #[error("observation is outside the requested verification set: {0}")]
    UnexpectedObservation(String),
    #[error("duplicate observation for target: {0}")]
    DuplicateObservation(String),
    #[error("verification result coverage does not match required predicates")]
    VerificationCoverageMismatch,
    #[error("verification result predicate was altered: {0}")]
    VerificationPredicateMismatch(String),
    #[error("required verification predicate is not proven")]
    RequiredVerificationNotProven,
    #[error("checkpoint claims a blocked state even though required verification passes")]
    ExpectedUnprovenVerification,
    #[error("rollback record references an action that was not successfully changed: {0}")]
    UnknownRollbackAction(String),
    #[error("duplicate rollback record for action: {0}")]
    DuplicateRollbackRecord(String),
    #[error("rollback requested for irreversible action: {0}")]
    IrreversibleRollbackAttempt(String),
    #[error("successful rollback records do not cover every changed action")]
    IncompleteRollbackProof,
    #[error("transaction event log is missing")]
    MissingEventLog,
    #[error("transaction event log sequence/stage is invalid")]
    InvalidEventLog,
    #[error("core plan validation failed: {0}")]
    CorePlan(#[from] neo_core::PlanValidationError),
    #[error("transaction JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

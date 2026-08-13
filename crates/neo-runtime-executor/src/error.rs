use neo_runtime::{RuntimeComponent, RuntimeState};
use neo_transaction::TransactionError;
use neo_vault::VaultError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeExecutorError {
    #[error("mission id cannot be empty")]
    MissingMissionId,
    #[error("transaction id cannot be empty")]
    MissingTransactionId,
    #[error("Phase 6 assessment failed: {0}")]
    Assessment(String),
    #[error("catalogue validation failed: {0}")]
    Catalogue(String),
    #[error("profile does not contain runtime component {0:?}")]
    MissingRecommendation(RuntimeComponent),
    #[error("runtime component {component:?} has no certified Phase 6 action")]
    MissingCertifiedAction { component: RuntimeComponent },
    #[error("runtime inventory has no observation for {component:?}")]
    MissingObservation { component: RuntimeComponent },
    #[error("runtime recommendation for {component:?} is not certified")]
    RecommendationNotCertified { component: RuntimeComponent },
    #[error("runtime recommendation for {component:?} has no exact package id")]
    MissingPackageId { component: RuntimeComponent },
    #[error("runtime package not found: {0}")]
    PackageNotFound(String),
    #[error("package is not a runtime package: {0}")]
    PackageNotRuntime(String),
    #[error("runtime package has no Phase 8 execution metadata: {0}")]
    MissingExecutionSpec(String),
    #[error("runtime package still has dependency/conflict edges: {0}")]
    DependencyClosureRequired(String),
    #[error("runtime package requests boot/security-state mutation: {0}")]
    SecurityMutationBlocked(String),
    #[error("runtime operation {operation} is incompatible with baseline state {state:?}")]
    OperationStateMismatch {
        operation: &'static str,
        state: RuntimeState,
    },
    #[error("runtime repair package has no explicit repair arguments: {0}")]
    MissingRepairArguments(String),
    #[error("Phase 6 action does not match Phase 8 runtime authority: {0}")]
    ActionMismatch(String),
    #[error("runtime execution plan is invalid: {0}")]
    InvalidPlan(String),
    #[error("runtime host preflight drifted: {0}")]
    HostDrift(String),
    #[error("runtime component baseline drifted before mutation: {0}")]
    BaselineDrift(String),
    #[error("runtime payload is unavailable: {0:?}")]
    PayloadUnavailable(PathBuf),
    #[error("runtime host failed before process creation: {0}")]
    Host(String),
    #[error("runtime execution is Windows-only")]
    UnsupportedPlatform,
    #[error("runtime staging cleanup failed after execution: {0}")]
    Cleanup(String),
    #[error("runtime JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("runtime I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Vault(#[from] VaultError),
    #[error(transparent)]
    Transaction(#[from] TransactionError),
}

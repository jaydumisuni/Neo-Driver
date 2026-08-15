use thiserror::Error;

#[derive(Debug, Error)]
pub enum TweakExecutionError {
    #[error(transparent)]
    State(#[from] neo_state_plan::StatePlanError),
    #[error(transparent)]
    Transaction(#[from] neo_transaction::TransactionError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("unsupported Phase 11 tweak: {0}")]
    UnsupportedTweak(String),
    #[error("tweak target does not match the curated Phase 11 binding: {0}")]
    TargetMismatch(String),
    #[error("Phase 11 requires the exact approved forward DWORD for this curated tweak: {0}")]
    UnsupportedOperation(String),
    #[error("Phase 11 mutation requires certified tweak evidence: {0}")]
    NonCertifiedTweak(String),
    #[error("invalid tweak execution request: {0}")]
    InvalidRequest(String),
    #[error("all selected tweaks are already satisfied")]
    NothingToChange,
    #[error("registry state drifted before authority or apply: {0}")]
    BaselineDrift(String),
    #[error("registry value exists with an unsupported type or size: {0}")]
    UnsupportedRegistryState(String),
    #[error("registry operation failed: {0}")]
    Registry(String),
    #[error("Phase 11 Windows registry execution is unavailable on this platform")]
    UnsupportedPlatform,
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DebloatHistoryError {
    #[error("Phase 17 Windows restore-readiness probing is supported only on Windows")]
    UnsupportedPlatform,
    #[error("completed-removal evidence is not eligible for a durable receipt: {0}")]
    IncompleteRemoval(String),
    #[error("invalid Phase 17 removal receipt: {0}")]
    InvalidReceipt(String),
    #[error("post-success restore is not ready: {0}")]
    RestoreNotReady(String),
    #[error("the exact removed package is already registered for the current user")]
    AlreadyRestored,
    #[error("current AppX state conflicts with deterministic restore: {0}")]
    InventoryConflict(String),
    #[error(transparent)]
    Plan(#[from] neo_debloat_plan::DebloatPlanError),
    #[error(transparent)]
    Transaction(#[from] neo_transaction::TransactionError),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}

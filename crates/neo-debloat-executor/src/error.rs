use thiserror::Error;

#[derive(Debug, Error)]
pub enum DebloatExecutionError {
    #[error("Phase 16 AppX execution is supported only on Windows")]
    UnsupportedPlatform,
    #[error("invalid Phase 15 prepared state: {0}")]
    InvalidPreparedState(String),
    #[error("captured AppX baseline drifted before mutation: {0}")]
    BaselineDrift(String),
    #[error("Windows AppX deployment operation failed: {0}")]
    NativeDeployment(String),
    #[error("AppX state observation failed: {0}")]
    Observation(String),
    #[error(transparent)]
    Transaction(#[from] neo_transaction::TransactionError),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}

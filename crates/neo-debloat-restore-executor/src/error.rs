use thiserror::Error;

#[derive(Debug, Error)]
pub enum DebloatRestoreExecutionError {
    #[error("Phase 18 AppX restore execution is supported only on Windows")]
    UnsupportedPlatform,
    #[error("invalid Phase 17 prepared restore state: {0}")]
    InvalidPreparedState(String),
    #[error("restore-time AppX baseline drifted before mutation: {0}")]
    BaselineDrift(String),
    #[error("local staged AppX restore route drifted before mutation: {0}")]
    RestoreRouteDrift(String),
    #[error("native AppX restore deployment failed: {0}")]
    NativeDeployment(String),
    #[error("AppX restore state observation failed: {0}")]
    Observation(String),
    #[error(transparent)]
    Plan(#[from] neo_debloat_plan::DebloatPlanError),
    #[error(transparent)]
    Transaction(#[from] neo_transaction::TransactionError),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}

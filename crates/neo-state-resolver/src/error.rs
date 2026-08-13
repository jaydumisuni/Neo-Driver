use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateResolverError {
    #[error("invalid binding field: {0}")]
    InvalidField(&'static str),
    #[error("duplicate binding target: {0}")]
    DuplicateBinding(String),
    #[error("missing binding for target: {0}")]
    MissingBinding(String),
    #[error("selected tweak does not exist: {0}")]
    UnknownTweak(String),
    #[error("state-plan validation failed: {0}")]
    StatePlan(String),
    #[error("command evidence could not be normalized: {0}")]
    Evidence(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

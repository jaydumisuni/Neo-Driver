use thiserror::Error;

#[derive(Debug, Error)]
pub enum StatePlanError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("transaction error: {0}")]
    Transaction(#[from] neo_transaction::TransactionError),
    #[error("{0} cannot be empty")]
    EmptyField(&'static str),
    #[error("invalid lowercase tweak id '{0}'")]
    InvalidId(String),
    #[error("invalid tweak target '{0}'")]
    InvalidTarget(String),
    #[error("duplicate tweak id '{0}'")]
    DuplicateId(String),
    #[error("duplicate tweak target '{0}'")]
    DuplicateTarget(String),
    #[error("high-risk tweak '{0}' cannot be preselected")]
    HighRiskPreselected(String),
    #[error("non-certified tweak '{0}' cannot be preselected")]
    NonCertifiedPreselected(String),
    #[error("unsafe recommendation tweak '{0}' cannot be preselected")]
    UnsafeRecommendationPreselected(String),
    #[error("duplicate tweak observation '{0}'")]
    DuplicateObservation(String),
    #[error("at least one tweak must be explicitly selected")]
    EmptySelection,
    #[error("duplicate selected tweak '{0}'")]
    DuplicateSelection(String),
    #[error("unknown selected tweak '{0}'")]
    UnknownTweak(String),
    #[error("rejected tweak '{0}' cannot become transaction authority")]
    RejectedTweak(String),
    #[error("selected tweak '{0}' has no exact current-state observation")]
    MissingObservation(String),
    #[error("selected tweak '{tweak_id}' current-state observation is unavailable: {reason}")]
    UnavailableObservation { tweak_id: String, reason: String },
    #[error("all selected tweaks are already in the requested state")]
    NoChangesRequired,
}

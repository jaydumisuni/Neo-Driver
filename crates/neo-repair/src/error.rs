use neo_transaction::TransactionError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepairError {
    #[error("Phase 21 repair support is available only on Windows")]
    UnsupportedPlatform,
    #[error("trusted Windows system directory could not be resolved: {0}")]
    WindowsDirectory(String),
    #[error("invalid Phase 21 repair request: {0}")]
    InvalidRequest(String),
    #[error("repair or feature state is unavailable: {0}")]
    StateUnavailable(String),
    #[error("elevated Windows servicing authority is required")]
    ElevationRequired,
    #[error("the selected repair target does not currently require repair: {0}")]
    NothingToRepair(String),
    #[error("the selected Windows feature is already in the requested state: {0}")]
    NothingToChange(String),
    #[error("the selected Windows feature cannot use the reversible Phase 21 route: {0}")]
    FeatureNotReversible(String),
    #[error("fresh Windows state differs from the prepared Phase 21 baseline: {0}")]
    BaselineDrift(String),
    #[error("trusted Windows command failed: {0}")]
    CommandFailed(String),
    #[error("Phase 21 transaction failure: {0}")]
    Transaction(#[from] TransactionError),
}

use neo_debloat::DebloatError;
use neo_debloat_probe::DebloatProbeError;
use neo_transaction::TransactionError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DebloatPlanError {
    #[error(transparent)]
    Debloat(#[from] DebloatError),
    #[error(transparent)]
    Probe(#[from] DebloatProbeError),
    #[error(transparent)]
    Transaction(#[from] TransactionError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("Neo Phase 15 exact AppX planning is currently supported on Windows only")]
    UnsupportedPlatform,
    #[error("invalid Phase 15 request: {0}")]
    InvalidRequest(String),
    #[error("Phase 15 accepts exactly one selected debloat item per prepared transaction")]
    BatchNotSupported,
    #[error("selected item is not a Phase 13 removal candidate: {0}")]
    NotRemovalCandidate(String),
    #[error("Phase 15 mutation planning supports current-user scope only: {0}")]
    UnsupportedScope(String),
    #[error("declared restore metadata is not executable Phase 15 rollback authority: {0}")]
    RestoreNotReady(String),
    #[error("native AppX inventory failure: {0}")]
    NativeInventory(String),
    #[error("Phase 14 presence and native exact identity evidence disagree: {0}")]
    InventoryDrift(String),
    #[error("missing exact AppX identity: {0}")]
    MissingExactIdentity(String),
    #[error("ambiguous exact AppX identity: {0}")]
    AmbiguousExactIdentity(String),
    #[error("unsupported AppX package kind for controlled removal planning: {0}")]
    UnsafePackageKind(String),
}

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DebloatHistoryStoreError {
    #[error("invalid Debloat history record id: {0}")]
    InvalidRecordId(String),
    #[error("Debloat history store is unavailable at {0}")]
    StoreUnavailable(PathBuf),
    #[error("unsafe link/reparse path in Debloat history store: {0}")]
    UnsafeLink(PathBuf),
    #[error("unexpected entry in Debloat history store: {0}")]
    UnexpectedEntry(PathBuf),
    #[error("Debloat history record not found: {0}")]
    RecordNotFound(String),
    #[error("Debloat history record conflict for {0}")]
    RecordConflict(String),
    #[error("Debloat history record exceeds the {limit} byte limit: {path}")]
    RecordTooLarge { path: PathBuf, limit: u64 },
    #[error("invalid Debloat history store record: {0}")]
    InvalidRecord(String),
    #[error(transparent)]
    History(#[from] neo_debloat_history::DebloatHistoryError),
    #[error(transparent)]
    Vault(#[from] neo_vault::VaultError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

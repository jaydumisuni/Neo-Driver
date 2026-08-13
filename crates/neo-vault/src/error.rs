use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("application root must be an absolute path: {0}")]
    ApplicationRootNotAbsolute(PathBuf),
    #[error("application root must already exist as a normal directory: {0}")]
    ApplicationRootUnavailable(PathBuf),
    #[error("path contains a parent traversal component: {0}")]
    ParentTraversal(PathBuf),
    #[error("invalid Neo vault segment: {0}")]
    InvalidSegment(String),
    #[error("managed path escapes NeoData: {0}")]
    OutsideManagedRoot(PathBuf),
    #[error("unsafe symlink or reparse-style path encountered: {0}")]
    UnsafeLink(PathBuf),
    #[error("staging directory is not owned by Neo: {0}")]
    UnownedStaging(PathBuf),
    #[error("staging ownership marker does not match session {session}: {path}")]
    StagingMarkerMismatch { session: String, path: PathBuf },
    #[error("another Neo import currently owns promotion lock: {0}")]
    ImportBusy(PathBuf),
    #[error("SHA-256 must contain exactly 64 hexadecimal characters")]
    InvalidSha256,
    #[error("SHA-256 mismatch for {path}: expected {expected}, observed {observed}")]
    HashMismatch {
        path: PathBuf,
        expected: String,
        observed: String,
    },
    #[error("vault destination already exists with different content: {0}")]
    DestinationConflict(PathBuf),
    #[error("source must be a regular file: {0}")]
    SourceNotFile(PathBuf),
    #[error("unsupported vault schema version {0}")]
    UnsupportedSchemaVersion(u32),
    #[error("source map contains no sources")]
    EmptySourceMap,
    #[error("duplicate source id: {0}")]
    DuplicateSourceId(String),
    #[error("duplicate repository/release/asset identity: {0}")]
    DuplicateSourceAsset(String),
    #[error("required source field is blank: {0}")]
    BlankSourceField(&'static str),
    #[error("source repository must use owner/name form: {0}")]
    InvalidRepository(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

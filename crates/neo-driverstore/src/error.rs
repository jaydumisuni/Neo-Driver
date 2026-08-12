use thiserror::Error;

#[derive(Debug, Error)]
pub enum DriverStoreError {
    #[error("catalogue package not found: {0}")]
    PackageNotFound(String),
    #[error("driver artifact not found in package {package_id}: {inf_path}")]
    ArtifactNotFound { package_id: String, inf_path: String },
    #[error("selected driver artifact is not catalogue-verified")]
    UnverifiedArtifact,
    #[error("selected driver artifact has no expected signer")]
    MissingExpectedSigner,
    #[error("driver INF path must be a relative path contained by the package root")]
    UnsafeInfPath,
    #[error("driver source signature/catalogue does not match the approved catalogue evidence")]
    SignatureMismatch,
    #[error("no present device is supported by the exact selected INF")]
    NoSupportedPresentDevice,
    #[error("present supported device has no captured active driver binding: {0}")]
    MissingBaselineBinding(String),
    #[error("present supported device has no captured published INF for exact rollback: {0}")]
    MissingBaselinePublishedInf(String),
    #[error("duplicate impacted device instance: {0}")]
    DuplicateImpact(String),
    #[error("driver pre-state changed after authority; apply blocked")]
    PrestateDrift,
    #[error("driver blast radius changed after authority; apply blocked")]
    ImpactDrift,
    #[error("staged package identity does not match the approved source package")]
    StagedPackageMismatch,
    #[error("Windows driver policy result is not proven")]
    PolicyUnsatisfied,
    #[error("unexpected device binding changed outside the authorized impact set: {0}")]
    UnexpectedBindingChange(String),
    #[error("rollback could not restore the captured driver binding: {0}")]
    RollbackBindingFailure(String),
    #[error("driver store state could not be restored")]
    DriverStoreRestoreFailure,
    #[error("driver action is not present in the transaction checkpoint")]
    TransactionActionMismatch,
    #[error("driver plan fingerprint is not bound into the transaction action evidence")]
    DriverPlanFingerprintMismatch,
    #[error("driver operation is not supported on this host: {0}")]
    UnsupportedHost(String),
    #[error("Windows backend error: {0}")]
    Windows(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("catalogue error: {0}")]
    Catalogue(#[from] neo_catalogue::CatalogueError),
    #[error("matcher error: {0}")]
    Matcher(#[from] neo_match::MatchError),
    #[error("transaction error: {0}")]
    Transaction(#[from] neo_transaction::TransactionError),
}

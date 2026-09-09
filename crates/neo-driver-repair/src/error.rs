use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DriverRepairError {
    #[error("driver repair assessment is supported only on Windows")]
    UnsupportedPlatform,
    #[error("driver evidence is invalid: {0}")]
    InvalidEvidence(String),
    #[error("duplicate device instance ID in repair evidence: {0}")]
    DuplicateDevice(String),
    #[error("Driver Store package evidence exists without an exact active published INF: {0}")]
    PackageWithoutBinding(String),
    #[error("Driver Store package does not match the active published INF for device: {0}")]
    PackageMismatch(String),
    #[error("driver host evidence failed: {0}")]
    DriverHost(String),
    #[error("driver repair evidence serialization failed: {0}")]
    Serialization(String),
}

impl From<neo_driverstore::DriverStoreError> for DriverRepairError {
    fn from(value: neo_driverstore::DriverStoreError) -> Self {
        Self::DriverHost(value.to_string())
    }
}

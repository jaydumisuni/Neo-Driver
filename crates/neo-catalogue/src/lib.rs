//! Typed package catalogue contracts for Neo Driver.
//!
//! Phase 2 validates package identity, provenance, applicability, signatures,
//! dependencies, conflicts, and security/reboot requirements. It does not
//! download or install packages.

use neo_device::OrderedDeviceIds;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageKind {
    InfDriverBundle,
    Runtime,
    TechnicianComponent,
    Application,
    WindowsFeature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedistributionPolicy {
    Allowed,
    VendorDownloadOnly,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureStatus {
    Verified,
    Unsigned,
    Invalid,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RequiredState {
    #[default]
    Unchanged,
    Enabled,
    Disabled,
}

impl RequiredState {
    pub fn changes_state(self) -> bool {
        self != Self::Unchanged
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RebootRequirement {
    None,
    Recommended,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub source_name: String,
    #[serde(default)]
    pub source_url: Option<String>,
    pub sha256: String,
    pub redistribution: RedistributionPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureEvidence {
    pub status: SignatureStatus,
    #[serde(default)]
    pub signer: Option<String>,
    #[serde(default)]
    pub verification_note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsApplicability {
    #[serde(default)]
    pub architectures: Vec<String>,
    #[serde(default)]
    pub minimum_build: Option<u32>,
    #[serde(default)]
    pub maximum_build: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverArtifact {
    pub inf_path: String,
    #[serde(default)]
    pub ids: OrderedDeviceIds,
    #[serde(default)]
    pub catalog_files: Vec<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub driver_version: Option<String>,
    #[serde(default)]
    pub driver_date: Option<String>,
    pub signature: SignatureEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SecurityRequirements {
    #[serde(default = "unchanged")]
    pub test_signing: RequiredState,
    #[serde(default = "unchanged")]
    pub no_integrity_checks: RequiredState,
    #[serde(default = "unchanged")]
    pub secure_boot: RequiredState,
    #[serde(default = "unchanged")]
    pub memory_integrity: RequiredState,
}

fn unchanged() -> RequiredState {
    RequiredState::Unchanged
}

impl SecurityRequirements {
    pub fn changes_boot_or_security_state(&self) -> bool {
        self.test_signing.changes_state()
            || self.no_integrity_checks.changes_state()
            || self.secure_boot.changes_state()
            || self.memory_integrity.changes_state()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageManifest {
    pub package_id: String,
    pub name: String,
    pub vendor: String,
    pub version: String,
    pub kind: PackageKind,
    pub provenance: Provenance,
    pub windows: WindowsApplicability,
    #[serde(default)]
    pub driver_artifacts: Vec<DriverArtifact>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub security: SecurityRequirements,
    pub reboot: RebootRequirement,
}

impl PackageManifest {
    pub fn validate(&self) -> Result<(), CatalogueError> {
        require_nonempty("package_id", &self.package_id)?;
        require_nonempty("name", &self.name)?;
        require_nonempty("vendor", &self.vendor)?;
        require_nonempty("version", &self.version)?;
        require_nonempty("source_name", &self.provenance.source_name)?;
        validate_sha256(&self.provenance.sha256)?;
        validate_windows(&self.windows)?;
        ensure_unique_strings("dependency", &self.dependencies)?;
        ensure_unique_strings("conflict", &self.conflicts)?;

        if self
            .dependencies
            .iter()
            .any(|value| value == &self.package_id)
        {
            return Err(CatalogueError::SelfDependency(self.package_id.clone()));
        }
        if self.conflicts.iter().any(|value| value == &self.package_id) {
            return Err(CatalogueError::SelfConflict(self.package_id.clone()));
        }
        if let Some(value) = self
            .dependencies
            .iter()
            .find(|value| self.conflicts.contains(*value))
        {
            return Err(CatalogueError::DependencyConflictOverlap(value.clone()));
        }

        if self.kind == PackageKind::InfDriverBundle && self.driver_artifacts.is_empty() {
            return Err(CatalogueError::DriverBundleWithoutArtifacts(
                self.package_id.clone(),
            ));
        }
        if self.kind != PackageKind::InfDriverBundle && !self.driver_artifacts.is_empty() {
            return Err(CatalogueError::UnexpectedDriverArtifacts(
                self.package_id.clone(),
            ));
        }

        let mut inf_paths = BTreeSet::new();
        for artifact in &self.driver_artifacts {
            validate_driver_artifact(artifact)?;
            if !inf_paths.insert(artifact.inf_path.to_ascii_lowercase()) {
                return Err(CatalogueError::DuplicateInfPath(artifact.inf_path.clone()));
            }
        }

        if self.security.changes_boot_or_security_state()
            && self.reboot != RebootRequirement::Required
        {
            return Err(CatalogueError::SecurityStateChangeWithoutRequiredReboot);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Catalogue {
    pub packages: Vec<PackageManifest>,
}

impl Catalogue {
    pub fn validate(&self) -> Result<(), CatalogueError> {
        let mut ids = BTreeSet::new();
        for package in &self.packages {
            package.validate()?;
            if !ids.insert(package.package_id.as_str()) {
                return Err(CatalogueError::DuplicatePackageId(
                    package.package_id.clone(),
                ));
            }
        }
        Ok(())
    }

    pub fn from_json_str(input: &str) -> Result<Self, CatalogueError> {
        let catalogue: Self = serde_json::from_str(input)?;
        catalogue.validate()?;
        Ok(catalogue)
    }

    pub fn read_json(path: impl AsRef<Path>) -> Result<Self, CatalogueError> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json_str(&content)
    }
}

fn validate_driver_artifact(artifact: &DriverArtifact) -> Result<(), CatalogueError> {
    require_nonempty("inf_path", &artifact.inf_path)?;
    artifact
        .ids
        .validate()
        .map_err(|error| CatalogueError::DeviceIds(error.to_string()))?;
    if artifact.ids.is_empty() {
        return Err(CatalogueError::DriverArtifactWithoutIds(
            artifact.inf_path.clone(),
        ));
    }
    ensure_unique_strings("catalog file", &artifact.catalog_files)?;
    if artifact.signature.status == SignatureStatus::Verified {
        if artifact.catalog_files.is_empty() {
            return Err(CatalogueError::VerifiedDriverWithoutCatalog(
                artifact.inf_path.clone(),
            ));
        }
        if artifact
            .signature
            .signer
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(CatalogueError::VerifiedDriverWithoutSigner(
                artifact.inf_path.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_windows(windows: &WindowsApplicability) -> Result<(), CatalogueError> {
    ensure_unique_strings("architecture", &windows.architectures)?;
    if let (Some(minimum), Some(maximum)) = (windows.minimum_build, windows.maximum_build) {
        if minimum > maximum {
            return Err(CatalogueError::InvalidBuildRange { minimum, maximum });
        }
    }
    Ok(())
}

fn validate_sha256(value: &str) -> Result<(), CatalogueError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(CatalogueError::InvalidSha256(value.to_string()));
    }
    Ok(())
}

fn require_nonempty(label: &'static str, value: &str) -> Result<(), CatalogueError> {
    if value.trim().is_empty() {
        return Err(CatalogueError::EmptyField(label));
    }
    Ok(())
}

fn ensure_unique_strings(label: &'static str, values: &[String]) -> Result<(), CatalogueError> {
    let mut seen = BTreeSet::new();
    for value in values {
        require_nonempty(label, value)?;
        if !seen.insert(value) {
            return Err(CatalogueError::DuplicateValue {
                label,
                value: value.clone(),
            });
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum CatalogueError {
    #[error("{0} cannot be empty")]
    EmptyField(&'static str),
    #[error("invalid SHA-256 value: {0}")]
    InvalidSha256(String),
    #[error("duplicate {label}: {value}")]
    DuplicateValue { label: &'static str, value: String },
    #[error("package cannot depend on itself: {0}")]
    SelfDependency(String),
    #[error("package cannot conflict with itself: {0}")]
    SelfConflict(String),
    #[error("package is both dependent on and conflicting with: {0}")]
    DependencyConflictOverlap(String),
    #[error("INF driver bundle has no driver artifacts: {0}")]
    DriverBundleWithoutArtifacts(String),
    #[error("non-INF package unexpectedly contains driver artifacts: {0}")]
    UnexpectedDriverArtifacts(String),
    #[error("driver artifact has no hardware or compatible IDs: {0}")]
    DriverArtifactWithoutIds(String),
    #[error("duplicate INF path: {0}")]
    DuplicateInfPath(String),
    #[error("verified driver artifact has no catalogue file: {0}")]
    VerifiedDriverWithoutCatalog(String),
    #[error("verified driver artifact has no signer: {0}")]
    VerifiedDriverWithoutSigner(String),
    #[error("security-state change requires reboot=required")]
    SecurityStateChangeWithoutRequiredReboot,
    #[error("invalid Windows build range: minimum {minimum} > maximum {maximum}")]
    InvalidBuildRange { minimum: u32, maximum: u32 },
    #[error("invalid device ID set: {0}")]
    DeviceIds(String),
    #[error("duplicate package ID: {0}")]
    DuplicatePackageId(String),
    #[error("catalogue JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("catalogue I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use neo_device::OpaqueDeviceId;

    fn sample_manifest() -> PackageManifest {
        PackageManifest {
            package_id: "neo.fixture.usb-driver".to_string(),
            name: "Neo Fixture USB Driver".to_string(),
            vendor: "Neo Fixture Vendor".to_string(),
            version: "1.0.0".to_string(),
            kind: PackageKind::InfDriverBundle,
            provenance: Provenance {
                source_name: "fixture".to_string(),
                source_url: Some("https://example.invalid/fixture".to_string()),
                sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .to_string(),
                redistribution: RedistributionPolicy::Unknown,
            },
            windows: WindowsApplicability {
                architectures: vec!["x64".to_string()],
                minimum_build: Some(19041),
                maximum_build: None,
            },
            driver_artifacts: vec![DriverArtifact {
                inf_path: "drivers/fixture.inf".to_string(),
                ids: OrderedDeviceIds {
                    hardware_ids: vec![OpaqueDeviceId::new(r"USB\VID_1234&PID_5678").unwrap()],
                    compatible_ids: vec![],
                },
                catalog_files: vec!["drivers/fixture.cat".to_string()],
                provider: Some("Neo Fixture Vendor".to_string()),
                driver_version: Some("1.0.0".to_string()),
                driver_date: Some("2026-01-01".to_string()),
                signature: SignatureEvidence {
                    status: SignatureStatus::Verified,
                    signer: Some("Neo Fixture Signer".to_string()),
                    verification_note: Some("fixture only".to_string()),
                },
            }],
            dependencies: vec![],
            conflicts: vec![],
            security: SecurityRequirements::default(),
            reboot: RebootRequirement::None,
        }
    }

    #[test]
    fn sample_manifest_validates() {
        assert!(sample_manifest().validate().is_ok());
    }

    #[test]
    fn verified_driver_requires_signer_and_catalog() {
        let mut manifest = sample_manifest();
        manifest.driver_artifacts[0].signature.signer = None;
        assert!(matches!(
            manifest.validate(),
            Err(CatalogueError::VerifiedDriverWithoutSigner(_))
        ));
    }

    #[test]
    fn security_state_change_requires_reboot() {
        let mut manifest = sample_manifest();
        manifest.security.test_signing = RequiredState::Enabled;
        assert!(matches!(
            manifest.validate(),
            Err(CatalogueError::SecurityStateChangeWithoutRequiredReboot)
        ));
    }

    #[test]
    fn duplicate_applicability_ids_fail_closed() {
        let mut manifest = sample_manifest();
        let duplicate = manifest.driver_artifacts[0].ids.hardware_ids[0].clone();
        manifest.driver_artifacts[0]
            .ids
            .hardware_ids
            .push(duplicate);
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn technician_component_is_not_forced_into_inf_semantics() {
        let mut manifest = sample_manifest();
        manifest.kind = PackageKind::TechnicianComponent;
        manifest.driver_artifacts.clear();
        assert!(manifest.validate().is_ok());
    }
}

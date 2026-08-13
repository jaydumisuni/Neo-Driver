//! Typed package catalogue contracts for Neo Driver.
//!
//! Phase 3 refines INF applicability into per-model entries so deterministic
//! matching can preserve Windows identifier-score semantics. The catalogue
//! remains read-only and contains no download or install authority by itself.

use neo_device::OpaqueDeviceId;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeInstallerKind {
    Exe,
    Msi,
}

impl RuntimeInstallerKind {
    pub fn staging_extension(self) -> &'static str {
        match self {
            Self::Exe => "exe",
            Self::Msi => "msi",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeVerificationRule {
    InstalledState,
    ExactDetectedVersion { value: String },
}

impl RuntimeVerificationRule {
    pub fn validate(&self) -> Result<(), CatalogueError> {
        if let Self::ExactDetectedVersion { value } = self {
            require_nonempty("runtime verification version", value)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeExecutionSpec {
    pub installer: RuntimeInstallerKind,
    pub unattended: bool,
    #[serde(default)]
    pub install_args: Vec<String>,
    #[serde(default)]
    pub repair_args: Option<Vec<String>>,
    pub success_exit_codes: Vec<i32>,
    #[serde(default)]
    pub reboot_exit_codes: Vec<i32>,
    pub verification: RuntimeVerificationRule,
}

impl RuntimeExecutionSpec {
    pub fn validate(&self) -> Result<(), CatalogueError> {
        if !self.unattended {
            return Err(CatalogueError::RuntimeExecutionNotUnattended);
        }
        validate_runtime_args(self.installer, "install argument", &self.install_args)?;
        if let Some(repair_args) = &self.repair_args {
            validate_runtime_args(self.installer, "repair argument", repair_args)?;
        }
        if self.success_exit_codes.is_empty() {
            return Err(CatalogueError::RuntimeExecutionWithoutSuccessCode);
        }
        ensure_unique_exit_codes("runtime success exit code", &self.success_exit_codes)?;
        ensure_unique_exit_codes("runtime reboot exit code", &self.reboot_exit_codes)?;
        let successes = self
            .success_exit_codes
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if let Some(code) = self
            .reboot_exit_codes
            .iter()
            .find(|code| !successes.contains(code))
        {
            return Err(CatalogueError::RuntimeRebootCodeNotSuccessful(*code));
        }
        self.verification.validate()
    }
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

/// One entry from an INF Models section.
///
/// The optional `hardware_id` is the INF hw-id slot; following compatible IDs
/// remain ordered. Neo treats every value as opaque and never parses bus fields
/// such as VID/PID/SUBSYS to manufacture compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InfModelEntry {
    #[serde(default)]
    pub hardware_id: Option<OpaqueDeviceId>,
    #[serde(default)]
    pub compatible_ids: Vec<OpaqueDeviceId>,
}

impl InfModelEntry {
    pub fn validate(&self) -> Result<(), CatalogueError> {
        if self.hardware_id.is_none() && self.compatible_ids.is_empty() {
            return Err(CatalogueError::EmptyInfModelEntry);
        }
        let mut seen = BTreeSet::new();
        for value in &self.compatible_ids {
            if !seen.insert(value.as_str().to_ascii_lowercase()) {
                return Err(CatalogueError::DuplicateModelCompatibleId(
                    value.to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverArtifact {
    pub inf_path: String,
    #[serde(default)]
    pub models: Vec<InfModelEntry>,
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
    pub runtime_execution: Option<RuntimeExecutionSpec>,
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

        if let Some(runtime_execution) = &self.runtime_execution {
            if self.kind != PackageKind::Runtime {
                return Err(CatalogueError::RuntimeExecutionOnNonRuntime(
                    self.package_id.clone(),
                ));
            }
            runtime_execution.validate()?;
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
            require_nonempty("package_id", &package.package_id)?;
            if !ids.insert(package.package_id.as_str()) {
                return Err(CatalogueError::DuplicatePackageId(
                    package.package_id.clone(),
                ));
            }
        }

        for package in &self.packages {
            package.validate()?;
            for dependency in &package.dependencies {
                if !ids.contains(dependency.as_str()) {
                    return Err(CatalogueError::UnresolvedDependency {
                        package_id: package.package_id.clone(),
                        dependency: dependency.clone(),
                    });
                }
            }
            for conflict in &package.conflicts {
                if !ids.contains(conflict.as_str()) {
                    return Err(CatalogueError::UnresolvedConflict {
                        package_id: package.package_id.clone(),
                        conflict: conflict.clone(),
                    });
                }
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
    if artifact.models.is_empty() {
        return Err(CatalogueError::DriverArtifactWithoutModels(
            artifact.inf_path.clone(),
        ));
    }
    for model in &artifact.models {
        model.validate()?;
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

fn validate_runtime_args(
    installer: RuntimeInstallerKind,
    label: &'static str,
    values: &[String],
) -> Result<(), CatalogueError> {
    for value in values {
        require_nonempty(label, value)?;
        if value
            .chars()
            .any(|ch| ch == '\0' || ch == '\r' || ch == '\n')
        {
            return Err(CatalogueError::InvalidRuntimeArgument(value.clone()));
        }
        if installer == RuntimeInstallerKind::Msi && !is_msi_property_assignment(value) {
            return Err(CatalogueError::InvalidMsiRuntimeArgument(value.clone()));
        }
    }
    Ok(())
}

fn is_msi_property_assignment(value: &str) -> bool {
    let Some((name, property_value)) = value.split_once('=') else {
        return false;
    };
    if property_value.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn ensure_unique_exit_codes(label: &'static str, values: &[i32]) -> Result<(), CatalogueError> {
    let mut seen = BTreeSet::new();
    for value in values {
        // Windows process exit statuses are raw 32-bit values surfaced by
        // std::process as i32. High-bit HRESULT/Win32 values therefore appear
        // negative and must retain their bit pattern rather than be rejected.
        if !seen.insert(*value) {
            return Err(CatalogueError::DuplicateRuntimeExitCode {
                label,
                code: *value,
            });
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
    #[error("runtime execution metadata is only valid on runtime packages: {0}")]
    RuntimeExecutionOnNonRuntime(String),
    #[error("runtime execution contract must be explicitly unattended")]
    RuntimeExecutionNotUnattended,
    #[error("runtime execution contract requires at least one successful exit code")]
    RuntimeExecutionWithoutSuccessCode,
    #[error("invalid runtime process argument: {0}")]
    InvalidRuntimeArgument(String),
    #[error("MSI runtime custom arguments must be PROPERTY=VALUE assignments: {0}")]
    InvalidMsiRuntimeArgument(String),
    #[error("invalid runtime exit code: {0}")]
    InvalidRuntimeExitCode(i32),
    #[error("duplicate {label}: {code}")]
    DuplicateRuntimeExitCode { label: &'static str, code: i32 },
    #[error("runtime reboot exit code is not also successful: {0}")]
    RuntimeRebootCodeNotSuccessful(i32),
    #[error("driver artifact has no INF model entries: {0}")]
    DriverArtifactWithoutModels(String),
    #[error("INF model entry must contain at least one hardware or compatible ID")]
    EmptyInfModelEntry,
    #[error("duplicate compatible ID inside one INF model entry: {0}")]
    DuplicateModelCompatibleId(String),
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
    #[error("duplicate package ID: {0}")]
    DuplicatePackageId(String),
    #[error("package {package_id} depends on missing package {dependency}")]
    UnresolvedDependency {
        package_id: String,
        dependency: String,
    },
    #[error("package {package_id} conflicts with missing package {conflict}")]
    UnresolvedConflict {
        package_id: String,
        conflict: String,
    },
    #[error("catalogue JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("catalogue I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> OpaqueDeviceId {
        OpaqueDeviceId::new(value).unwrap()
    }

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
                models: vec![InfModelEntry {
                    hardware_id: Some(id(r"USB\VID_1234&PID_5678")),
                    compatible_ids: vec![id(r"USB\Class_FF")],
                }],
                catalog_files: vec!["drivers/fixture.cat".to_string()],
                provider: Some("Neo Fixture Vendor".to_string()),
                driver_version: Some("1.0.0.0".to_string()),
                driver_date: Some("2026-01-01".to_string()),
                signature: SignatureEvidence {
                    status: SignatureStatus::Verified,
                    signer: Some("Neo Fixture Signer".to_string()),
                    verification_note: Some("fixture only".to_string()),
                },
            }],
            runtime_execution: None,
            dependencies: vec![],
            conflicts: vec![],
            security: SecurityRequirements::default(),
            reboot: RebootRequirement::None,
        }
    }

    fn runtime_spec() -> RuntimeExecutionSpec {
        RuntimeExecutionSpec {
            installer: RuntimeInstallerKind::Exe,
            unattended: true,
            install_args: vec!["/quiet".to_string(), "/norestart".to_string()],
            repair_args: Some(vec!["/repair".to_string(), "/quiet".to_string()]),
            success_exit_codes: vec![0, 3010],
            reboot_exit_codes: vec![3010],
            verification: RuntimeVerificationRule::ExactDetectedVersion {
                value: "1.2.3".to_string(),
            },
        }
    }

    #[test]
    fn sample_manifest_validates() {
        assert!(sample_manifest().validate().is_ok());
    }

    #[test]
    fn runtime_execution_is_optional_and_runtime_only() {
        let mut manifest = sample_manifest();
        manifest.kind = PackageKind::Runtime;
        manifest.driver_artifacts.clear();
        manifest.runtime_execution = Some(runtime_spec());
        assert!(manifest.validate().is_ok());

        manifest.kind = PackageKind::Application;
        assert!(matches!(
            manifest.validate(),
            Err(CatalogueError::RuntimeExecutionOnNonRuntime(_))
        ));
    }

    #[test]
    fn runtime_reboot_code_must_be_successful() {
        let mut spec = runtime_spec();
        spec.reboot_exit_codes = vec![1641];
        assert!(matches!(
            spec.validate(),
            Err(CatalogueError::RuntimeRebootCodeNotSuccessful(1641))
        ));
    }

    #[test]
    fn msi_arguments_cannot_replace_neos_fixed_operation_switches() {
        let mut spec = runtime_spec();
        spec.installer = RuntimeInstallerKind::Msi;
        spec.install_args = vec!["/x".to_string()];
        assert!(matches!(
            spec.validate(),
            Err(CatalogueError::InvalidMsiRuntimeArgument(_))
        ));
        spec.install_args = vec!["ADDLOCAL=ALL".to_string()];
        spec.repair_args = Some(vec!["REINSTALL=ALL".to_string()]);
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn msi_bare_empty_property_is_rejected() {
        let mut spec = runtime_spec();
        spec.installer = RuntimeInstallerKind::Msi;
        spec.install_args = vec!["addlocal=ALL".to_string()];
        spec.repair_args = Some(vec!["REINSTALL=ALL".to_string()]);
        assert!(spec.validate().is_ok());

        spec.install_args = vec!["ADDLOCAL=".to_string()];
        assert!(matches!(
            spec.validate(),
            Err(CatalogueError::InvalidMsiRuntimeArgument(_))
        ));

        spec.install_args = vec!["ADDLOCAL=\"\"".to_string()];
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn windows_high_bit_exit_code_bit_pattern_is_accepted() {
        let mut spec = runtime_spec();
        let high_bit_code = 0x8007_0666u32 as i32;
        spec.success_exit_codes = vec![0, high_bit_code];
        spec.reboot_exit_codes.clear();
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn driver_artifact_requires_models() {
        let mut manifest = sample_manifest();
        manifest.driver_artifacts[0].models.clear();
        assert!(matches!(
            manifest.validate(),
            Err(CatalogueError::DriverArtifactWithoutModels(_))
        ));
    }

    #[test]
    fn model_entry_requires_at_least_one_identifier() {
        let mut manifest = sample_manifest();
        manifest.driver_artifacts[0].models[0].hardware_id = None;
        manifest.driver_artifacts[0].models[0]
            .compatible_ids
            .clear();
        assert!(matches!(
            manifest.validate(),
            Err(CatalogueError::EmptyInfModelEntry)
        ));
    }

    #[test]
    fn model_compatible_ids_are_ordered_and_unique() {
        let mut manifest = sample_manifest();
        let duplicate = manifest.driver_artifacts[0].models[0].compatible_ids[0].clone();
        manifest.driver_artifacts[0].models[0]
            .compatible_ids
            .push(duplicate);
        assert!(matches!(
            manifest.validate(),
            Err(CatalogueError::DuplicateModelCompatibleId(_))
        ));
    }

    #[test]
    fn model_compatible_ids_reject_case_only_duplicates() {
        let mut manifest = sample_manifest();
        manifest.driver_artifacts[0].models[0]
            .compatible_ids
            .push(id(r"usb\class_ff"));
        assert!(matches!(
            manifest.validate(),
            Err(CatalogueError::DuplicateModelCompatibleId(_))
        ));
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
    fn unresolved_dependency_fails_closed() {
        let mut manifest = sample_manifest();
        manifest
            .dependencies
            .push("neo.missing.dependency".to_string());
        let catalogue = Catalogue {
            packages: vec![manifest],
        };
        assert!(matches!(
            catalogue.validate(),
            Err(CatalogueError::UnresolvedDependency { .. })
        ));
    }

    #[test]
    fn unresolved_conflict_fails_closed() {
        let mut manifest = sample_manifest();
        manifest.conflicts.push("neo.missing.conflict".to_string());
        let catalogue = Catalogue {
            packages: vec![manifest],
        };
        assert!(matches!(
            catalogue.validate(),
            Err(CatalogueError::UnresolvedConflict { .. })
        ));
    }

    #[test]
    fn technician_component_is_not_forced_into_inf_semantics() {
        let mut manifest = sample_manifest();
        manifest.kind = PackageKind::TechnicianComponent;
        manifest.driver_artifacts.clear();
        assert!(manifest.validate().is_ok());
    }
}

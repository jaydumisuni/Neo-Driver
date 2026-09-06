use neo_device::{DeviceRecord, DriverBinding, OpaqueDeviceId, OrderedDeviceIds};
use neo_driverstore::StoredDriverPackage;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::DriverRepairError;

pub(crate) const CM_PROB_DISABLED_CODE: u32 = 22;

pub(crate) fn is_phase5_oem_published_inf(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if value.contains(['\\', '/']) || !lower.starts_with("oem") || !lower.ends_with(".inf") {
        return false;
    }
    let digits = &lower[3..lower.len() - 4];
    !digits.is_empty() && digits.chars().all(|character| character.is_ascii_digit())
}

fn is_driver_store_inf_path(path: &Path) -> bool {
    let value = path.to_string_lossy();
    if value.is_empty() || value.trim() != value {
        return false;
    }

    // Imported Phase 22 evidence must preserve the fully-qualified Windows path shape
    // returned by SetupGetInfDriverStoreLocationW. Parse Windows separators explicitly so
    // the same evidence validates deterministically on non-Windows CI hosts.
    let normalized = value.replace('/', "\\");
    let bytes = normalized.as_bytes();
    if bytes.len() < 4 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' || bytes[2] != b'\\' {
        return false;
    }

    let components: Vec<&str> = normalized[3..].split('\\').collect();
    if components.iter().any(|component| {
        component.is_empty() || matches!(*component, "." | "..") || component.contains(':')
    }) {
        return false;
    }

    let Some(repository_index) = components.windows(3).position(|window| {
        window[0].eq_ignore_ascii_case("System32")
            && window[1].eq_ignore_ascii_case("DriverStore")
            && window[2].eq_ignore_ascii_case("FileRepository")
    }) else {
        return false;
    };

    // A Windows root component must precede System32, and FileRepository contains one
    // package directory whose direct child is the original INF returned by SetupAPI.
    if repository_index != 1 || components.len() != repository_index + 5 {
        return false;
    }

    let package_directory = components[repository_index + 3];
    let inf_name = components[repository_index + 4];
    !package_directory.is_empty()
        && inf_name.len() > 4
        && inf_name.to_ascii_lowercase().ends_with(".inf")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PnpStatusEvidence {
    NoProblem,
    Problem { code: u32 },
}

impl PnpStatusEvidence {
    #[cfg(any(windows, test))]
    pub(crate) fn from_device(device: &DeviceRecord) -> Result<Self, DriverRepairError> {
        match device.problem_code {
            None => Ok(Self::NoProblem),
            Some(0) => Err(DriverRepairError::InvalidEvidence(format!(
                "device {} contains non-canonical PnP problem code 0; Phase 5 encodes a successful no-problem observation as None",
                device.instance_id
            ))),
            Some(code) => Ok(Self::Problem { code }),
        }
    }

    fn validate_against(&self, device: &DeviceRecord) -> Result<(), DriverRepairError> {
        match (*self, device.problem_code) {
            (Self::NoProblem, None) => {}
            (Self::Problem { code }, Some(device_code)) if code != 0 && code == device_code => {}
            (Self::Problem { code: 0 }, _) => {
                return Err(DriverRepairError::InvalidEvidence(format!(
                    "device {} contains non-canonical PnP status problem code 0",
                    device.instance_id
                )))
            }
            _ => {
                return Err(DriverRepairError::InvalidEvidence(format!(
                    "device {} PnP status evidence does not match the inherited Phase 5 problem-code evidence",
                    device.instance_id
                )))
            }
        }

        let status_disabled = matches!(
            self,
            Self::Problem {
                code: CM_PROB_DISABLED_CODE
            }
        );
        match device.disabled {
            Some(true) if !status_disabled => Err(DriverRepairError::InvalidEvidence(format!(
                "device {} reports disabled=true without CM_PROB_DISABLED (Code 22)",
                device.instance_id
            ))),
            Some(false) if status_disabled => Err(DriverRepairError::InvalidEvidence(format!(
                "device {} reports disabled=false while PnP reports CM_PROB_DISABLED (Code 22)",
                device.instance_id
            ))),
            _ => Ok(()),
        }
    }

    pub(crate) fn problem_code(self) -> Option<u32> {
        match self {
            Self::NoProblem => None,
            Self::Problem { code } => Some(code),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverRepairDeviceEvidence {
    pub device: DeviceRecord,
    pub pnp_status: PnpStatusEvidence,
    #[serde(default)]
    pub current_package: Option<StoredDriverPackage>,
}

impl DriverRepairDeviceEvidence {
    pub fn validate(&self) -> Result<(), DriverRepairError> {
        self.device
            .validate()
            .map_err(|error| DriverRepairError::InvalidEvidence(error.to_string()))?;
        self.pnp_status.validate_against(&self.device)?;

        let published = self
            .device
            .active_driver
            .as_ref()
            .and_then(|binding| binding.published_name.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty());

        if let Some(package) = &self.current_package {
            package
                .validate()
                .map_err(|error| DriverRepairError::InvalidEvidence(error.to_string()))?;
            let Some(published) = published else {
                return Err(DriverRepairError::PackageWithoutBinding(
                    self.device.instance_id.to_string(),
                ));
            };
            if !is_phase5_oem_published_inf(published)
                || !is_phase5_oem_published_inf(&package.published_inf)
            {
                return Err(DriverRepairError::InvalidEvidence(format!(
                    "device {} current package is not a Phase 5 OEM published INF identity",
                    self.device.instance_id
                )));
            }
            if !package.published_inf.eq_ignore_ascii_case(published) {
                return Err(DriverRepairError::PackageMismatch(
                    self.device.instance_id.to_string(),
                ));
            }
            if !is_driver_store_inf_path(&package.driver_store_inf) {
                return Err(DriverRepairError::InvalidEvidence(format!(
                    "device {} current package does not contain a fully qualified Driver Store FileRepository INF path",
                    self.device.instance_id
                )));
            }
        }
        Ok(())
    }

    pub fn active_published_inf(&self) -> Option<&str> {
        self.device
            .active_driver
            .as_ref()
            .and_then(|binding| binding.published_name.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportedDriverRepairEvidence {
    devices: Vec<ImportedDriverRepairDeviceEvidence>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportedDriverRepairDeviceEvidence {
    device: ImportedDeviceRecord,
    pnp_status: ImportedPnpStatusEvidence,
    #[serde(default)]
    current_package: Option<ImportedStoredDriverPackage>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportedDeviceRecord {
    instance_id: OpaqueDeviceId,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    manufacturer: Option<String>,
    #[serde(default)]
    class_name: Option<String>,
    #[serde(default)]
    class_guid: Option<String>,
    #[serde(default)]
    problem_code: Option<u32>,
    #[serde(default)]
    disabled: Option<bool>,
    #[serde(default)]
    ids: ImportedOrderedDeviceIds,
    #[serde(default)]
    active_driver: Option<ImportedDriverBinding>,
    #[serde(default)]
    upper_filters: Vec<String>,
    #[serde(default)]
    lower_filters: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportedOrderedDeviceIds {
    #[serde(default)]
    hardware_ids: Vec<OpaqueDeviceId>,
    #[serde(default)]
    compatible_ids: Vec<OpaqueDeviceId>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportedDriverBinding {
    #[serde(default)]
    published_name: Option<String>,
    #[serde(default)]
    original_name: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    class_name: Option<String>,
    #[serde(default)]
    class_guid: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    signer: Option<String>,
    #[serde(default)]
    catalog_file: Option<String>,
    #[serde(default)]
    service: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ImportedPnpState {
    NoProblem,
    Problem,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportedPnpStatusEnvelope {
    state: ImportedPnpState,
    #[serde(default)]
    code: Option<u32>,
}

#[derive(Debug)]
enum ImportedPnpStatusEvidence {
    NoProblem,
    Problem { code: u32 },
}

impl<'de> Deserialize<'de> for ImportedPnpStatusEvidence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = ImportedPnpStatusEnvelope::deserialize(deserializer)?;
        match (value.state, value.code) {
            (ImportedPnpState::NoProblem, None) => Ok(Self::NoProblem),
            (ImportedPnpState::Problem, Some(code)) => Ok(Self::Problem { code }),
            (ImportedPnpState::NoProblem, Some(_)) => {
                Err(D::Error::custom("no_problem PnP status must not contain code"))
            }
            (ImportedPnpState::Problem, None) => {
                Err(D::Error::custom("problem PnP status requires code"))
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportedStoredDriverPackage {
    published_inf: String,
    driver_store_inf: PathBuf,
}

impl From<ImportedDriverRepairEvidence> for DriverRepairEvidence {
    fn from(value: ImportedDriverRepairEvidence) -> Self {
        Self {
            devices: value.devices.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ImportedDriverRepairDeviceEvidence> for DriverRepairDeviceEvidence {
    fn from(value: ImportedDriverRepairDeviceEvidence) -> Self {
        Self {
            device: value.device.into(),
            pnp_status: value.pnp_status.into(),
            current_package: value.current_package.map(Into::into),
        }
    }
}

impl From<ImportedDeviceRecord> for DeviceRecord {
    fn from(value: ImportedDeviceRecord) -> Self {
        Self {
            instance_id: value.instance_id,
            description: value.description,
            manufacturer: value.manufacturer,
            class_name: value.class_name,
            class_guid: value.class_guid,
            problem_code: value.problem_code,
            disabled: value.disabled,
            ids: value.ids.into(),
            active_driver: value.active_driver.map(Into::into),
            upper_filters: value.upper_filters,
            lower_filters: value.lower_filters,
        }
    }
}

impl From<ImportedOrderedDeviceIds> for OrderedDeviceIds {
    fn from(value: ImportedOrderedDeviceIds) -> Self {
        Self {
            hardware_ids: value.hardware_ids,
            compatible_ids: value.compatible_ids,
        }
    }
}

impl From<ImportedDriverBinding> for DriverBinding {
    fn from(value: ImportedDriverBinding) -> Self {
        Self {
            published_name: value.published_name,
            original_name: value.original_name,
            provider: value.provider,
            class_name: value.class_name,
            class_guid: value.class_guid,
            version: value.version,
            date: value.date,
            signer: value.signer,
            catalog_file: value.catalog_file,
            service: value.service,
        }
    }
}

impl From<ImportedPnpStatusEvidence> for PnpStatusEvidence {
    fn from(value: ImportedPnpStatusEvidence) -> Self {
        match value {
            ImportedPnpStatusEvidence::NoProblem => Self::NoProblem,
            ImportedPnpStatusEvidence::Problem { code } => Self::Problem { code },
        }
    }
}

impl From<ImportedStoredDriverPackage> for StoredDriverPackage {
    fn from(value: ImportedStoredDriverPackage) -> Self {
        Self {
            published_inf: value.published_inf,
            driver_store_inf: value.driver_store_inf,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct DriverRepairEvidence {
    pub devices: Vec<DriverRepairDeviceEvidence>,
}

impl<'de> Deserialize<'de> for DriverRepairEvidence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ImportedDriverRepairEvidence::deserialize(deserializer).map(Into::into)
    }
}

impl DriverRepairEvidence {
    pub fn validate(&self) -> Result<(), DriverRepairError> {
        let mut seen = BTreeSet::new();
        for item in &self.devices {
            item.validate()?;
            let identity = item.device.instance_id.as_str().to_ascii_lowercase();
            if !seen.insert(identity) {
                return Err(DriverRepairError::DuplicateDevice(
                    item.device.instance_id.to_string(),
                ));
            }
        }
        Ok(())
    }

    pub fn from_json_str(input: &str) -> Result<Self, DriverRepairError> {
        let value: Self = serde_json::from_str(input)
            .map_err(|error| DriverRepairError::Serialization(error.to_string()))?;
        value.validate()?;
        Ok(value)
    }

    pub fn digest(&self) -> Result<String, DriverRepairError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|error| DriverRepairError::Serialization(error.to_string()))?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriverRepairState {
    Healthy,
    Disabled,
    MissingDriverBinding,
    PnpProblem,
    EvidenceUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriverRepairRoute {
    NoAction,
    CurrentExactDriverReinstallCandidate,
    DriverSelectionRequired,
    ManualInvestigation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverRepairAssessment {
    pub instance_id: String,
    pub description: Option<String>,
    pub pnp_status: PnpStatusEvidence,
    pub problem_code: Option<u32>,
    pub disabled: Option<bool>,
    pub active_published_inf: Option<String>,
    pub exact_driver_store_package: Option<StoredDriverPackage>,
    pub upper_filters: Vec<String>,
    pub lower_filters: Vec<String>,
    pub state: DriverRepairState,
    pub route: DriverRepairRoute,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverRepairAssessmentReport {
    pub source_evidence_sha256: String,
    pub assessments: Vec<DriverRepairAssessment>,
    pub machine_changes: bool,
}

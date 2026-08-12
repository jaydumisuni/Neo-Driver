use neo_device::{DeviceRecord, DriverBinding};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::DriverStoreError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedInfSignature {
    pub catalog_file: String,
    pub signer: String,
    #[serde(default)]
    pub signer_version: Option<String>,
}

impl VerifiedInfSignature {
    pub fn validate(&self) -> Result<(), DriverStoreError> {
        if self.catalog_file.trim().is_empty() || self.signer.trim().is_empty() {
            return Err(DriverStoreError::InvalidSignatureEvidence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredDriverPackage {
    pub published_inf: String,
    pub driver_store_inf: PathBuf,
}

impl StoredDriverPackage {
    pub fn validate(&self) -> Result<(), DriverStoreError> {
        if self.published_inf.trim().is_empty()
            || !self.published_inf.to_ascii_lowercase().ends_with(".inf")
            || self.driver_store_inf.as_os_str().is_empty()
        {
            return Err(DriverStoreError::InvalidStoredPackage);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DriverStoreBaseline {
    Existing { package: StoredDriverPackage },
    Absent,
}

impl DriverStoreBaseline {
    pub fn validate(&self) -> Result<(), DriverStoreError> {
        if let Self::Existing { package } = self {
            package.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverBindingBaseline {
    pub binding: DriverBinding,
    pub problem_code: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverInstallImpact {
    pub instance_id: String,
    pub baseline: DriverBindingBaseline,
    pub baseline_package: StoredDriverPackage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DriverInstallPlanWire")]
pub struct DriverInstallPlan {
    pub action_id: String,
    pub mission_id: String,
    pub package_id: String,
    pub inf_path: String,
    pub package_root: PathBuf,
    pub source_inf: PathBuf,
    pub source_inf_sha256: String,
    pub architecture: String,
    pub windows_build: u32,
    pub expected_signature: VerifiedInfSignature,
    pub store_baseline: DriverStoreBaseline,
    pub impacts: Vec<DriverInstallImpact>,
}

#[derive(Debug, Deserialize)]
struct DriverInstallPlanWire {
    action_id: String,
    mission_id: String,
    package_id: String,
    inf_path: String,
    package_root: PathBuf,
    source_inf: PathBuf,
    source_inf_sha256: String,
    architecture: String,
    windows_build: u32,
    expected_signature: VerifiedInfSignature,
    store_baseline: DriverStoreBaseline,
    impacts: Vec<DriverInstallImpact>,
}

impl TryFrom<DriverInstallPlanWire> for DriverInstallPlan {
    type Error = DriverStoreError;

    fn try_from(value: DriverInstallPlanWire) -> Result<Self, Self::Error> {
        let plan = Self {
            action_id: value.action_id,
            mission_id: value.mission_id,
            package_id: value.package_id,
            inf_path: value.inf_path,
            package_root: value.package_root,
            source_inf: value.source_inf,
            source_inf_sha256: value.source_inf_sha256,
            architecture: value.architecture,
            windows_build: value.windows_build,
            expected_signature: value.expected_signature,
            store_baseline: value.store_baseline,
            impacts: value.impacts,
        };
        plan.validate()?;
        Ok(plan)
    }
}

impl DriverInstallPlan {
    pub fn validate(&self) -> Result<(), DriverStoreError> {
        for (label, value) in [
            ("action_id", self.action_id.as_str()),
            ("mission_id", self.mission_id.as_str()),
            ("package_id", self.package_id.as_str()),
            ("inf_path", self.inf_path.as_str()),
            ("architecture", self.architecture.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(DriverStoreError::EmptyField(label));
            }
        }
        if !is_sha256(&self.source_inf_sha256) {
            return Err(DriverStoreError::InvalidSourceHash(
                self.source_inf_sha256.clone(),
            ));
        }
        if self.package_root.as_os_str().is_empty() || self.source_inf.as_os_str().is_empty() {
            return Err(DriverStoreError::UnsafeInfPath);
        }
        if !self.source_inf.starts_with(&self.package_root) {
            return Err(DriverStoreError::UnsafeInfPath);
        }
        self.expected_signature.validate()?;
        self.store_baseline.validate()?;
        if self.impacts.is_empty() {
            return Err(DriverStoreError::NoSupportedPresentDevice);
        }
        let mut instances = BTreeSet::new();
        for impact in &self.impacts {
            if impact.instance_id.trim().is_empty() {
                return Err(DriverStoreError::EmptyField("instance_id"));
            }
            let identity = impact.instance_id.to_ascii_lowercase();
            if !instances.insert(identity) {
                return Err(DriverStoreError::DuplicateImpact(
                    impact.instance_id.clone(),
                ));
            }
            let published = impact
                .baseline
                .binding
                .published_name
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    DriverStoreError::MissingBaselinePublishedInf(impact.instance_id.clone())
                })?;
            if !published.to_ascii_lowercase().ends_with(".inf") {
                return Err(DriverStoreError::MissingBaselinePublishedInf(
                    impact.instance_id.clone(),
                ));
            }
            impact.baseline_package.validate()?;
            if !impact
                .baseline_package
                .published_inf
                .eq_ignore_ascii_case(published)
            {
                return Err(DriverStoreError::BaselinePackageMismatch(
                    impact.instance_id.clone(),
                ));
            }
        }
        Ok(())
    }

    pub fn fingerprint(&self) -> Result<String, DriverStoreError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)?;
        Ok(sha256_bytes(&bytes))
    }

    pub fn from_json_str(input: &str) -> Result<Self, DriverStoreError> {
        let wire: DriverInstallPlanWire = serde_json::from_str(input)?;
        Self::try_from(wire)
    }

    pub fn impact_ids(&self) -> BTreeSet<String> {
        self.impacts
            .iter()
            .map(|impact| impact.instance_id.to_ascii_lowercase())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedDriverInstall {
    pub driver_plan: DriverInstallPlan,
    pub transaction_plan: neo_transaction::TransactionPlan,
    pub baseline: neo_transaction::BaselineSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverBackendResult {
    pub reboot_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverInventory {
    pub devices: Vec<DeviceRecord>,
}

impl DriverInventory {
    pub fn validate(&self) -> Result<(), DriverStoreError> {
        let mut instances = BTreeSet::new();
        for device in &self.devices {
            device
                .validate()
                .map_err(|error| DriverStoreError::Device(error.to_string()))?;
            let identity = device.instance_id.as_str().to_ascii_lowercase();
            if !instances.insert(identity) {
                return Err(DriverStoreError::DuplicateInventoryDevice(
                    device.instance_id.to_string(),
                ));
            }
        }
        Ok(())
    }

    pub fn device(&self, instance_id: &str) -> Option<&DeviceRecord> {
        self.devices.iter().find(|device| {
            device
                .instance_id
                .as_str()
                .eq_ignore_ascii_case(instance_id)
        })
    }
}

pub(crate) fn sha256_file(path: &Path) -> Result<String, DriverStoreError> {
    let bytes = std::fs::read(path)?;
    Ok(sha256_bytes(&bytes))
}

pub(crate) fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

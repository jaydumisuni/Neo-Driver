use neo_device::{DeviceRecord, DriverBinding};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::path::PathBuf;

use crate::DriverStoreError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedInfSignature {
    pub catalog_file: String,
    pub signer: String,
    #[serde(default)]
    pub signer_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredDriverPackage {
    pub published_inf: String,
    pub driver_store_inf: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DriverStoreBaseline {
    Existing { package: StoredDriverPackage },
    Absent,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverInstallPlan {
    pub action_id: String,
    pub package_id: String,
    pub inf_path: String,
    pub package_root: PathBuf,
    pub source_inf: PathBuf,
    pub architecture: String,
    pub windows_build: u32,
    pub expected_signature: VerifiedInfSignature,
    pub store_baseline: DriverStoreBaseline,
    pub impacts: Vec<DriverInstallImpact>,
    #[serde(default)]
    pub preexisting_target_bindings: Vec<String>,
}

impl DriverInstallPlan {
    pub fn fingerprint(&self) -> Result<String, DriverStoreError> {
        let bytes = serde_json::to_vec(self)?;
        let digest = Sha256::digest(bytes);
        let mut encoded = String::with_capacity(digest.len() * 2);
        for byte in digest {
            write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
        }
        Ok(encoded)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverInventory {
    pub devices: Vec<DeviceRecord>,
}

impl DriverInventory {
    pub fn device(&self, instance_id: &str) -> Option<&DeviceRecord> {
        self.devices
            .iter()
            .find(|device| device.instance_id.as_str().eq_ignore_ascii_case(instance_id))
    }
}

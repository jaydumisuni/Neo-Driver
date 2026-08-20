use neo_device::DeviceRecord;
use neo_driverstore::StoredDriverPackage;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use crate::DriverRepairError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverRepairDeviceEvidence {
    pub device: DeviceRecord,
    #[serde(default)]
    pub current_package: Option<StoredDriverPackage>,
}

impl DriverRepairDeviceEvidence {
    pub fn validate(&self) -> Result<(), DriverRepairError> {
        self.device
            .validate()
            .map_err(|error| DriverRepairError::InvalidEvidence(error.to_string()))?;

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
            if !package.published_inf.eq_ignore_ascii_case(published) {
                return Err(DriverRepairError::PackageMismatch(
                    self.device.instance_id.to_string(),
                ));
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DriverRepairEvidence {
    pub devices: Vec<DriverRepairDeviceEvidence>,
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

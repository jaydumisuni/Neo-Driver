use neo_device::DeviceRecord;
use neo_driverstore::StoredDriverPackage;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use crate::DriverRepairError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PnpStatusEvidence {
    NoProblem,
    Problem { code: u32 },
}

impl PnpStatusEvidence {
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
            (Self::NoProblem, None) => Ok(()),
            (Self::Problem { code }, Some(device_code)) if code != 0 && code == device_code => {
                Ok(())
            }
            (Self::Problem { code: 0 }, _) => Err(DriverRepairError::InvalidEvidence(format!(
                "device {} contains non-canonical PnP status problem code 0",
                device.instance_id
            ))),
            _ => Err(DriverRepairError::InvalidEvidence(format!(
                "device {} PnP status evidence does not match the inherited Phase 5 problem-code evidence",
                device.instance_id
            ))),
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

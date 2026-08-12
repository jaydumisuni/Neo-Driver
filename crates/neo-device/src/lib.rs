//! Normalized, read-only device evidence contracts for Neo Driver.
//!
//! Windows hardware and compatible IDs are preserved as ordered opaque strings.
//! Neo must not infer authoritative compatibility by splitting or reinterpreting
//! those identifiers; matching logic belongs to a later phase.

use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct OpaqueDeviceId(String);

impl OpaqueDeviceId {
    pub fn new(value: impl Into<String>) -> Result<Self, DeviceValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DeviceValidationError::EmptyDeviceId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for OpaqueDeviceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

impl fmt::Display for OpaqueDeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OrderedDeviceIds {
    pub hardware_ids: Vec<OpaqueDeviceId>,
    pub compatible_ids: Vec<OpaqueDeviceId>,
}

impl OrderedDeviceIds {
    pub fn validate(&self) -> Result<(), DeviceValidationError> {
        ensure_unique("hardware ID", &self.hardware_ids)?;
        ensure_unique("compatible ID", &self.compatible_ids)?;
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.hardware_ids.is_empty() && self.compatible_ids.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DriverBinding {
    pub published_name: Option<String>,
    pub original_name: Option<String>,
    pub provider: Option<String>,
    pub class_name: Option<String>,
    pub class_guid: Option<String>,
    pub version: Option<String>,
    pub date: Option<String>,
    pub signer: Option<String>,
    pub catalog_file: Option<String>,
    pub service: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DeviceRecordWire")]
pub struct DeviceRecord {
    pub instance_id: OpaqueDeviceId,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub manufacturer: Option<String>,
    #[serde(default)]
    pub class_name: Option<String>,
    #[serde(default)]
    pub class_guid: Option<String>,
    #[serde(default)]
    pub problem_code: Option<u32>,
    #[serde(default)]
    pub disabled: Option<bool>,
    #[serde(default)]
    pub ids: OrderedDeviceIds,
    #[serde(default)]
    pub active_driver: Option<DriverBinding>,
    #[serde(default)]
    pub upper_filters: Vec<String>,
    #[serde(default)]
    pub lower_filters: Vec<String>,
}

impl DeviceRecord {
    pub fn validate(&self) -> Result<(), DeviceValidationError> {
        self.ids.validate()?;
        ensure_unique_strings("upper filter", &self.upper_filters)?;
        ensure_unique_strings("lower filter", &self.lower_filters)?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct DeviceRecordWire {
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
    ids: OrderedDeviceIds,
    #[serde(default)]
    active_driver: Option<DriverBinding>,
    #[serde(default)]
    upper_filters: Vec<String>,
    #[serde(default)]
    lower_filters: Vec<String>,
}

impl TryFrom<DeviceRecordWire> for DeviceRecord {
    type Error = DeviceValidationError;

    fn try_from(value: DeviceRecordWire) -> Result<Self, Self::Error> {
        let record = Self {
            instance_id: value.instance_id,
            description: value.description,
            manufacturer: value.manufacturer,
            class_name: value.class_name,
            class_guid: value.class_guid,
            problem_code: value.problem_code,
            disabled: value.disabled,
            ids: value.ids,
            active_driver: value.active_driver,
            upper_filters: value.upper_filters,
            lower_filters: value.lower_filters,
        };
        record.validate()?;
        Ok(record)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(try_from = "DeviceInventoryWire")]
pub struct DeviceInventory {
    pub devices: Vec<DeviceRecord>,
}

impl DeviceInventory {
    pub fn validate(&self) -> Result<(), DeviceValidationError> {
        let mut instances = BTreeSet::new();
        for device in &self.devices {
            device.validate()?;
            if !instances.insert(device.instance_id.clone()) {
                return Err(DeviceValidationError::DuplicateInstanceId(
                    device.instance_id.to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct DeviceInventoryWire {
    #[serde(default)]
    devices: Vec<DeviceRecord>,
}

impl TryFrom<DeviceInventoryWire> for DeviceInventory {
    type Error = DeviceValidationError;

    fn try_from(value: DeviceInventoryWire) -> Result<Self, Self::Error> {
        let inventory = Self {
            devices: value.devices,
        };
        inventory.validate()?;
        Ok(inventory)
    }
}

fn ensure_unique(
    label: &'static str,
    values: &[OpaqueDeviceId],
) -> Result<(), DeviceValidationError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value.clone()) {
            return Err(DeviceValidationError::DuplicateOpaqueValue {
                label,
                value: value.to_string(),
            });
        }
    }
    Ok(())
}

fn ensure_unique_strings(
    label: &'static str,
    values: &[String],
) -> Result<(), DeviceValidationError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() {
            return Err(DeviceValidationError::EmptyStringValue(label));
        }
        if !seen.insert(value) {
            return Err(DeviceValidationError::DuplicateStringValue {
                label,
                value: value.clone(),
            });
        }
    }
    Ok(())
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DeviceValidationError {
    #[error("device identifier cannot be empty")]
    EmptyDeviceId,
    #[error("duplicate {label}: {value}")]
    DuplicateOpaqueValue { label: &'static str, value: String },
    #[error("duplicate device instance ID: {0}")]
    DuplicateInstanceId(String),
    #[error("{0} cannot be empty")]
    EmptyStringValue(&'static str),
    #[error("duplicate {label}: {value}")]
    DuplicateStringValue { label: &'static str, value: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_id_rejects_empty_constructor_value() {
        assert_eq!(
            OpaqueDeviceId::new("   ").unwrap_err(),
            DeviceValidationError::EmptyDeviceId
        );
    }

    #[test]
    fn opaque_id_rejects_empty_deserialized_value() {
        let error = serde_json::from_str::<OpaqueDeviceId>(r#""""#).unwrap_err();
        assert!(error.to_string().contains("cannot be empty"));
    }

    #[test]
    fn ordered_ids_preserve_input_order() {
        let ids = OrderedDeviceIds {
            hardware_ids: vec![
                OpaqueDeviceId::new(r"PCI\VEN_1111&DEV_0001").unwrap(),
                OpaqueDeviceId::new(r"PCI\VEN_1111&DEV_0001&REV_02").unwrap(),
            ],
            compatible_ids: vec![],
        };
        assert!(ids.validate().is_ok());
        assert!(ids.hardware_ids[0].as_str().ends_with("DEV_0001"));
        assert!(ids.hardware_ids[1].as_str().ends_with("REV_02"));
    }

    #[test]
    fn duplicate_filters_fail_during_deserialization() {
        let input = r#"{
            "instance_id":"USB\\VID_1234&PID_5678\\ABC",
            "upper_filters":["libusb0","libusb0"]
        }"#;
        let error = serde_json::from_str::<DeviceRecord>(input).unwrap_err();
        assert!(error.to_string().contains("duplicate upper filter"));
    }

    #[test]
    fn duplicate_instances_fail_during_inventory_deserialization() {
        let input = r#"{
            "devices":[
                {"instance_id":"USB\\VID_1234&PID_5678\\ABC"},
                {"instance_id":"USB\\VID_1234&PID_5678\\ABC"}
            ]
        }"#;
        let error = serde_json::from_str::<DeviceInventory>(input).unwrap_err();
        assert!(error.to_string().contains("duplicate device instance ID"));
    }

    #[test]
    fn duplicate_instance_ids_fail_closed() {
        let instance = OpaqueDeviceId::new(r"USB\VID_1234&PID_5678\ABC").unwrap();
        let record = DeviceRecord {
            instance_id: instance.clone(),
            description: None,
            manufacturer: None,
            class_name: None,
            class_guid: None,
            problem_code: None,
            disabled: None,
            ids: OrderedDeviceIds::default(),
            active_driver: None,
            upper_filters: vec![],
            lower_filters: vec![],
        };
        let inventory = DeviceInventory {
            devices: vec![record.clone(), record],
        };
        assert!(matches!(
            inventory.validate(),
            Err(DeviceValidationError::DuplicateInstanceId(_))
        ));
    }
}

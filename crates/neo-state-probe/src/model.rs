use crate::StateProbeError;
use neo_state_plan::TweakTarget;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryHive {
    LocalMachine,
    CurrentUser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryView {
    Native,
    Registry32,
    Registry64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryValueKind {
    Text,
    U32,
    U64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WindowsStateSource {
    RegistryValue {
        hive: RegistryHive,
        subkey: String,
        value_name: String,
        value_kind: RegistryValueKind,
        view: RegistryView,
    },
    ServiceStartType { service_name: String },
    ServiceState { service_name: String },
    OptionalFeature { feature_name: String },
    AppxPackage { package_family_name: String },
}

impl WindowsStateSource {
    pub fn validate(&self) -> Result<(), StateProbeError> {
        match self {
            Self::RegistryValue { subkey, value_name, .. } => {
                require_text("registry subkey", subkey)?;
                reject_nul("registry subkey", subkey)?;
                reject_nul("registry value name", value_name)?;
            }
            Self::ServiceStartType { service_name } | Self::ServiceState { service_name } => {
                require_text("service name", service_name)?;
                reject_nul("service name", service_name)?;
            }
            Self::OptionalFeature { feature_name } => {
                require_safe_identifier("feature name", feature_name)?;
            }
            Self::AppxPackage { package_family_name } => {
                require_safe_identifier("package family name", package_family_name)?;
            }
        }
        Ok(())
    }

    pub fn evidence_source(&self) -> &'static str {
        match self {
            Self::RegistryValue { .. } => "windows.registry",
            Self::ServiceStartType { .. } => "windows.service.start_type",
            Self::ServiceState { .. } => "windows.service.state",
            Self::OptionalFeature { .. } => "windows.optional_feature",
            Self::AppxPackage { .. } => "windows.appx.current_user",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsStateBinding {
    pub target: TweakTarget,
    pub source: WindowsStateSource,
}

impl WindowsStateBinding {
    pub fn validate(&self) -> Result<(), StateProbeError> {
        self.target
            .validate()
            .map_err(|error| StateProbeError::StatePlan(error.to_string()))?;
        self.source.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "WindowsStateBindingsWire")]
pub struct WindowsStateBindings {
    pub bindings: Vec<WindowsStateBinding>,
}

#[derive(Debug, Deserialize)]
struct WindowsStateBindingsWire {
    bindings: Vec<WindowsStateBinding>,
}

impl TryFrom<WindowsStateBindingsWire> for WindowsStateBindings {
    type Error = StateProbeError;

    fn try_from(value: WindowsStateBindingsWire) -> Result<Self, Self::Error> {
        Self::new(value.bindings)
    }
}

impl WindowsStateBindings {
    pub fn new(bindings: Vec<WindowsStateBinding>) -> Result<Self, StateProbeError> {
        let value = Self { bindings };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), StateProbeError> {
        let mut targets = BTreeSet::new();
        for binding in &self.bindings {
            binding.validate()?;
            let key = binding
                .target
                .canonical_key()
                .map_err(|error| StateProbeError::StatePlan(error.to_string()))?;
            if !targets.insert(key.clone()) {
                return Err(StateProbeError::DuplicateBinding(key));
            }
        }
        Ok(())
    }

    pub fn find(&self, target: &TweakTarget) -> Result<Option<&WindowsStateBinding>, StateProbeError> {
        let wanted = target
            .canonical_key()
            .map_err(|error| StateProbeError::StatePlan(error.to_string()))?;
        for binding in &self.bindings {
            let key = binding
                .target
                .canonical_key()
                .map_err(|error| StateProbeError::StatePlan(error.to_string()))?;
            if key == wanted {
                return Ok(Some(binding));
            }
        }
        Ok(None)
    }

    pub fn from_json_str(input: &str) -> Result<Self, StateProbeError> {
        Ok(serde_json::from_str(input)?)
    }

    pub fn read_json(path: impl AsRef<Path>) -> Result<Self, StateProbeError> {
        Self::from_json_str(&std::fs::read_to_string(path)?)
    }
}

fn require_text(field: &'static str, value: &str) -> Result<(), StateProbeError> {
    if value.trim().is_empty() {
        return Err(StateProbeError::InvalidField {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn reject_nul(field: &'static str, value: &str) -> Result<(), StateProbeError> {
    if value.contains('\0') {
        return Err(StateProbeError::InvalidField {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn require_safe_identifier(field: &'static str, value: &str) -> Result<(), StateProbeError> {
    require_text(field, value)?;
    let valid = value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if !valid {
        return Err(StateProbeError::InvalidField {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

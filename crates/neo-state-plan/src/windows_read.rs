use crate::{ReaderId, StatePlanError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryHive {
    LocalMachine,
    CurrentUser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryView {
    Default,
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
pub enum WindowsReadSource {
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
    AppxCurrentUser { package_family_name: String },
}

impl WindowsReadSource {
    pub fn validate(&self) -> Result<(), StatePlanError> {
        match self {
            Self::RegistryValue { subkey, value_name, .. } => {
                require_text(subkey)?;
                reject_nul(subkey)?;
                reject_nul(value_name)?;
            }
            Self::ServiceStartType { service_name } | Self::ServiceState { service_name } => {
                require_identifier(service_name)?;
            }
            Self::OptionalFeature { feature_name } => require_identifier(feature_name)?,
            Self::AppxCurrentUser { package_family_name } => require_identifier(package_family_name)?,
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsReaderSource {
    pub reader: ReaderId,
    pub source: WindowsReadSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "WindowsReaderSourcesWire")]
pub struct WindowsReaderSources {
    pub readers: Vec<WindowsReaderSource>,
}

#[derive(Debug, Deserialize)]
struct WindowsReaderSourcesWire {
    readers: Vec<WindowsReaderSource>,
}

impl TryFrom<WindowsReaderSourcesWire> for WindowsReaderSources {
    type Error = StatePlanError;

    fn try_from(value: WindowsReaderSourcesWire) -> Result<Self, Self::Error> {
        Self::new(value.readers)
    }
}

impl WindowsReaderSources {
    pub fn new(readers: Vec<WindowsReaderSource>) -> Result<Self, StatePlanError> {
        let value = Self { readers };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), StatePlanError> {
        let mut ids = BTreeSet::new();
        for item in &self.readers {
            ReaderId::new(item.reader.as_str())?;
            item.source.validate()?;
            if !ids.insert(item.reader.clone()) {
                return Err(StatePlanError::DuplicateReaderSource(
                    item.reader.as_str().to_string(),
                ));
            }
        }
        Ok(())
    }

    pub fn find(&self, reader: &ReaderId) -> Option<&WindowsReaderSource> {
        self.readers.iter().find(|item| &item.reader == reader)
    }
}

fn require_text(value: &str) -> Result<(), StatePlanError> {
    if value.trim().is_empty() {
        return Err(StatePlanError::InvalidReaderSource("empty source field"));
    }
    Ok(())
}

fn reject_nul(value: &str) -> Result<(), StatePlanError> {
    if value.contains('\0') {
        return Err(StatePlanError::InvalidReaderSource("source contains NUL"));
    }
    Ok(())
}

fn require_identifier(value: &str) -> Result<(), StatePlanError> {
    require_text(value)?;
    if !value.is_ascii()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'$'))
    {
        return Err(StatePlanError::InvalidReaderSource("invalid source identifier"));
    }
    Ok(())
}

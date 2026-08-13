use crate::StatePlanError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum TweakValue {
    Text(String),
    U32(u32),
    U64(u64),
}

impl TweakValue {
    pub fn canonical_json(&self) -> Result<String, StatePlanError> {
        Ok(serde_json::to_string(self)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TweakTarget {
    pub key: String,
}

impl TweakTarget {
    pub fn validate(&self) -> Result<(), StatePlanError> {
        if self.key.trim().is_empty() || self.key.contains('\0') {
            return Err(StatePlanError::InvalidTarget(self.key.clone()));
        }
        Ok(())
    }

    pub fn canonical_key(&self) -> Result<String, StatePlanError> {
        self.validate()?;
        Ok(self.key.trim().to_ascii_lowercase())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TweakOperation {
    Set { value: TweakValue },
    Delete,
}

impl TweakOperation {
    pub(crate) fn desired(&self) -> Result<String, StatePlanError> {
        match self {
            Self::Set { value } => value.canonical_json(),
            Self::Delete => Ok("absent".to_string()),
        }
    }
}

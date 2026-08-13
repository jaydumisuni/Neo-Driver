use crate::StateResolverError;
use neo_state_plan::TweakTarget;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReaderId(String);

impl ReaderId {
    pub fn new(value: impl Into<String>) -> Result<Self, StateResolverError> {
        let value = value.into();
        if value.is_empty()
            || !value.is_ascii()
            || !value.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'_' | b'-')
            })
        {
            return Err(StateResolverError::InvalidField("reader id"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateBinding {
    pub target: TweakTarget,
    pub reader: ReaderId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateBindings {
    pub bindings: Vec<StateBinding>,
}

impl StateBindings {
    pub fn validate(&self) -> Result<(), StateResolverError> {
        let mut targets = BTreeSet::new();
        for binding in &self.bindings {
            binding
                .target
                .validate()
                .map_err(|error| StateResolverError::StatePlan(error.to_string()))?;
            ReaderId::new(binding.reader.as_str())?;
            let key = binding
                .target
                .canonical_key()
                .map_err(|error| StateResolverError::StatePlan(error.to_string()))?;
            if !targets.insert(key.clone()) {
                return Err(StateResolverError::DuplicateBinding(key));
            }
        }
        Ok(())
    }

    pub fn find(&self, target: &TweakTarget) -> Result<Option<&StateBinding>, StateResolverError> {
        let wanted = target
            .canonical_key()
            .map_err(|error| StateResolverError::StatePlan(error.to_string()))?;
        for binding in &self.bindings {
            let key = binding
                .target
                .canonical_key()
                .map_err(|error| StateResolverError::StatePlan(error.to_string()))?;
            if key == wanted {
                return Ok(Some(binding));
            }
        }
        Ok(None)
    }
}

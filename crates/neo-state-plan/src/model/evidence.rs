use super::{definition::require_text, TweakTarget, TweakValue};
use crate::StatePlanError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ObservedState {
    Present { value: TweakValue },
    Absent,
    Unavailable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TweakObservation {
    pub target: TweakTarget,
    pub state: ObservedState,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TweakEvidenceWire")]
pub struct TweakEvidence {
    pub observations: Vec<TweakObservation>,
}

#[derive(Debug, Deserialize)]
struct TweakEvidenceWire {
    observations: Vec<TweakObservation>,
}

impl TryFrom<TweakEvidenceWire> for TweakEvidence {
    type Error = StatePlanError;

    fn try_from(value: TweakEvidenceWire) -> Result<Self, Self::Error> {
        Self::new(value.observations)
    }
}

impl TweakEvidence {
    pub fn new(observations: Vec<TweakObservation>) -> Result<Self, StatePlanError> {
        let evidence = Self { observations };
        evidence.validate()?;
        Ok(evidence)
    }

    pub fn validate(&self) -> Result<(), StatePlanError> {
        let mut targets = BTreeSet::new();
        for observation in &self.observations {
            observation.target.validate()?;
            require_text("observation source", &observation.source)?;
            if let ObservedState::Unavailable { reason } = &observation.state {
                require_text("unavailable reason", reason)?;
            }
            let key = observation.target.canonical_key()?;
            if !targets.insert(key.clone()) {
                return Err(StatePlanError::DuplicateObservation(key));
            }
        }
        Ok(())
    }

    pub fn from_json_str(input: &str) -> Result<Self, StatePlanError> {
        Ok(serde_json::from_str(input)?)
    }

    pub fn read_json(path: impl AsRef<Path>) -> Result<Self, StatePlanError> {
        Self::from_json_str(&std::fs::read_to_string(path)?)
    }

    pub(crate) fn find(
        &self,
        target: &TweakTarget,
    ) -> Result<Option<&TweakObservation>, StatePlanError> {
        let key = target.canonical_key()?;
        for observation in &self.observations {
            if observation.target.canonical_key()? == key {
                return Ok(Some(observation));
            }
        }
        Ok(None)
    }
}

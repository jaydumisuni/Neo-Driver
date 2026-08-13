use crate::{ObservedState, StatePlanError, TweakCatalogue, TweakEvidence, TweakObservation, TweakTarget};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReaderId(String);

impl ReaderId {
    pub fn new(value: impl Into<String>) -> Result<Self, StatePlanError> {
        let value = value.into();
        if value.is_empty()
            || !value.is_ascii()
            || !value.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'_' | b'-')
            })
        {
            return Err(StatePlanError::InvalidReaderId(value));
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
#[serde(try_from = "StateBindingsWire")]
pub struct StateBindings {
    pub bindings: Vec<StateBinding>,
}

#[derive(Debug, Deserialize)]
struct StateBindingsWire {
    bindings: Vec<StateBinding>,
}

impl TryFrom<StateBindingsWire> for StateBindings {
    type Error = StatePlanError;

    fn try_from(value: StateBindingsWire) -> Result<Self, Self::Error> {
        Self::new(value.bindings)
    }
}

impl StateBindings {
    pub fn new(bindings: Vec<StateBinding>) -> Result<Self, StatePlanError> {
        let value = Self { bindings };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), StatePlanError> {
        let mut targets = BTreeSet::new();
        for binding in &self.bindings {
            binding.target.validate()?;
            ReaderId::new(binding.reader.as_str())?;
            let key = binding.target.canonical_key()?;
            if !targets.insert(key.clone()) {
                return Err(StatePlanError::DuplicateBinding(key));
            }
        }
        Ok(())
    }

    pub fn find(&self, target: &TweakTarget) -> Result<Option<&StateBinding>, StatePlanError> {
        let wanted = target.canonical_key()?;
        for binding in &self.bindings {
            if binding.target.canonical_key()? == wanted {
                return Ok(Some(binding));
            }
        }
        Ok(None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedState {
    pub reader: ReaderId,
    pub state: ObservedState,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "CapturedStatesWire")]
pub struct CapturedStates {
    pub values: Vec<CapturedState>,
}

#[derive(Debug, Deserialize)]
struct CapturedStatesWire {
    values: Vec<CapturedState>,
}

impl TryFrom<CapturedStatesWire> for CapturedStates {
    type Error = StatePlanError;

    fn try_from(value: CapturedStatesWire) -> Result<Self, Self::Error> {
        Self::new(value.values)
    }
}

impl CapturedStates {
    pub fn new(values: Vec<CapturedState>) -> Result<Self, StatePlanError> {
        let mut readers = BTreeSet::new();
        for item in &values {
            ReaderId::new(item.reader.as_str())?;
            if item.source.trim().is_empty() {
                return Err(StatePlanError::EmptyField("captured state source"));
            }
            if !readers.insert(item.reader.clone()) {
                return Err(StatePlanError::DuplicateCapturedState(
                    item.reader.as_str().to_string(),
                ));
            }
        }
        Ok(Self { values })
    }

    fn indexed(&self) -> BTreeMap<&ReaderId, &CapturedState> {
        self.values.iter().map(|item| (&item.reader, item)).collect()
    }
}

pub fn resolve_selected_evidence(
    catalogue: &TweakCatalogue,
    bindings: &StateBindings,
    captured: &CapturedStates,
    selected_ids: &[String],
) -> Result<TweakEvidence, StatePlanError> {
    catalogue.validate()?;
    bindings.validate()?;
    if selected_ids.is_empty() {
        return Err(StatePlanError::EmptySelection);
    }

    let captured_by_reader = captured.indexed();
    let mut selected = BTreeSet::new();
    let mut observations = Vec::with_capacity(selected_ids.len());
    for id in selected_ids {
        if !selected.insert(id.as_str()) {
            return Err(StatePlanError::DuplicateSelection(id.clone()));
        }
        let definition = catalogue
            .get(id)
            .ok_or_else(|| StatePlanError::UnknownTweak(id.clone()))?;
        let binding = bindings
            .find(&definition.target)?
            .ok_or_else(|| StatePlanError::MissingBinding(definition.target.canonical_key().unwrap_or_default()))?;
        let captured = captured_by_reader.get(&binding.reader).copied();
        let (state, source) = match captured {
            Some(item) => (item.state.clone(), item.source.clone()),
            None => (
                ObservedState::Unavailable {
                    reason: "state was not captured".to_string(),
                },
                format!("reader:{}", binding.reader.as_str()),
            ),
        };
        observations.push(TweakObservation {
            target: definition.target.clone(),
            state,
            source,
        });
    }
    TweakEvidence::new(observations)
}

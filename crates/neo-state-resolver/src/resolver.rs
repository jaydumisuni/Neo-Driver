use crate::{ReaderId, StateBindings, StateResolverError};
use neo_state_plan::{ObservedState, TweakCatalogue, TweakEvidence, TweakObservation};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default)]
pub struct CapturedStates {
    values: BTreeMap<ReaderId, ObservedState>,
}

impl CapturedStates {
    pub fn insert(&mut self, reader: ReaderId, state: ObservedState) -> Option<ObservedState> {
        self.values.insert(reader, state)
    }
}

pub fn resolve_selected_evidence(
    catalogue: &TweakCatalogue,
    bindings: &StateBindings,
    selected_ids: &[String],
    captured: &CapturedStates,
) -> Result<TweakEvidence, StateResolverError> {
    catalogue.validate().map_err(|error| StateResolverError::StatePlan(error.to_string()))?;
    bindings.validate()?;
    let mut seen = BTreeSet::new();
    let mut observations = Vec::new();
    for id in selected_ids {
        if !seen.insert(id.as_str()) {
            return Err(StateResolverError::StatePlan("duplicate selection".to_string()));
        }
        let definition = catalogue.get(id).ok_or_else(|| StateResolverError::UnknownTweak(id.clone()))?;
        let binding = bindings.find(&definition.target)?.ok_or_else(|| StateResolverError::MissingBinding(definition.target.key.clone()))?;
        let state = captured.values.get(&binding.reader).cloned().unwrap_or_else(|| ObservedState::Unavailable { reason: "captured state unavailable".to_string() });
        observations.push(TweakObservation { target: definition.target.clone(), state, source: binding.reader.as_str().to_string() });
    }
    TweakEvidence::new(observations).map_err(|error| StateResolverError::StatePlan(error.to_string()))
}

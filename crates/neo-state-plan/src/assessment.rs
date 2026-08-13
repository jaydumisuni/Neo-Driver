use crate::{ObservedState, StatePlanError, TweakCatalogue, TweakEvidence, TweakOperation};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TweakAssessmentItem {
    pub id: String,
    pub title: String,
    pub target_key: String,
    pub current_state: String,
    pub desired_state: String,
    pub already_satisfied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TweakAssessment {
    pub mission_id: String,
    pub items: Vec<TweakAssessmentItem>,
}

pub fn assess_tweaks(
    catalogue: &TweakCatalogue,
    evidence: &TweakEvidence,
    selected_ids: &[String],
    mission_id: impl Into<String>,
) -> Result<TweakAssessment, StatePlanError> {
    catalogue.validate()?;
    evidence.validate()?;
    if selected_ids.is_empty() {
        return Err(StatePlanError::EmptySelection);
    }

    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for id in selected_ids {
        if !seen.insert(id.as_str()) {
            return Err(StatePlanError::DuplicateSelection(id.clone()));
        }
        let definition = catalogue
            .get(id)
            .ok_or_else(|| StatePlanError::UnknownTweak(id.clone()))?;
        let observation = evidence
            .find(&definition.target)?
            .ok_or_else(|| StatePlanError::MissingObservation(id.clone()))?;
        if let ObservedState::Unavailable { reason } = &observation.state {
            return Err(StatePlanError::UnavailableObservation {
                tweak_id: id.clone(),
                reason: reason.clone(),
            });
        }
        items.push(TweakAssessmentItem {
            id: definition.id.clone(),
            title: definition.title.clone(),
            target_key: definition.target.canonical_key()?,
            current_state: state_text(&observation.state)?,
            desired_state: definition.operation.desired()?,
            already_satisfied: satisfied(&definition.operation, &observation.state),
        });
    }

    Ok(TweakAssessment {
        mission_id: mission_id.into(),
        items,
    })
}

fn satisfied(operation: &TweakOperation, state: &ObservedState) -> bool {
    match (operation, state) {
        (TweakOperation::Set { value: expected }, ObservedState::Present { value }) => value == expected,
        (TweakOperation::Delete, ObservedState::Absent) => true,
        _ => false,
    }
}

fn state_text(state: &ObservedState) -> Result<String, StatePlanError> {
    match state {
        ObservedState::Present { value } => value.canonical_json(),
        ObservedState::Absent => Ok("absent".to_string()),
        ObservedState::Unavailable { reason } => Ok(format!("unavailable:{reason}")),
    }
}

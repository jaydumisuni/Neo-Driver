use crate::error::TransactionError;
use crate::plan::TransactionPlan;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateTargetKind {
    RegistryValue,
    Service,
    DriverBinding,
    SecurityState,
    WindowsFeature,
    AppxPackage,
    File,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StateTarget {
    pub kind: StateTargetKind,
    pub key: String,
}

impl StateTarget {
    pub(crate) fn validate(&self) -> Result<(), TransactionError> {
        if self.key.trim().is_empty() {
            return Err(TransactionError::EmptyStateTarget);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub enum CapturedValue {
    Present(String),
    Absent,
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedState {
    pub target: StateTarget,
    pub value: CapturedValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineSnapshot {
    pub states: Vec<CapturedState>,
}

impl BaselineSnapshot {
    pub fn for_plan(
        plan: &TransactionPlan,
        states: Vec<CapturedState>,
    ) -> Result<Self, TransactionError> {
        let snapshot = Self { states };
        snapshot.validate_for_plan(plan)?;
        Ok(snapshot)
    }

    pub fn get(&self, target: &StateTarget) -> Option<&CapturedValue> {
        self.states
            .iter()
            .find(|state| &state.target == target)
            .map(|state| &state.value)
    }

    pub(crate) fn validate_for_plan(&self, plan: &TransactionPlan) -> Result<(), TransactionError> {
        let expected = plan.required_snapshot_targets();
        let mut actual = BTreeSet::new();
        for state in &self.states {
            state.target.validate()?;
            if !actual.insert(state.target.clone()) {
                return Err(TransactionError::DuplicateBaselineTarget(
                    state.target.key.clone(),
                ));
            }
        }
        if actual != expected {
            return Err(TransactionError::BaselineCoverageMismatch);
        }

        let rollback_targets = plan.required_rollback_targets();
        for state in &self.states {
            if rollback_targets.contains(&state.target) {
                if let CapturedValue::Unavailable(reason) = &state.value {
                    return Err(TransactionError::RollbackBaselineUnavailable {
                        target: state.target.key.clone(),
                        reason: reason.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub enum ObservedValue {
    Present(String),
    Absent,
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub target: StateTarget,
    pub value: ObservedValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum VerificationExpectation {
    Equals(String),
    Present,
    Absent,
    MatchesBaseline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationPredicate {
    pub id: String,
    pub target: StateTarget,
    pub expectation: VerificationExpectation,
    pub required: bool,
}

impl VerificationPredicate {
    fn validate(&self) -> Result<(), TransactionError> {
        if self.id.trim().is_empty() {
            return Err(TransactionError::EmptyPredicateId);
        }
        self.target.validate()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Pass,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationResult {
    pub predicate: VerificationPredicate,
    pub observed: ObservedValue,
}

impl VerificationResult {
    pub fn status(&self, baseline: &BaselineSnapshot) -> VerificationStatus {
        match &self.predicate.expectation {
            VerificationExpectation::Equals(expected) => match &self.observed {
                ObservedValue::Present(actual) if actual == expected => VerificationStatus::Pass,
                ObservedValue::Unavailable(_) => VerificationStatus::Unknown,
                _ => VerificationStatus::Fail,
            },
            VerificationExpectation::Present => match &self.observed {
                ObservedValue::Present(_) => VerificationStatus::Pass,
                ObservedValue::Unavailable(_) => VerificationStatus::Unknown,
                ObservedValue::Absent => VerificationStatus::Fail,
            },
            VerificationExpectation::Absent => match &self.observed {
                ObservedValue::Absent => VerificationStatus::Pass,
                ObservedValue::Unavailable(_) => VerificationStatus::Unknown,
                ObservedValue::Present(_) => VerificationStatus::Fail,
            },
            VerificationExpectation::MatchesBaseline => {
                match baseline.get(&self.predicate.target) {
                    Some(CapturedValue::Present(expected)) => match &self.observed {
                        ObservedValue::Present(actual) if actual == expected => {
                            VerificationStatus::Pass
                        }
                        ObservedValue::Unavailable(_) => VerificationStatus::Unknown,
                        _ => VerificationStatus::Fail,
                    },
                    Some(CapturedValue::Absent) => match &self.observed {
                        ObservedValue::Absent => VerificationStatus::Pass,
                        ObservedValue::Unavailable(_) => VerificationStatus::Unknown,
                        ObservedValue::Present(_) => VerificationStatus::Fail,
                    },
                    Some(CapturedValue::Unavailable(_)) | None => VerificationStatus::Unknown,
                }
            }
        }
    }
}

pub(crate) fn ensure_unique_targets(targets: &[StateTarget]) -> Result<(), TransactionError> {
    let mut seen = BTreeSet::new();
    for target in targets {
        target.validate()?;
        if !seen.insert(target.clone()) {
            return Err(TransactionError::DuplicateStateTarget(target.key.clone()));
        }
    }
    Ok(())
}

pub(crate) fn set_of_targets(targets: &[StateTarget]) -> BTreeSet<StateTarget> {
    targets.iter().cloned().collect()
}

pub(crate) fn ensure_unique_predicates(
    predicates: &[VerificationPredicate],
) -> Result<(), TransactionError> {
    let mut seen = BTreeSet::new();
    for predicate in predicates {
        predicate.validate()?;
        if !seen.insert(predicate.id.as_str()) {
            return Err(TransactionError::DuplicatePredicateId(predicate.id.clone()));
        }
    }
    Ok(())
}

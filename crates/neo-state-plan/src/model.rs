use crate::StatePlanError;
use neo_core::{EvidenceVerdict, RebootRequirement, RecommendationState, RiskLevel};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TweakDefinition {
    pub id: String,
    pub title: String,
    pub category: String,
    pub benefit: String,
    pub tradeoff: String,
    pub risk: RiskLevel,
    pub recommendation: RecommendationState,
    pub verdict: EvidenceVerdict,
    pub selected_by_default: bool,
    pub requires_admin: bool,
    pub reboot: RebootRequirement,
    pub target: TweakTarget,
    pub operation: TweakOperation,
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl TweakDefinition {
    pub fn validate(&self) -> Result<(), StatePlanError> {
        validate_id(&self.id)?;
        require_text("title", &self.title)?;
        require_text("category", &self.category)?;
        require_text("benefit", &self.benefit)?;
        require_text("tradeoff", &self.tradeoff)?;
        self.target.validate()?;
        if self.risk >= RiskLevel::High && self.selected_by_default {
            return Err(StatePlanError::HighRiskPreselected(self.id.clone()));
        }
        if self.selected_by_default && self.verdict != EvidenceVerdict::Certified {
            return Err(StatePlanError::NonCertifiedPreselected(self.id.clone()));
        }
        if self.selected_by_default
            && matches!(
                self.recommendation,
                RecommendationState::Conflict
                    | RecommendationState::Unsupported
                    | RecommendationState::DoNotTouch
                    | RecommendationState::Unknown
            )
        {
            return Err(StatePlanError::UnsafeRecommendationPreselected(
                self.id.clone(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TweakCatalogueWire")]
pub struct TweakCatalogue {
    pub tweaks: Vec<TweakDefinition>,
}

#[derive(Debug, Deserialize)]
struct TweakCatalogueWire {
    tweaks: Vec<TweakDefinition>,
}

impl TryFrom<TweakCatalogueWire> for TweakCatalogue {
    type Error = StatePlanError;

    fn try_from(value: TweakCatalogueWire) -> Result<Self, Self::Error> {
        Self::new(value.tweaks)
    }
}

impl TweakCatalogue {
    pub fn new(tweaks: Vec<TweakDefinition>) -> Result<Self, StatePlanError> {
        let catalogue = Self { tweaks };
        catalogue.validate()?;
        Ok(catalogue)
    }

    pub fn validate(&self) -> Result<(), StatePlanError> {
        let mut ids = BTreeSet::new();
        let mut targets = BTreeSet::new();
        for tweak in &self.tweaks {
            tweak.validate()?;
            if !ids.insert(tweak.id.as_str()) {
                return Err(StatePlanError::DuplicateId(tweak.id.clone()));
            }
            let target = tweak.target.canonical_key()?;
            if !targets.insert(target.clone()) {
                return Err(StatePlanError::DuplicateTarget(target));
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

    pub fn get(&self, id: &str) -> Option<&TweakDefinition> {
        self.tweaks.iter().find(|tweak| tweak.id == id)
    }
}

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

fn validate_id(value: &str) -> Result<(), StatePlanError> {
    if value.is_empty()
        || value != value.to_ascii_lowercase()
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-')
        })
    {
        return Err(StatePlanError::InvalidId(value.to_string()));
    }
    Ok(())
}

fn require_text(label: &'static str, value: &str) -> Result<(), StatePlanError> {
    if value.trim().is_empty() {
        return Err(StatePlanError::EmptyField(label));
    }
    Ok(())
}

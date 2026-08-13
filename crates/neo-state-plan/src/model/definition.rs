use super::{TweakOperation, TweakTarget};
use crate::StatePlanError;
use neo_core::{EvidenceVerdict, RebootRequirement, RecommendationState, RiskLevel};
use serde::{Deserialize, Serialize};

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

fn validate_id(value: &str) -> Result<(), StatePlanError> {
    let valid = value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
    });
    if value.is_empty() || value != value.to_ascii_lowercase() || !valid {
        return Err(StatePlanError::InvalidId(value.to_string()));
    }
    Ok(())
}

pub(crate) fn require_text(label: &'static str, value: &str) -> Result<(), StatePlanError> {
    if value.trim().is_empty() {
        return Err(StatePlanError::EmptyField(label));
    }
    Ok(())
}

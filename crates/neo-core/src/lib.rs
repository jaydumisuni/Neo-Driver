//! Neo Driver core contracts.
//!
//! This crate intentionally contains no OS mutation code. It defines the
//! model-free evidence, plan, authority, risk, and verification contracts that
//! every GUI, CLI, probe, installer, tweak, repair, and runtime module must use.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserDepth {
    Beginner,
    Standard,
    Expert,
}

impl fmt::Display for UserDepth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Beginner => "beginner",
            Self::Standard => "standard",
            Self::Expert => "expert",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserIntent {
    SetupPc,
    FixProblem,
    InstallDrivers,
    PrepareGaming,
    PrepareTechnician,
    ImproveWindows,
    DebloatWindows,
    RepairDevices,
    Advanced,
}

impl fmt::Display for UserIntent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::SetupPc => "setup_pc",
            Self::FixProblem => "fix_problem",
            Self::InstallDrivers => "install_drivers",
            Self::PrepareGaming => "prepare_gaming",
            Self::PrepareTechnician => "prepare_technician",
            Self::ImproveWindows => "improve_windows",
            Self::DebloatWindows => "debloat_windows",
            Self::RepairDevices => "repair_devices",
            Self::Advanced => "advanced",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Normal,
    Elevated,
    High,
    Expert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationState {
    Required,
    Repair,
    Recommended,
    Healthy,
    OptionalUpdate,
    OptionalComponent,
    OemPreferred,
    GenericAvailable,
    Conflict,
    Unsupported,
    DoNotTouch,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceVerdict {
    Certified,
    Provisional,
    Investigate,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Scan,
    DriverInstall,
    DriverRepair,
    RuntimeInstall,
    RuntimeRepair,
    Debloat,
    Tweak,
    WindowsFeature,
    Repair,
    ToolInstall,
    SecurityChange,
    Reboot,
}

impl ActionKind {
    pub fn mutates_machine(self) -> bool {
        !matches!(self, Self::Scan)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RebootRequirement {
    None,
    Possible,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionStage {
    Planned,
    Authorized,
    BaselineCaptured,
    Applying,
    AwaitingReboot,
    Verifying,
    RollingBack,
    Complete,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub key: String,
    pub value: String,
    pub source: String,
}

impl EvidenceItem {
    pub fn new(
        key: impl Into<String>,
        value: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            source: source.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedAction {
    pub id: String,
    pub title: String,
    pub kind: ActionKind,
    pub risk: RiskLevel,
    pub recommendation: RecommendationState,
    pub verdict: EvidenceVerdict,
    pub rationale: String,
    pub selected_by_default: bool,
    pub requires_confirmation: bool,
    pub requires_admin: bool,
    pub reboot: RebootRequirement,
    pub rollback_available: bool,
    pub evidence: Vec<EvidenceItem>,
    pub warnings: Vec<String>,
}

impl PlannedAction {
    pub fn validate(&self) -> Result<(), PlanValidationError> {
        if self.id.trim().is_empty() {
            return Err(PlanValidationError::MissingActionId);
        }
        if self.title.trim().is_empty() {
            return Err(PlanValidationError::MissingActionTitle {
                action_id: self.id.clone(),
            });
        }
        if self.kind.mutates_machine() && self.rationale.trim().is_empty() {
            return Err(PlanValidationError::MissingActionRationale {
                action_id: self.id.clone(),
            });
        }
        if self.kind.mutates_machine() && self.evidence.is_empty() {
            return Err(PlanValidationError::MutationWithoutEvidence {
                action_id: self.id.clone(),
            });
        }
        if self.kind.mutates_machine() && !self.requires_confirmation {
            return Err(PlanValidationError::MutationWithoutConfirmation {
                action_id: self.id.clone(),
            });
        }
        if self.risk >= RiskLevel::High && self.selected_by_default {
            return Err(PlanValidationError::HighRiskPreselected {
                action_id: self.id.clone(),
            });
        }
        if matches!(
            self.recommendation,
            RecommendationState::Conflict
                | RecommendationState::Unsupported
                | RecommendationState::DoNotTouch
                | RecommendationState::Unknown
        ) && self.selected_by_default
        {
            return Err(PlanValidationError::UnsafeRecommendationPreselected {
                action_id: self.id.clone(),
            });
        }
        if self.selected_by_default && self.verdict != EvidenceVerdict::Certified {
            return Err(PlanValidationError::NonCertifiedActionPreselected {
                action_id: self.id.clone(),
                verdict: self.verdict,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionPlan {
    pub mission_id: String,
    pub intent: UserIntent,
    pub user_depth: UserDepth,
    pub stage: MissionStage,
    pub actions: Vec<PlannedAction>,
    pub warnings: Vec<String>,
}

impl MissionPlan {
    pub fn new(
        mission_id: impl Into<String>,
        intent: UserIntent,
        user_depth: UserDepth,
    ) -> Self {
        Self {
            mission_id: mission_id.into(),
            intent,
            user_depth,
            stage: MissionStage::Planned,
            actions: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn selected_actions(&self) -> impl Iterator<Item = &PlannedAction> {
        self.actions.iter().filter(|action| action.selected_by_default)
    }

    pub fn validate(&self) -> Result<(), PlanValidationError> {
        if self.mission_id.trim().is_empty() {
            return Err(PlanValidationError::MissingMissionId);
        }

        let mut action_ids = BTreeSet::new();
        for action in &self.actions {
            action.validate()?;
            if !action_ids.insert(action.id.as_str()) {
                return Err(PlanValidationError::DuplicateActionId {
                    action_id: action.id.clone(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OsIdentity {
    pub product_name: Option<String>,
    pub display_version: Option<String>,
    pub build_number: Option<String>,
    pub update_build_revision: Option<String>,
    pub installation_type: Option<String>,
    pub architecture: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SecurityState {
    pub test_signing: Option<bool>,
    pub no_integrity_checks: Option<bool>,
    pub secure_boot: Option<bool>,
    pub memory_integrity: Option<bool>,
    pub pending_reboot: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MachineProfile {
    pub os: OsIdentity,
    pub security: SecurityState,
    pub evidence: Vec<EvidenceItem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PlanValidationError {
    #[error("mission id is required")]
    MissingMissionId,
    #[error("action id is required")]
    MissingActionId,
    #[error("action '{action_id}' is missing a title")]
    MissingActionTitle { action_id: String },
    #[error("mutating action '{action_id}' is missing a rationale")]
    MissingActionRationale { action_id: String },
    #[error("mutating action '{action_id}' has no supporting evidence")]
    MutationWithoutEvidence { action_id: String },
    #[error("mutating action '{action_id}' does not require user confirmation")]
    MutationWithoutConfirmation { action_id: String },
    #[error("high-risk action '{action_id}' must not be preselected")]
    HighRiskPreselected { action_id: String },
    #[error("unsafe recommendation '{action_id}' must not be preselected")]
    UnsafeRecommendationPreselected { action_id: String },
    #[error("action '{action_id}' with verdict '{verdict:?}' must not be preselected")]
    NonCertifiedActionPreselected {
        action_id: String,
        verdict: EvidenceVerdict,
    },
    #[error("duplicate action id '{action_id}'")]
    DuplicateActionId { action_id: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(kind: ActionKind) -> PlannedAction {
        PlannedAction {
            id: "neo.test".to_string(),
            title: "Test action".to_string(),
            kind,
            risk: RiskLevel::Normal,
            recommendation: RecommendationState::Recommended,
            verdict: EvidenceVerdict::Certified,
            rationale: "fixture rationale".to_string(),
            selected_by_default: true,
            requires_confirmation: true,
            requires_admin: false,
            reboot: RebootRequirement::None,
            rollback_available: true,
            evidence: vec![EvidenceItem::new("fixture", "true", "unit-test")],
            warnings: vec![],
        }
    }

    #[test]
    fn scan_may_be_unconfirmed_and_have_no_evidence() {
        let mut a = action(ActionKind::Scan);
        a.requires_confirmation = false;
        a.evidence.clear();
        a.rationale.clear();
        assert_eq!(a.validate(), Ok(()));
    }

    #[test]
    fn mutation_requires_confirmation() {
        let mut a = action(ActionKind::Tweak);
        a.requires_confirmation = false;
        assert!(matches!(
            a.validate(),
            Err(PlanValidationError::MutationWithoutConfirmation { .. })
        ));
    }

    #[test]
    fn mutation_requires_rationale() {
        let mut a = action(ActionKind::DriverInstall);
        a.rationale.clear();
        assert!(matches!(
            a.validate(),
            Err(PlanValidationError::MissingActionRationale { .. })
        ));
    }

    #[test]
    fn mutation_requires_evidence() {
        let mut a = action(ActionKind::RuntimeInstall);
        a.evidence.clear();
        assert!(matches!(
            a.validate(),
            Err(PlanValidationError::MutationWithoutEvidence { .. })
        ));
    }

    #[test]
    fn high_risk_action_cannot_be_preselected() {
        let mut a = action(ActionKind::SecurityChange);
        a.risk = RiskLevel::High;
        assert!(matches!(
            a.validate(),
            Err(PlanValidationError::HighRiskPreselected { .. })
        ));
    }

    #[test]
    fn unknown_action_cannot_be_preselected() {
        let mut a = action(ActionKind::DriverInstall);
        a.recommendation = RecommendationState::Unknown;
        assert!(matches!(
            a.validate(),
            Err(PlanValidationError::UnsafeRecommendationPreselected { .. })
        ));
    }

    #[test]
    fn provisional_action_cannot_be_preselected() {
        let mut a = action(ActionKind::DriverInstall);
        a.verdict = EvidenceVerdict::Provisional;
        assert!(matches!(
            a.validate(),
            Err(PlanValidationError::NonCertifiedActionPreselected { .. })
        ));
    }

    #[test]
    fn investigate_action_cannot_be_preselected() {
        let mut a = action(ActionKind::DriverInstall);
        a.verdict = EvidenceVerdict::Investigate;
        assert!(matches!(
            a.validate(),
            Err(PlanValidationError::NonCertifiedActionPreselected { .. })
        ));
    }

    #[test]
    fn duplicate_action_ids_are_rejected() {
        let mut plan = MissionPlan::new("NEO-TEST", UserIntent::SetupPc, UserDepth::Standard);
        plan.actions.push(action(ActionKind::DriverInstall));
        plan.actions.push(action(ActionKind::RuntimeInstall));
        assert!(matches!(
            plan.validate(),
            Err(PlanValidationError::DuplicateActionId { .. })
        ));
    }
}

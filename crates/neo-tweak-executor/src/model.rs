use crate::TweakExecutionError;
use neo_core::{EvidenceVerdict, RecommendationState, RiskLevel};
use neo_state_plan::{TweakAssessment, TweakDefinition, TweakOperation, TweakTarget, TweakValue};
use neo_transaction::{TransactionCheckpoint, TransactionPlan, TransactionStage};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const SHOW_FILE_EXTENSIONS: &str = "windows.explorer.show_file_extensions";
pub const SHOW_HIDDEN_FILES: &str = "windows.explorer.show_hidden_files";
pub const TASKBAR_CENTERED_ICONS: &str = "windows.taskbar.centered_icons";

const EXPLORER_ADVANCED: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub enum RegistrySnapshot {
    Absent,
    Dword(u32),
}

impl RegistrySnapshot {
    pub(crate) fn encoded(self) -> Result<String, TweakExecutionError> {
        Ok(serde_json::to_string(&self)?)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RegistryTweakSpec {
    pub id: &'static str,
    pub target_key: &'static str,
    pub subkey: &'static str,
    pub value_name: &'static str,
    pub title: &'static str,
    pub risk: RiskLevel,
}

impl RegistryTweakSpec {
    pub fn state_target_key(self) -> String {
        format!("hkcu\\{}|{}", self.subkey, self.value_name)
    }
}

pub(crate) fn spec_for_id(id: &str) -> Option<RegistryTweakSpec> {
    match id {
        SHOW_FILE_EXTENSIONS => Some(RegistryTweakSpec {
            id: SHOW_FILE_EXTENSIONS,
            target_key: SHOW_FILE_EXTENSIONS,
            subkey: EXPLORER_ADVANCED,
            value_name: "HideFileExt",
            title: "Show file extensions",
            risk: RiskLevel::Low,
        }),
        SHOW_HIDDEN_FILES => Some(RegistryTweakSpec {
            id: SHOW_HIDDEN_FILES,
            target_key: SHOW_HIDDEN_FILES,
            subkey: EXPLORER_ADVANCED,
            value_name: "Hidden",
            title: "Show hidden files",
            risk: RiskLevel::Low,
        }),
        TASKBAR_CENTERED_ICONS => Some(RegistryTweakSpec {
            id: TASKBAR_CENTERED_ICONS,
            target_key: TASKBAR_CENTERED_ICONS,
            subkey: EXPLORER_ADVANCED,
            value_name: "TaskbarAl",
            title: "Taskbar centered icons",
            risk: RiskLevel::Low,
        }),
        _ => None,
    }
}

pub fn curated_tweak_ids() -> [&'static str; 3] {
    [SHOW_FILE_EXTENSIONS, SHOW_HIDDEN_FILES, TASKBAR_CENTERED_ICONS]
}

pub(crate) fn validate_definition(
    definition: &TweakDefinition,
) -> Result<(RegistryTweakSpec, u32), TweakExecutionError> {
    definition.validate()?;
    let spec = spec_for_id(&definition.id)
        .ok_or_else(|| TweakExecutionError::UnsupportedTweak(definition.id.clone()))?;
    let expected_target = TweakTarget {
        key: spec.target_key.to_string(),
    };
    if definition.target.canonical_key()? != expected_target.canonical_key()? {
        return Err(TweakExecutionError::TargetMismatch(definition.id.clone()));
    }
    if definition.verdict != EvidenceVerdict::Certified {
        return Err(TweakExecutionError::NonCertifiedTweak(
            definition.id.clone(),
        ));
    }
    let desired = match &definition.operation {
        TweakOperation::Set {
            value: TweakValue::U32(value),
        } if *value <= 1 => *value,
        _ => {
            return Err(TweakExecutionError::UnsupportedOperation(
                definition.id.clone(),
            ))
        }
    };
    Ok((spec, desired))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TweakExecutionStep {
    tweak_id: String,
    desired_dword: u32,
    baseline: RegistrySnapshot,
}

impl TweakExecutionStep {
    pub fn tweak_id(&self) -> &str {
        &self.tweak_id
    }

    pub fn desired_dword(&self) -> u32 {
        self.desired_dword
    }

    pub fn baseline(&self) -> RegistrySnapshot {
        self.baseline
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TweakExecutionPlan {
    assessment: TweakAssessment,
    steps: Vec<TweakExecutionStep>,
    transaction: TransactionPlan,
}

impl TweakExecutionPlan {
    pub(crate) fn new(
        assessment: TweakAssessment,
        steps: Vec<TweakExecutionStep>,
        transaction: TransactionPlan,
    ) -> Result<Self, TweakExecutionError> {
        if steps.is_empty() {
            return Err(TweakExecutionError::NothingToChange);
        }
        let mut ids = BTreeSet::new();
        for step in &steps {
            if !ids.insert(step.tweak_id.as_str()) || spec_for_id(&step.tweak_id).is_none() {
                return Err(TweakExecutionError::UnsupportedTweak(step.tweak_id.clone()));
            }
            if step.desired_dword > 1 {
                return Err(TweakExecutionError::UnsupportedOperation(
                    step.tweak_id.clone(),
                ));
            }
        }
        transaction.validate()?;
        Ok(Self {
            assessment,
            steps,
            transaction,
        })
    }

    pub fn assessment(&self) -> &TweakAssessment {
        &self.assessment
    }

    pub fn steps(&self) -> &[TweakExecutionStep] {
        &self.steps
    }

    pub fn transaction(&self) -> &TransactionPlan {
        &self.transaction
    }
}

#[derive(Debug, Clone)]
pub struct TweakExecutionSession {
    pub(crate) plan: TweakExecutionPlan,
    pub(crate) checkpoint: TransactionCheckpoint,
    pub(crate) changed_ids: BTreeSet<String>,
}

impl TweakExecutionSession {
    pub(crate) fn new(
        plan: TweakExecutionPlan,
        checkpoint: TransactionCheckpoint,
    ) -> Result<Self, TweakExecutionError> {
        if checkpoint.plan_fingerprint() != plan.transaction.fingerprint()? {
            return Err(TweakExecutionError::Transaction(
                neo_transaction::TransactionError::CheckpointFingerprintMismatch,
            ));
        }
        Ok(Self {
            plan,
            checkpoint,
            changed_ids: BTreeSet::new(),
        })
    }

    pub fn plan(&self) -> &TweakExecutionPlan {
        &self.plan
    }

    pub fn checkpoint(&self) -> &TransactionCheckpoint {
        &self.checkpoint
    }

    pub fn stage(&self) -> TransactionStage {
        self.checkpoint.stage()
    }
}

#[derive(Debug)]
pub struct TweakExecutorCapability {
    _private: (),
}

impl TweakExecutorCapability {
    #[cfg(test)]
    pub(crate) fn for_tests() -> Self {
        Self { _private: () }
    }
}

pub(crate) fn fixed_recommendation() -> RecommendationState {
    RecommendationState::Recommended
}

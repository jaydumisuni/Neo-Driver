#[cfg(any(windows, test))]
use crate::TweakExecutionError;
#[cfg(any(windows, test))]
use neo_core::{EvidenceVerdict, RecommendationState, RiskLevel};
use neo_state_plan::TweakAssessment;
#[cfg(any(windows, test))]
use neo_state_plan::{TweakDefinition, TweakOperation, TweakTarget, TweakValue};
use neo_transaction::{TransactionCheckpoint, TransactionPlan, TransactionStage};
use serde::{Deserialize, Serialize};
#[cfg(any(windows, test))]
use std::collections::BTreeSet;

pub const SHOW_FILE_EXTENSIONS: &str = "windows.explorer.show_file_extensions";
pub const SHOW_HIDDEN_FILES: &str = "windows.explorer.show_hidden_files";
pub const TASKBAR_CENTERED_ICONS: &str = "windows.taskbar.centered_icons";

#[cfg(any(windows, test))]
const EXPLORER_ADVANCED: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub enum RegistrySnapshot {
    Absent,
    Dword(u32),
}

#[cfg(any(windows, test))]
impl RegistrySnapshot {
    pub(crate) fn encoded(self) -> Result<String, TweakExecutionError> {
        Ok(serde_json::to_string(&self)?)
    }
}

#[cfg(any(windows, test))]
#[derive(Debug, Clone, Copy)]
pub(crate) struct RegistryTweakSpec {
    pub id: &'static str,
    pub target_key: &'static str,
    pub subkey: &'static str,
    pub value_name: &'static str,
    pub desired_dword: u32,
    pub title: &'static str,
    pub risk: RiskLevel,
}

#[cfg(any(windows, test))]
impl RegistryTweakSpec {
    pub fn state_target_key(self) -> String {
        format!("hkcu\\{}|{}", self.subkey, self.value_name)
    }
}

#[cfg(any(windows, test))]
pub(crate) fn spec_for_id(id: &str) -> Option<RegistryTweakSpec> {
    match id {
        SHOW_FILE_EXTENSIONS => Some(RegistryTweakSpec {
            id: SHOW_FILE_EXTENSIONS,
            target_key: SHOW_FILE_EXTENSIONS,
            subkey: EXPLORER_ADVANCED,
            value_name: "HideFileExt",
            desired_dword: 0,
            title: "Show file extensions",
            risk: RiskLevel::Low,
        }),
        SHOW_HIDDEN_FILES => Some(RegistryTweakSpec {
            id: SHOW_HIDDEN_FILES,
            target_key: SHOW_HIDDEN_FILES,
            subkey: EXPLORER_ADVANCED,
            value_name: "Hidden",
            desired_dword: 1,
            title: "Show hidden files",
            risk: RiskLevel::Low,
        }),
        TASKBAR_CENTERED_ICONS => Some(RegistryTweakSpec {
            id: TASKBAR_CENTERED_ICONS,
            target_key: TASKBAR_CENTERED_ICONS,
            subkey: EXPLORER_ADVANCED,
            value_name: "TaskbarAl",
            desired_dword: 1,
            title: "Taskbar centered icons",
            risk: RiskLevel::Low,
        }),
        _ => None,
    }
}

pub fn curated_tweak_ids() -> [&'static str; 3] {
    [
        SHOW_FILE_EXTENSIONS,
        SHOW_HIDDEN_FILES,
        TASKBAR_CENTERED_ICONS,
    ]
}

#[cfg(any(windows, test))]
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
        } if *value == spec.desired_dword => *value,
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
    #[cfg(any(windows, test))]
    pub(crate) fn new(tweak_id: String, desired_dword: u32, baseline: RegistrySnapshot) -> Self {
        Self {
            tweak_id,
            desired_dword,
            baseline,
        }
    }

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
    #[cfg(any(windows, test))]
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
            if !ids.insert(step.tweak_id.as_str()) {
                return Err(TweakExecutionError::UnsupportedTweak(step.tweak_id.clone()));
            }
            let spec = spec_for_id(&step.tweak_id)
                .ok_or_else(|| TweakExecutionError::UnsupportedTweak(step.tweak_id.clone()))?;
            if step.desired_dword != spec.desired_dword {
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
    #[cfg(any(windows, test))]
    pub(crate) changed_ids: BTreeSet<String>,
}

impl TweakExecutionSession {
    #[cfg(any(windows, test))]
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

    #[cfg(any(windows, test))]
    pub(crate) fn for_rpc() -> Self {
        Self { _private: () }
    }
}

#[cfg(any(windows, test))]
pub(crate) fn fixed_recommendation() -> RecommendationState {
    RecommendationState::Recommended
}

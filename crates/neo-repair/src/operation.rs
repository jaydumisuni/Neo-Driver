use crate::model::{
    ComponentStoreState, FeatureDesiredState, SupportedWindowsFeature, SystemFileState,
    WindowsFeatureState,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RepairOperation {
    RestoreComponentStore,
    RepairSystemFiles,
    SetWindowsFeature {
        feature: SupportedWindowsFeature,
        desired: FeatureDesiredState,
    },
}

impl RepairOperation {
    pub fn action_id(self) -> String {
        match self {
            Self::RestoreComponentStore => "repair.component_store.restore_health".to_string(),
            Self::RepairSystemFiles => "repair.system_files.scannow".to_string(),
            Self::SetWindowsFeature { feature, desired } => format!(
                "windows_feature.{}.{}",
                feature.id(),
                match desired {
                    FeatureDesiredState::Enabled => "enable",
                    FeatureDesiredState::Disabled => "disable",
                }
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "state", rename_all = "snake_case")]
pub enum RepairBaseline {
    ComponentStore(ComponentStoreState),
    SystemFiles(SystemFileState),
    WindowsFeature {
        feature: SupportedWindowsFeature,
        state: WindowsFeatureState,
    },
}

impl RepairBaseline {
    pub fn transaction_value(self) -> &'static str {
        match self {
            Self::ComponentStore(ComponentStoreState::Healthy) => "healthy",
            Self::ComponentStore(ComponentStoreState::Repairable) => "repairable",
            Self::ComponentStore(ComponentStoreState::Unrepairable) => "unrepairable",
            Self::ComponentStore(ComponentStoreState::Unavailable) => "unavailable",
            Self::SystemFiles(SystemFileState::Healthy) => "healthy",
            Self::SystemFiles(SystemFileState::IntegrityViolations) => "integrity_violations",
            Self::SystemFiles(SystemFileState::Unavailable) => "unavailable",
            Self::WindowsFeature { state, .. } => state.as_transaction_value(),
        }
    }
}

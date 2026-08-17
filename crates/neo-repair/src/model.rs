use neo_core::{RebootRequirement, RiskLevel};
use neo_probe::CommandEvidence;
use serde::{Deserialize, Serialize};

pub const MAX_REPAIR_EVIDENCE_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairTarget {
    ComponentStore,
    SystemFiles,
}

impl RepairTarget {
    pub fn id(self) -> &'static str {
        match self {
            Self::ComponentStore => "component_store",
            Self::SystemFiles => "system_files",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::ComponentStore => "Windows component store",
            Self::SystemFiles => "Protected Windows system files",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentStoreState {
    Healthy,
    Repairable,
    Unrepairable,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemFileState {
    Healthy,
    IntegrityViolations,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowsFeatureState {
    Enabled,
    Disabled,
    EnablePending,
    DisablePending,
    Removed,
    Unavailable,
}

impl WindowsFeatureState {
    pub fn is_stable(self) -> bool {
        matches!(self, Self::Enabled | Self::Disabled)
    }

    pub fn is_pending(self) -> bool {
        matches!(self, Self::EnablePending | Self::DisablePending)
    }

    pub fn as_transaction_value(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::EnablePending => "enable_pending",
            Self::DisablePending => "disable_pending",
            Self::Removed => "removed",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportedWindowsFeature {
    NetFx3,
    DirectPlay,
    HyperV,
    WindowsSubsystemLinux,
    VirtualMachinePlatform,
    WindowsSandbox,
}

impl SupportedWindowsFeature {
    pub const ALL: [Self; 6] = [
        Self::NetFx3,
        Self::DirectPlay,
        Self::HyperV,
        Self::WindowsSubsystemLinux,
        Self::VirtualMachinePlatform,
        Self::WindowsSandbox,
    ];

    pub fn all() -> &'static [Self] {
        &Self::ALL
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::NetFx3 => "netfx3",
            Self::DirectPlay => "directplay",
            Self::HyperV => "hyper_v",
            Self::WindowsSubsystemLinux => "windows_subsystem_linux",
            Self::VirtualMachinePlatform => "virtual_machine_platform",
            Self::WindowsSandbox => "windows_sandbox",
        }
    }

    pub fn dism_name(self) -> &'static str {
        match self {
            Self::NetFx3 => "NetFx3",
            Self::DirectPlay => "DirectPlay",
            Self::HyperV => "Microsoft-Hyper-V-All",
            Self::WindowsSubsystemLinux => "Microsoft-Windows-Subsystem-Linux",
            Self::VirtualMachinePlatform => "VirtualMachinePlatform",
            Self::WindowsSandbox => "Containers-DisposableClientVM",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::NetFx3 => ".NET Framework 3.5",
            Self::DirectPlay => "DirectPlay",
            Self::HyperV => "Hyper-V",
            Self::WindowsSubsystemLinux => "Windows Subsystem for Linux",
            Self::VirtualMachinePlatform => "Virtual Machine Platform",
            Self::WindowsSandbox => "Windows Sandbox",
        }
    }

    pub fn risk(self) -> RiskLevel {
        match self {
            Self::NetFx3 | Self::DirectPlay => RiskLevel::Low,
            Self::WindowsSubsystemLinux | Self::VirtualMachinePlatform => RiskLevel::Normal,
            Self::HyperV | Self::WindowsSandbox => RiskLevel::Elevated,
        }
    }

    pub fn reboot(self) -> RebootRequirement {
        RebootRequirement::Possible
    }

    pub fn parse_id(value: &str) -> Option<Self> {
        Self::all()
            .iter()
            .copied()
            .find(|feature| feature.id() == value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedCommandEvidence {
    pub program: String,
    pub args: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub start_error: Option<String>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

impl BoundedCommandEvidence {
    pub fn from_command(value: CommandEvidence) -> Self {
        let (stdout, stdout_truncated) = truncate_utf8(&value.stdout, MAX_REPAIR_EVIDENCE_BYTES);
        let (stderr, stderr_truncated) = truncate_utf8(&value.stderr, MAX_REPAIR_EVIDENCE_BYTES);
        Self {
            program: value.program,
            args: value.args,
            exit_code: value.exit_code,
            stdout,
            stderr,
            start_error: value.start_error,
            stdout_truncated,
            stderr_truncated,
        }
    }

    pub fn succeeded(&self) -> bool {
        self.start_error.is_none() && matches!(self.exit_code, Some(0) | Some(3010))
    }

    pub fn truncated(&self) -> bool {
        self.stdout_truncated || self.stderr_truncated
    }

    pub fn combined_text(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

fn truncate_utf8(value: &str, maximum_bytes: usize) -> (String, bool) {
    if value.len() <= maximum_bytes {
        return (value.to_string(), false);
    }
    let mut end = maximum_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    (value[..end].to_string(), true)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentStoreObservation {
    pub state: ComponentStoreState,
    pub elevation_required: bool,
    pub detail: String,
    pub evidence: BoundedCommandEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemFileObservation {
    pub state: SystemFileState,
    pub elevation_required: bool,
    pub detail: String,
    pub evidence: BoundedCommandEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsFeatureObservation {
    pub feature: SupportedWindowsFeature,
    pub state: WindowsFeatureState,
    pub elevation_required: bool,
    pub detail: String,
    pub evidence: BoundedCommandEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairHealthInspectionReport {
    pub component_store: ComponentStoreObservation,
    pub system_files: SystemFileObservation,
    pub machine_changes: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowsFeaturesInspectionReport {
    pub features: Vec<WindowsFeatureObservation>,
    pub machine_changes: bool,
}

impl WindowsFeaturesInspectionReport {
    pub fn feature(&self, feature: SupportedWindowsFeature) -> Option<&WindowsFeatureObservation> {
        self.features.iter().find(|item| item.feature == feature)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairInspectionReport {
    pub component_store: ComponentStoreObservation,
    pub system_files: SystemFileObservation,
    pub features: Vec<WindowsFeatureObservation>,
    pub machine_changes: bool,
}

impl RepairInspectionReport {
    pub fn feature(&self, feature: SupportedWindowsFeature) -> Option<&WindowsFeatureObservation> {
        self.features.iter().find(|item| item.feature == feature)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureDesiredState {
    Enabled,
    Disabled,
}

impl FeatureDesiredState {
    pub fn target_state(self) -> WindowsFeatureState {
        match self {
            Self::Enabled => WindowsFeatureState::Enabled,
            Self::Disabled => WindowsFeatureState::Disabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_feature_catalogue_is_unique() {
        let mut ids = std::collections::BTreeSet::new();
        let mut names = std::collections::BTreeSet::new();
        for feature in SupportedWindowsFeature::all() {
            assert!(ids.insert(feature.id()));
            assert!(names.insert(feature.dism_name().to_ascii_lowercase()));
            assert_eq!(
                SupportedWindowsFeature::parse_id(feature.id()),
                Some(*feature)
            );
        }
    }

    #[test]
    fn servicing_reboot_exit_is_successful() {
        let evidence = BoundedCommandEvidence {
            program: "dism.exe".to_string(),
            args: vec!["/Online".to_string()],
            exit_code: Some(3010),
            stdout: String::new(),
            stderr: String::new(),
            start_error: None,
            stdout_truncated: false,
            stderr_truncated: false,
        };
        assert!(evidence.succeeded());
    }

    #[test]
    fn command_evidence_is_bounded_at_utf8_boundary() {
        let evidence = CommandEvidence {
            program: "tool".to_string(),
            args: vec![],
            exit_code: Some(0),
            stdout: "é".repeat(MAX_REPAIR_EVIDENCE_BYTES),
            stderr: String::new(),
            start_error: None,
        };
        let bounded = BoundedCommandEvidence::from_command(evidence);
        assert!(bounded.stdout_truncated);
        assert!(bounded.stdout.len() <= MAX_REPAIR_EVIDENCE_BYTES);
        assert!(std::str::from_utf8(bounded.stdout.as_bytes()).is_ok());
    }
}

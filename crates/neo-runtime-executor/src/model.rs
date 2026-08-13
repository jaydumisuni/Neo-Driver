use crate::RuntimeExecutorError;
use neo_catalogue::{
    PackageKind, RuntimeExecutionSpec, RuntimeInstallerKind, RuntimeVerificationRule,
    SecurityRequirements,
};
use neo_core::{ActionKind, EvidenceVerdict, PlannedAction};
use neo_runtime::{
    component_key, RuntimeComponent, RuntimeObservation, RuntimeProfile, RuntimeState,
};
use neo_transaction::{StateTarget, StateTargetKind, VerificationExpectation};
use neo_vault::{Sha256Digest, VaultLayout, VaultMode, VaultSegment};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeExecutionOperation {
    Install,
    Repair,
}

impl RuntimeExecutionOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Repair => "repair",
        }
    }

    fn expected_action_kind(self) -> ActionKind {
        match self {
            Self::Install => ActionKind::RuntimeInstall,
            Self::Repair => ActionKind::RuntimeRepair,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeExecutionPlan {
    pub mission_id: String,
    pub transaction_id: String,
    pub profile: RuntimeProfile,
    pub component: RuntimeComponent,
    pub operation: RuntimeExecutionOperation,
    pub package_kind: PackageKind,
    pub package_id: VaultSegment,
    pub package_version: VaultSegment,
    pub package_sha256: Sha256Digest,
    pub package_dependencies: Vec<String>,
    pub package_conflicts: Vec<String>,
    pub package_security: SecurityRequirements,
    pub execution: RuntimeExecutionSpec,
    pub vault_mode: VaultMode,
    pub application_root: PathBuf,
    pub windows_build: u32,
    pub architecture: String,
    pub baseline: RuntimeObservation,
    pub action: PlannedAction,
}

#[derive(Debug, Deserialize)]
struct RuntimeExecutionPlanWire {
    mission_id: String,
    transaction_id: String,
    profile: RuntimeProfile,
    component: RuntimeComponent,
    operation: RuntimeExecutionOperation,
    package_kind: PackageKind,
    package_id: VaultSegment,
    package_version: VaultSegment,
    package_sha256: Sha256Digest,
    #[serde(default)]
    package_dependencies: Vec<String>,
    #[serde(default)]
    package_conflicts: Vec<String>,
    #[serde(default)]
    package_security: SecurityRequirements,
    execution: RuntimeExecutionSpec,
    vault_mode: VaultMode,
    application_root: PathBuf,
    windows_build: u32,
    architecture: String,
    baseline: RuntimeObservation,
    action: PlannedAction,
}

impl<'de> Deserialize<'de> for RuntimeExecutionPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = RuntimeExecutionPlanWire::deserialize(deserializer)?;
        Self::try_from(wire).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<RuntimeExecutionPlanWire> for RuntimeExecutionPlan {
    type Error = RuntimeExecutorError;

    fn try_from(value: RuntimeExecutionPlanWire) -> Result<Self, Self::Error> {
        let plan = Self {
            mission_id: value.mission_id,
            transaction_id: value.transaction_id,
            profile: value.profile,
            component: value.component,
            operation: value.operation,
            package_kind: value.package_kind,
            package_id: value.package_id,
            package_version: value.package_version,
            package_sha256: value.package_sha256,
            package_dependencies: value.package_dependencies,
            package_conflicts: value.package_conflicts,
            package_security: value.package_security,
            execution: value.execution,
            vault_mode: value.vault_mode,
            application_root: value.application_root,
            windows_build: value.windows_build,
            architecture: value.architecture,
            baseline: value.baseline,
            action: value.action,
        };
        plan.validate()?;
        Ok(plan)
    }
}

impl RuntimeExecutionPlan {
    pub fn from_json_str(input: &str) -> Result<Self, RuntimeExecutorError> {
        Ok(serde_json::from_str(input)?)
    }

    pub fn validate(&self) -> Result<(), RuntimeExecutorError> {
        if self.mission_id.trim().is_empty() {
            return Err(RuntimeExecutorError::MissingMissionId);
        }
        if self.transaction_id.trim().is_empty() {
            return Err(RuntimeExecutorError::MissingTransactionId);
        }
        if self.package_kind != PackageKind::Runtime {
            return Err(RuntimeExecutorError::PackageNotRuntime(
                self.package_id.to_string(),
            ));
        }
        if !self.package_dependencies.is_empty() || !self.package_conflicts.is_empty() {
            return Err(RuntimeExecutorError::DependencyClosureRequired(
                self.package_id.to_string(),
            ));
        }
        if self.package_security.changes_boot_or_security_state() {
            return Err(RuntimeExecutorError::SecurityMutationBlocked(
                self.package_id.to_string(),
            ));
        }
        if self.windows_build == 0 {
            return Err(RuntimeExecutorError::InvalidPlan(
                "Windows build must be greater than zero".to_string(),
            ));
        }
        let Some(architecture) = canonical_arch(&self.architecture) else {
            return Err(RuntimeExecutorError::InvalidPlan(format!(
                "unsupported architecture {}",
                self.architecture
            )));
        };
        if architecture != self.architecture {
            return Err(RuntimeExecutorError::InvalidPlan(
                "runtime execution plan architecture must be canonical".to_string(),
            ));
        }
        if matches!(
            self.component,
            RuntimeComponent::DotNetFramework35 | RuntimeComponent::DirectPlay
        ) {
            return Err(RuntimeExecutorError::InvalidPlan(
                "Windows Feature components remain outside Phase 8 direct-installer authority"
                    .to_string(),
            ));
        }
        if self.baseline.component != self.component {
            return Err(RuntimeExecutorError::InvalidPlan(
                "baseline component does not match plan component".to_string(),
            ));
        }
        if self.baseline.source.trim().is_empty() {
            return Err(RuntimeExecutorError::InvalidPlan(
                "baseline observation source cannot be empty".to_string(),
            ));
        }
        match (self.operation, self.baseline.state) {
            (RuntimeExecutionOperation::Install, RuntimeState::Missing)
            | (RuntimeExecutionOperation::Repair, RuntimeState::Broken | RuntimeState::Partial) => {
            }
            _ => {
                return Err(RuntimeExecutorError::OperationStateMismatch {
                    operation: self.operation.as_str(),
                    state: self.baseline.state,
                })
            }
        }
        self.execution
            .validate()
            .map_err(|error| RuntimeExecutorError::Catalogue(error.to_string()))?;
        if self.operation == RuntimeExecutionOperation::Repair
            && self.execution.repair_args.is_none()
        {
            return Err(RuntimeExecutorError::MissingRepairArguments(
                self.package_id.to_string(),
            ));
        }
        self.action
            .validate()
            .map_err(|error| RuntimeExecutorError::ActionMismatch(error.to_string()))?;
        let expected_action_id = format!("runtime.{}", component_key(self.component));
        if self.action.id != expected_action_id
            || self.action.kind != self.operation.expected_action_kind()
            || self.action.verdict != EvidenceVerdict::Certified
            || !self.action.requires_confirmation
            || !self.action.requires_admin
            || self.action.rollback_available
        {
            return Err(RuntimeExecutorError::ActionMismatch(
                "Phase 6 action shape does not match the exact Phase 8 runtime contract"
                    .to_string(),
            ));
        }
        require_exact_evidence(&self.action, "package_id", self.package_id.as_str())?;
        require_exact_evidence(&self.action, "package_sha256", self.package_sha256.as_str())?;
        let layout = self.layout()?;
        layout.ensure_managed(self.payload_path()?)?;
        Ok(())
    }

    pub fn layout(&self) -> Result<VaultLayout, RuntimeExecutorError> {
        Ok(VaultLayout::new(
            self.vault_mode,
            self.application_root.clone(),
        )?)
    }

    pub fn payload_path(&self) -> Result<PathBuf, RuntimeExecutorError> {
        let layout = self.layout()?;
        Ok(layout.runtime_pack_destination(
            &self.package_id,
            &self.package_version,
            self.package_sha256.as_str(),
        ))
    }

    pub fn execution_args(&self) -> Result<Vec<String>, RuntimeExecutorError> {
        match self.operation {
            RuntimeExecutionOperation::Install => Ok(self.execution.install_args.clone()),
            RuntimeExecutionOperation::Repair => {
                self.execution.repair_args.clone().ok_or_else(|| {
                    RuntimeExecutorError::MissingRepairArguments(self.package_id.to_string())
                })
            }
        }
    }

    pub fn state_target(&self) -> StateTarget {
        StateTarget {
            kind: StateTargetKind::Other,
            key: format!("runtime:{}", component_key(self.component)),
        }
    }

    pub fn expected_verification_value(&self) -> String {
        match &self.execution.verification {
            RuntimeVerificationRule::InstalledState => "installed".to_string(),
            RuntimeVerificationRule::ExactDetectedVersion { value } => {
                format!("installed:{value}")
            }
        }
    }

    pub fn verification_expectation(&self) -> VerificationExpectation {
        VerificationExpectation::Equals(self.expected_verification_value())
    }

    pub fn lock_path(&self) -> Result<PathBuf, RuntimeExecutorError> {
        Ok(self.layout()?.sessions().join("runtime-executor.lock"))
    }

    pub fn staged_filename(&self) -> Result<VaultSegment, RuntimeExecutorError> {
        VaultSegment::new(format!(
            "runtime-installer.{}",
            self.execution.installer.staging_extension()
        ))
        .map_err(RuntimeExecutorError::from)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeInvocation {
    pub installer: RuntimeInstallerKind,
    pub payload: PathBuf,
    pub expected_sha256: Sha256Digest,
    pub arguments: Vec<String>,
    pub execution_lock: PathBuf,
}

impl RuntimeInvocation {
    pub fn validate(&self) -> Result<(), RuntimeExecutorError> {
        if self.payload.as_os_str().is_empty() || self.execution_lock.as_os_str().is_empty() {
            return Err(RuntimeExecutorError::InvalidPlan(
                "runtime invocation paths cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeProcessResult {
    pub started: bool,
    #[serde(default)]
    pub exit_code: Option<i32>,
    pub detail: String,
}

impl RuntimeProcessResult {
    pub fn start_failed(detail: impl Into<String>) -> Self {
        Self {
            started: false,
            exit_code: None,
            detail: detail.into(),
        }
    }

    pub fn exited(exit_code: i32, detail: impl Into<String>) -> Self {
        Self {
            started: true,
            exit_code: Some(exit_code),
            detail: detail.into(),
        }
    }

    pub fn started_without_exit(detail: impl Into<String>) -> Self {
        Self {
            started: true,
            exit_code: None,
            detail: detail.into(),
        }
    }
}

pub(crate) fn canonical_arch(value: &str) -> Option<String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "x64" | "amd64" | "x86_64" => Some("x64".to_string()),
        "x86" | "i386" | "i686" => Some("x86".to_string()),
        "arm64" | "aarch64" => Some("arm64".to_string()),
        _ => None,
    }
}

pub(crate) fn runtime_baseline_value(observation: &RuntimeObservation) -> String {
    let state = match observation.state {
        RuntimeState::Installed => "installed",
        RuntimeState::Missing => "missing",
        RuntimeState::Broken => "broken",
        RuntimeState::Partial => "partial",
        RuntimeState::Unknown => "unknown",
    };
    match &observation.detected_version {
        Some(version) => format!("{state}:{version}"),
        None => state.to_string(),
    }
}

pub(crate) fn observation_matches_baseline(
    current: &RuntimeObservation,
    baseline: &RuntimeObservation,
) -> bool {
    current.component == baseline.component
        && current.state == baseline.state
        && current.detected_version == baseline.detected_version
}

pub(crate) fn verification_value(
    rule: &RuntimeVerificationRule,
    observation: &RuntimeObservation,
) -> Option<String> {
    if observation.state == RuntimeState::Unknown {
        return None;
    }
    match rule {
        RuntimeVerificationRule::InstalledState => {
            if observation.state == RuntimeState::Installed {
                Some("installed".to_string())
            } else {
                Some(runtime_baseline_value(observation))
            }
        }
        RuntimeVerificationRule::ExactDetectedVersion { .. } => {
            if observation.state == RuntimeState::Installed {
                Some(format!(
                    "installed:{}",
                    observation
                        .detected_version
                        .as_deref()
                        .unwrap_or("<missing>")
                ))
            } else {
                Some(runtime_baseline_value(observation))
            }
        }
    }
}

fn require_exact_evidence(
    action: &PlannedAction,
    key: &str,
    expected: &str,
) -> Result<(), RuntimeExecutorError> {
    let matches = action
        .evidence
        .iter()
        .filter(|evidence| evidence.key == key && evidence.value == expected)
        .count();
    if matches != 1 {
        return Err(RuntimeExecutorError::ActionMismatch(format!(
            "expected exactly one {key} evidence item matching {expected}"
        )));
    }
    Ok(())
}

use crate::model::canonical_arch;
#[cfg(any(windows, test))]
use crate::model::{observation_matches_baseline, verification_value, RuntimeInvocation};
use crate::plan::{transaction_plan_for, PreparedRuntimeExecution};
#[cfg(windows)]
use crate::windows::WindowsRuntimeHost;
#[cfg(any(windows, test))]
use crate::RuntimeHost;
use crate::{RuntimeExecutionPlan, RuntimeExecutorError};
#[cfg(any(windows, test))]
use neo_transaction::{ApplyOutcome, ApplyRecord, Observation, ObservedValue, TransactionStage};
use neo_transaction::{TransactionAuthorization, TransactionCheckpoint};
#[cfg(any(windows, test))]
use neo_vault::{VaultSegment, VaultStore};
use serde::{Deserialize, Serialize};
#[cfg(any(windows, test))]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(any(windows, test))]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(any(windows, test))]
static NEXT_RUNTIME_SESSION: AtomicU64 = AtomicU64::new(1);

/// Opaque token required for every public Phase 8 mutation transition.
///
/// There is deliberately no public constructor and the only field is
/// crate-private. Safe outside code can inspect/deserialize sessions but cannot
/// authorize or execute them in Phase 8.
#[derive(Debug)]
pub struct RuntimeExecutorCapability {
    pub(crate) _private: (),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeExecutionSession {
    pub(crate) plan: RuntimeExecutionPlan,
    pub(crate) checkpoint: TransactionCheckpoint,
    #[serde(default)]
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RuntimeExecutionSessionWire {
    plan: RuntimeExecutionPlan,
    checkpoint: TransactionCheckpoint,
    #[serde(default)]
    warnings: Vec<String>,
}

impl<'de> Deserialize<'de> for RuntimeExecutionSession {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = RuntimeExecutionSessionWire::deserialize(deserializer)?;
        let session = Self {
            plan: wire.plan,
            checkpoint: wire.checkpoint,
            warnings: wire.warnings,
        };
        session.validate().map_err(serde::de::Error::custom)?;
        Ok(session)
    }
}

impl RuntimeExecutionSession {
    pub fn new(prepared: PreparedRuntimeExecution) -> Result<Self, RuntimeExecutorError> {
        prepared.plan.validate()?;
        let expected = transaction_plan_for(&prepared.plan)?;
        if expected.fingerprint()? != prepared.transaction_plan.fingerprint()? {
            return Err(RuntimeExecutorError::InvalidPlan(
                "prepared transaction does not match the runtime execution plan".to_string(),
            ));
        }
        let mut checkpoint = TransactionCheckpoint::new(prepared.transaction_plan)?;
        checkpoint.capture_baseline(prepared.baseline.states.clone())?;
        let session = Self {
            plan: prepared.plan,
            checkpoint,
            warnings: Vec::new(),
        };
        session.validate()?;
        Ok(session)
    }

    pub fn from_json_str(input: &str) -> Result<Self, RuntimeExecutorError> {
        Ok(serde_json::from_str(input)?)
    }

    pub fn validate(&self) -> Result<(), RuntimeExecutorError> {
        self.plan.validate()?;
        let expected = transaction_plan_for(&self.plan)?;
        if expected.fingerprint()? != self.checkpoint.plan_fingerprint() {
            return Err(RuntimeExecutorError::InvalidPlan(
                "persisted transaction fingerprint does not match runtime plan".to_string(),
            ));
        }
        if expected.transaction_id() != self.checkpoint.plan().transaction_id()
            || expected.mission_id() != self.checkpoint.plan().mission_id()
        {
            return Err(RuntimeExecutorError::InvalidPlan(
                "persisted transaction identity does not match runtime plan".to_string(),
            ));
        }
        Ok(())
    }

    pub fn authorize_with_capability(
        &mut self,
        _capability: &RuntimeExecutorCapability,
        authorization: TransactionAuthorization,
    ) -> Result<(), RuntimeExecutorError> {
        self.authorize(authorization)
    }

    pub(crate) fn authorize(
        &mut self,
        authorization: TransactionAuthorization,
    ) -> Result<(), RuntimeExecutorError> {
        self.validate()?;
        self.checkpoint.authorize(authorization)?;
        Ok(())
    }

    #[cfg(windows)]
    pub fn apply_windows(
        &mut self,
        _capability: &RuntimeExecutorCapability,
    ) -> Result<(), RuntimeExecutorError> {
        self.apply(&WindowsRuntimeHost)
    }

    #[cfg(windows)]
    pub fn verify_windows(
        &mut self,
        _capability: &RuntimeExecutorCapability,
    ) -> Result<(), RuntimeExecutorError> {
        self.verify_current(&WindowsRuntimeHost)
    }

    #[cfg(windows)]
    pub fn resume_after_reboot_windows(
        &mut self,
        _capability: &RuntimeExecutorCapability,
    ) -> Result<(), RuntimeExecutorError> {
        self.resume_after_reboot(&WindowsRuntimeHost)
    }

    #[cfg(windows)]
    pub fn reprobe_after_block_windows(
        &mut self,
        _capability: &RuntimeExecutorCapability,
    ) -> Result<(), RuntimeExecutorError> {
        self.reprobe_after_block(&WindowsRuntimeHost)
    }

    #[cfg(any(windows, test))]
    pub(crate) fn apply<H: RuntimeHost>(&mut self, host: &H) -> Result<(), RuntimeExecutorError> {
        self.validate()?;
        if self.checkpoint.stage() != TransactionStage::Authorized {
            return Err(RuntimeExecutorError::InvalidPlan(format!(
                "apply requires Authorized stage, found {:?}",
                self.checkpoint.stage()
            )));
        }

        let inventory = host.inventory()?;
        self.validate_preflight(&inventory)?;

        let layout = self.plan.layout()?;
        let store = VaultStore::new(layout);
        let payload = self.plan.payload_path()?;
        store.verify_pack(&payload, &self.plan.package_sha256)?;

        let staging_session = unique_staging_session()?;
        let staged_name = self.plan.staged_filename()?;
        let staged = match store.stage_managed_file(
            &staging_session,
            &payload,
            &staged_name,
            &self.plan.package_sha256,
        ) {
            Ok(path) => path,
            Err(error) => {
                let _ = store.cleanup_staging(&staging_session);
                return Err(error.into());
            }
        };

        let invocation = match self.plan.execution_args().and_then(|arguments| {
            let invocation = RuntimeInvocation {
                installer: self.plan.execution.installer,
                payload: staged,
                expected_sha256: self.plan.package_sha256.clone(),
                arguments,
            };
            invocation.validate()?;
            Ok(invocation)
        }) {
            Ok(invocation) => invocation,
            Err(error) => {
                let _ = store.cleanup_staging(&staging_session);
                return Err(error);
            }
        };

        if let Err(error) = self.checkpoint.begin_apply() {
            let _ = store.cleanup_staging(&staging_session);
            return Err(error.into());
        }
        if let Err(error) = self.checkpoint.assert_action_pending(&self.plan.action.id) {
            let _ = store.cleanup_staging(&staging_session);
            return Err(error.into());
        }

        let process = match host.execute(&invocation) {
            Ok(result) => result,
            Err(error) => {
                self.checkpoint.record_apply_result(ApplyRecord {
                    action_id: self.plan.action.id.clone(),
                    outcome: ApplyOutcome::Failure,
                    detail: format!("runtime process was not created: {error}"),
                    machine_changed: false,
                    reboot_required: false,
                })?;
                self.cleanup_after_execution(&store, &staging_session);
                return Err(error);
            }
        };

        let exit_code = process.exit_code;
        let success = process.started
            && exit_code.is_some_and(|code| self.plan.execution.success_exit_codes.contains(&code));
        let reboot_required = success
            && exit_code.is_some_and(|code| self.plan.execution.reboot_exit_codes.contains(&code));
        let detail = if process.detail.trim().is_empty() {
            match exit_code {
                Some(code) => format!("runtime installer exited with code {code}"),
                None if process.started => {
                    "runtime installer started but its exit status was unavailable".to_string()
                }
                None => "runtime installer did not start".to_string(),
            }
        } else {
            process.detail
        };

        self.checkpoint.record_apply_result(ApplyRecord {
            action_id: self.plan.action.id.clone(),
            outcome: if success {
                ApplyOutcome::Success
            } else {
                ApplyOutcome::Failure
            },
            detail,
            machine_changed: process.started,
            reboot_required,
        })?;
        self.cleanup_after_execution(&store, &staging_session);

        if success && self.checkpoint.stage() == TransactionStage::Verifying {
            self.verify_current(host)?;
        }
        Ok(())
    }

    #[cfg(any(windows, test))]
    pub(crate) fn verify_current<H: RuntimeHost>(
        &mut self,
        host: &H,
    ) -> Result<(), RuntimeExecutorError> {
        self.validate()?;
        if self.checkpoint.stage() != TransactionStage::Verifying {
            return Err(RuntimeExecutorError::InvalidPlan(format!(
                "verification requires Verifying stage, found {:?}",
                self.checkpoint.stage()
            )));
        }
        let inventory = host.inventory()?;
        self.checkpoint
            .verify_postconditions(vec![self.verification_observation(&inventory)])?;
        Ok(())
    }

    #[cfg(any(windows, test))]
    pub(crate) fn resume_after_reboot<H: RuntimeHost>(
        &mut self,
        host: &H,
    ) -> Result<(), RuntimeExecutorError> {
        self.validate()?;
        let inventory = host.inventory()?;
        self.checkpoint
            .resume_after_reboot(vec![self.verification_observation(&inventory)])?;
        if self.checkpoint.stage() == TransactionStage::Verifying {
            self.verify_current(host)?;
        }
        Ok(())
    }

    #[cfg(any(windows, test))]
    pub(crate) fn reprobe_after_block<H: RuntimeHost>(
        &mut self,
        host: &H,
    ) -> Result<(), RuntimeExecutorError> {
        self.validate()?;
        let inventory = host.inventory()?;
        self.checkpoint
            .reprobe_after_block(vec![self.verification_observation(&inventory)])?;
        if self.checkpoint.stage() == TransactionStage::Verifying {
            self.verify_current(host)?;
        }
        Ok(())
    }

    #[cfg(any(windows, test))]
    fn validate_preflight(
        &self,
        inventory: &neo_runtime::RuntimeInventory,
    ) -> Result<(), RuntimeExecutorError> {
        inventory
            .validate()
            .map_err(|error| RuntimeExecutorError::Host(error.to_string()))?;
        if inventory.windows_build != self.plan.windows_build {
            return Err(RuntimeExecutorError::HostDrift(format!(
                "Windows build changed from {} to {}",
                self.plan.windows_build, inventory.windows_build
            )));
        }
        let current_arch = canonical_arch(&inventory.architecture).ok_or_else(|| {
            RuntimeExecutorError::HostDrift(format!(
                "unsupported current architecture {}",
                inventory.architecture
            ))
        })?;
        if current_arch != self.plan.architecture {
            return Err(RuntimeExecutorError::HostDrift(format!(
                "architecture changed from {} to {}",
                self.plan.architecture, current_arch
            )));
        }
        let current = inventory
            .observations
            .iter()
            .find(|observation| observation.component == self.plan.component)
            .ok_or_else(|| {
                RuntimeExecutorError::BaselineDrift(
                    "component observation disappeared before apply".to_string(),
                )
            })?;
        if !observation_matches_baseline(current, &self.plan.baseline) {
            return Err(RuntimeExecutorError::BaselineDrift(format!(
                "expected {:?} {:?}, observed {:?} {:?}",
                self.plan.baseline.state,
                self.plan.baseline.detected_version,
                current.state,
                current.detected_version
            )));
        }
        Ok(())
    }

    #[cfg(any(windows, test))]
    fn verification_observation(&self, inventory: &neo_runtime::RuntimeInventory) -> Observation {
        let target = self.plan.state_target();
        if inventory.windows_build != self.plan.windows_build {
            return Observation {
                target,
                value: ObservedValue::Unavailable(format!(
                    "Windows build drifted from {} to {}",
                    self.plan.windows_build, inventory.windows_build
                )),
            };
        }
        let Some(current_arch) = canonical_arch(&inventory.architecture) else {
            return Observation {
                target,
                value: ObservedValue::Unavailable(format!(
                    "current architecture is unsupported: {}",
                    inventory.architecture
                )),
            };
        };
        if current_arch != self.plan.architecture {
            return Observation {
                target,
                value: ObservedValue::Unavailable(format!(
                    "architecture drifted from {} to {}",
                    self.plan.architecture, current_arch
                )),
            };
        }
        let Some(observation) = inventory
            .observations
            .iter()
            .find(|observation| observation.component == self.plan.component)
        else {
            return Observation {
                target,
                value: ObservedValue::Unavailable(
                    "runtime component observation is unavailable".to_string(),
                ),
            };
        };
        match verification_value(&self.plan.execution.verification, observation) {
            Some(value) => Observation {
                target,
                value: ObservedValue::Present(value),
            },
            None => Observation {
                target,
                value: ObservedValue::Unavailable(
                    "runtime detector returned Unknown; verification cannot be certified"
                        .to_string(),
                ),
            },
        }
    }

    #[cfg(any(windows, test))]
    fn cleanup_after_execution(&mut self, store: &VaultStore, session: &VaultSegment) {
        if let Err(error) = store.cleanup_staging(session) {
            self.warnings.push(format!(
                "marker-owned runtime staging cleanup requires retry: {error}"
            ));
        }
    }
}

#[cfg(any(windows, test))]
fn unique_staging_session() -> Result<VaultSegment, RuntimeExecutorError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| RuntimeExecutorError::InvalidPlan(error.to_string()))?
        .as_nanos();
    let sequence = NEXT_RUNTIME_SESSION.fetch_add(1, Ordering::Relaxed);
    Ok(VaultSegment::new(format!(
        "runtime-exec-{}-{now}-{sequence}",
        std::process::id()
    ))?)
}

#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(path: str, old: str, new: str) -> None:
    p = ROOT / path
    text = p.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one anchor, found {count}: {old[:80]!r}")
    p.write_text(text.replace(old, new, 1), encoding="utf-8")


# Public API boundary: planning/session inspection is public, mutation is gated by
# an opaque capability with no public constructor. Raw host/invocation/Windows
# adapter types remain crate-private.
(ROOT / "crates/neo-runtime-executor/src/lib.rs").write_text(
    '''//! Phase 8 bounded runtime execution for Neo Driver.\n//!\n//! This crate is the internal machine-changing boundary for exact, already\n//! present runtime payloads. It deliberately exposes no network acquisition,\n//! archive extraction, Windows-feature mutation, shell execution, or public CLI\n//! apply path.\n//!\n//! Public callers may inspect validated plans/sessions, but mutation authority\n//! requires an opaque `RuntimeExecutorCapability`. The capability has no public\n//! constructor, while raw host/invocation/process/Windows-host types stay\n//! crate-private. Safe outside code therefore cannot bypass Phase 6 assessment,\n//! Phase 7 vault authority, or Phase 4 transaction authorization.\n\nmod error;\nmod executor;\n#[cfg(any(windows, test))]\nmod host;\nmod model;\nmod plan;\n\n#[cfg(windows)]\nmod windows;\n\npub use error::RuntimeExecutorError;\npub use executor::{RuntimeExecutionSession, RuntimeExecutorCapability};\npub use model::{RuntimeExecutionOperation, RuntimeExecutionPlan};\npub use plan::{prepare_runtime_execution, PreparedRuntimeExecution};\n\n#[cfg(any(windows, test))]\npub(crate) use host::RuntimeHost;\n#[cfg(any(windows, test))]\npub(crate) use model::{RuntimeInvocation, RuntimeProcessResult};\n\n#[cfg(test)]\nmod tests;\n''',
    encoding="utf-8",
)

# Host boundary is never publicly implementable.
replace_once(
    "crates/neo-runtime-executor/src/host.rs",
    "pub trait RuntimeHost {",
    "pub(crate) trait RuntimeHost {",
)

# Catalogue: reject bare empty MSI assignments while preserving case, and allow
# signed i32 representations of Windows' raw 32-bit exit-code bit patterns.
replace_once(
    "crates/neo-catalogue/src/lib.rs",
    """fn is_msi_property_assignment(value: &str) -> bool {\n    let Some((name, _)) = value.split_once('=') else {\n        return false;\n    };\n    let mut chars = name.chars();\n""",
    """fn is_msi_property_assignment(value: &str) -> bool {\n    let Some((name, property_value)) = value.split_once('=') else {\n        return false;\n    };\n    if property_value.is_empty() {\n        return false;\n    }\n    let mut chars = name.chars();\n""",
)
replace_once(
    "crates/neo-catalogue/src/lib.rs",
    """    for value in values {\n        if *value < 0 {\n            return Err(CatalogueError::InvalidRuntimeExitCode(*value));\n        }\n        if !seen.insert(*value) {\n""",
    """    for value in values {\n        // Windows process exit statuses are raw 32-bit values surfaced by\n        // std::process as i32. High-bit HRESULT/Win32 values therefore appear\n        // negative and must retain their bit pattern rather than be rejected.\n        if !seen.insert(*value) {\n""",
)
replace_once(
    "crates/neo-catalogue/src/lib.rs",
    """    #[test]\n    fn driver_artifact_requires_models() {\n""",
    """    #[test]\n    fn msi_bare_empty_property_is_rejected() {\n        let mut spec = runtime_spec();\n        spec.installer = RuntimeInstallerKind::Msi;\n        spec.install_args = vec![\"addlocal=ALL\".to_string()];\n        spec.repair_args = Some(vec![\"REINSTALL=ALL\".to_string()]);\n        assert!(spec.validate().is_ok());\n\n        spec.install_args = vec![\"ADDLOCAL=\".to_string()];\n        assert!(matches!(\n            spec.validate(),\n            Err(CatalogueError::InvalidMsiRuntimeArgument(_))\n        ));\n\n        spec.install_args = vec![\"ADDLOCAL=\\\"\\\"\".to_string()];\n        assert!(spec.validate().is_ok());\n    }\n\n    #[test]\n    fn windows_high_bit_exit_code_bit_pattern_is_accepted() {\n        let mut spec = runtime_spec();\n        let high_bit_code = 0x8007_0666u32 as i32;\n        spec.success_exit_codes = vec![0, high_bit_code];\n        spec.reboot_exit_codes.clear();\n        assert!(spec.validate().is_ok());\n    }\n\n    #[test]\n    fn driver_artifact_requires_models() {\n""",
)

# Error taxonomy/formatting.
replace_once(
    "crates/neo-runtime-executor/src/error.rs",
    """    #[error(\"runtime component {component:?} has no certified Phase 6 action\")]\n    MissingCertifiedAction { component: RuntimeComponent },\n""",
    """    #[error(\"runtime component {component:?} has no certified Phase 6 action\")]\n    MissingCertifiedAction { component: RuntimeComponent },\n    #[error(\"runtime inventory has no observation for {component:?}\")]\n    MissingObservation { component: RuntimeComponent },\n""",
)
replace_once(
    "crates/neo-runtime-executor/src/error.rs",
    """    #[error(\"runtime payload is unavailable: {0}\")]\n    PayloadUnavailable(PathBuf),\n""",
    """    #[error(\"runtime payload is unavailable: {0:?}\")]\n    PayloadUnavailable(PathBuf),\n""",
)

# Plan: distinguish missing observation.
replace_once(
    "crates/neo-runtime-executor/src/plan.rs",
    """        .find(|observation| observation.component == component)\n        .cloned()\n        .ok_or(RuntimeExecutorError::MissingCertifiedAction { component })?;\n""",
    """        .find(|observation| observation.component == component)\n        .cloned()\n        .ok_or(RuntimeExecutorError::MissingObservation { component })?;\n""",
)

# Model: absolute Builder/portable root, strict unique evidence-key authority,
# and crate-private invocation/result types that only exist on Windows/tests.
replace_once(
    "crates/neo-runtime-executor/src/model.rs",
    """        if self.windows_build == 0 {\n            return Err(RuntimeExecutorError::InvalidPlan(\n                \"Windows build must be greater than zero\".to_string(),\n            ));\n        }\n        let Some(architecture) = canonical_arch(&self.architecture) else {\n""",
    """        if self.windows_build == 0 {\n            return Err(RuntimeExecutorError::InvalidPlan(\n                \"Windows build must be greater than zero\".to_string(),\n            ));\n        }\n        if !self.application_root.is_absolute() {\n            return Err(RuntimeExecutorError::InvalidPlan(\n                \"runtime execution plan application_root must be an absolute path\".to_string(),\n            ));\n        }\n        let Some(architecture) = canonical_arch(&self.architecture) else {\n""",
)
replace_once(
    "crates/neo-runtime-executor/src/model.rs",
    """#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct RuntimeInvocation {\n    pub installer: RuntimeInstallerKind,\n    pub payload: PathBuf,\n    pub expected_sha256: Sha256Digest,\n    pub arguments: Vec<String>,\n}\n\nimpl RuntimeInvocation {\n    pub fn validate(&self) -> Result<(), RuntimeExecutorError> {\n""",
    """#[cfg(any(windows, test))]\n#[derive(Debug, Clone, PartialEq, Eq)]\npub(crate) struct RuntimeInvocation {\n    pub(crate) installer: RuntimeInstallerKind,\n    pub(crate) payload: PathBuf,\n    pub(crate) expected_sha256: Sha256Digest,\n    pub(crate) arguments: Vec<String>,\n}\n\n#[cfg(any(windows, test))]\nimpl RuntimeInvocation {\n    pub(crate) fn validate(&self) -> Result<(), RuntimeExecutorError> {\n""",
)
replace_once(
    "crates/neo-runtime-executor/src/model.rs",
    """#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct RuntimeProcessResult {\n    pub started: bool,\n    #[serde(default)]\n    pub exit_code: Option<i32>,\n    pub detail: String,\n}\n\nimpl RuntimeProcessResult {\n    pub fn start_failed(detail: impl Into<String>) -> Self {\n""",
    """#[cfg(any(windows, test))]\n#[derive(Debug, Clone, PartialEq, Eq)]\npub(crate) struct RuntimeProcessResult {\n    pub(crate) started: bool,\n    pub(crate) exit_code: Option<i32>,\n    pub(crate) detail: String,\n}\n\n#[cfg(any(windows, test))]\nimpl RuntimeProcessResult {\n    pub(crate) fn start_failed(detail: impl Into<String>) -> Self {\n""",
)
replace_once(
    "crates/neo-runtime-executor/src/model.rs",
    """    pub fn exited(exit_code: i32, detail: impl Into<String>) -> Self {\n""",
    """    pub(crate) fn exited(exit_code: i32, detail: impl Into<String>) -> Self {\n""",
)
replace_once(
    "crates/neo-runtime-executor/src/model.rs",
    """    pub fn started_without_exit(detail: impl Into<String>) -> Self {\n""",
    """    pub(crate) fn started_without_exit(detail: impl Into<String>) -> Self {\n""",
)
replace_once(
    "crates/neo-runtime-executor/src/model.rs",
    """pub(crate) fn observation_matches_baseline(\n""",
    """#[cfg(any(windows, test))]\npub(crate) fn observation_matches_baseline(\n""",
)
replace_once(
    "crates/neo-runtime-executor/src/model.rs",
    """pub(crate) fn verification_value(\n""",
    """#[cfg(any(windows, test))]\npub(crate) fn verification_value(\n""",
)
replace_once(
    "crates/neo-runtime-executor/src/model.rs",
    """    let matches = action\n        .evidence\n        .iter()\n        .filter(|evidence| evidence.key == key && evidence.value == expected)\n        .count();\n    if matches != 1 {\n        return Err(RuntimeExecutorError::ActionMismatch(format!(\n            \"expected exactly one {key} evidence item matching {expected}\"\n        )));\n    }\n""",
    """    let mut keyed = action.evidence.iter().filter(|evidence| evidence.key == key);\n    let Some(evidence) = keyed.next() else {\n        return Err(RuntimeExecutorError::ActionMismatch(format!(\n            \"expected exactly one {key} evidence item matching {expected}\"\n        )));\n    };\n    if evidence.value != expected || keyed.next().is_some() {\n        return Err(RuntimeExecutorError::ActionMismatch(format!(\n            \"expected exactly one {key} evidence item matching {expected}\"\n        )));\n    }\n""",
)

# Executor: public read-only session shell + opaque capability-gated mutation.
executor = ROOT / "crates/neo-runtime-executor/src/executor.rs"
text = executor.read_text(encoding="utf-8")
text = text.replace(
    """use crate::model::{\n    canonical_arch, observation_matches_baseline, verification_value, RuntimeInvocation,\n};\n""",
    """use crate::model::canonical_arch;\n#[cfg(any(windows, test))]\nuse crate::model::{observation_matches_baseline, verification_value, RuntimeInvocation};\n""",
)
text = text.replace(
    """use crate::{RuntimeExecutionPlan, RuntimeExecutorError, RuntimeHost};\n""",
    """use crate::{RuntimeExecutionPlan, RuntimeExecutorError};\n#[cfg(any(windows, test))]\nuse crate::RuntimeHost;\n#[cfg(windows)]\nuse crate::windows::WindowsRuntimeHost;\n""",
)
text = text.replace(
    """use neo_transaction::{\n    ApplyOutcome, ApplyRecord, Observation, ObservedValue, TransactionAuthorization,\n    TransactionCheckpoint, TransactionStage,\n};\nuse neo_vault::{VaultSegment, VaultStore};\n""",
    """use neo_transaction::{TransactionAuthorization, TransactionCheckpoint};\n#[cfg(any(windows, test))]\nuse neo_transaction::{ApplyOutcome, ApplyRecord, Observation, ObservedValue, TransactionStage};\n#[cfg(any(windows, test))]\nuse neo_vault::{VaultSegment, VaultStore};\n""",
)
text = text.replace(
    """use std::sync::atomic::{AtomicU64, Ordering};\nuse std::time::{SystemTime, UNIX_EPOCH};\n\nstatic NEXT_RUNTIME_SESSION: AtomicU64 = AtomicU64::new(1);\n""",
    """#[cfg(any(windows, test))]\nuse std::sync::atomic::{AtomicU64, Ordering};\n#[cfg(any(windows, test))]\nuse std::time::{SystemTime, UNIX_EPOCH};\n\n#[cfg(any(windows, test))]\nstatic NEXT_RUNTIME_SESSION: AtomicU64 = AtomicU64::new(1);\n\n/// Opaque token required for every public Phase 8 mutation transition.\n///\n/// There is deliberately no public constructor and the only field is\n/// crate-private. Safe outside code can inspect/deserialize sessions but cannot\n/// authorize or execute them in Phase 8.\n#[derive(Debug)]\npub struct RuntimeExecutorCapability {\n    pub(crate) _private: (),\n}\n""",
)
text = text.replace("    pub plan: RuntimeExecutionPlan,", "    pub(crate) plan: RuntimeExecutionPlan,")
text = text.replace("    pub checkpoint: TransactionCheckpoint,", "    pub(crate) checkpoint: TransactionCheckpoint,")
text = text.replace("    pub warnings: Vec<String>,", "    pub(crate) warnings: Vec<String>,")
text = text.replace(
    """    pub fn authorize(\n        &mut self,\n        authorization: TransactionAuthorization,\n    ) -> Result<(), RuntimeExecutorError> {\n""",
    """    pub fn authorize_with_capability(\n        &mut self,\n        _capability: &RuntimeExecutorCapability,\n        authorization: TransactionAuthorization,\n    ) -> Result<(), RuntimeExecutorError> {\n        self.authorize(authorization)\n    }\n\n    pub(crate) fn authorize(\n        &mut self,\n        authorization: TransactionAuthorization,\n    ) -> Result<(), RuntimeExecutorError> {\n""",
)
text = text.replace(
    """    pub fn apply<H: RuntimeHost>(&mut self, host: &H) -> Result<(), RuntimeExecutorError> {\n""",
    """    #[cfg(windows)]\n    pub fn apply_windows(\n        &mut self,\n        _capability: &RuntimeExecutorCapability,\n    ) -> Result<(), RuntimeExecutorError> {\n        self.apply(&WindowsRuntimeHost)\n    }\n\n    #[cfg(windows)]\n    pub fn verify_windows(\n        &mut self,\n        _capability: &RuntimeExecutorCapability,\n    ) -> Result<(), RuntimeExecutorError> {\n        self.verify_current(&WindowsRuntimeHost)\n    }\n\n    #[cfg(windows)]\n    pub fn resume_after_reboot_windows(\n        &mut self,\n        _capability: &RuntimeExecutorCapability,\n    ) -> Result<(), RuntimeExecutorError> {\n        self.resume_after_reboot(&WindowsRuntimeHost)\n    }\n\n    #[cfg(windows)]\n    pub fn reprobe_after_block_windows(\n        &mut self,\n        _capability: &RuntimeExecutorCapability,\n    ) -> Result<(), RuntimeExecutorError> {\n        self.reprobe_after_block(&WindowsRuntimeHost)\n    }\n\n    #[cfg(any(windows, test))]\n    pub(crate) fn apply<H: RuntimeHost>(\n        &mut self,\n        host: &H,\n    ) -> Result<(), RuntimeExecutorError> {\n""",
)
text = text.replace(
    """    pub fn verify_current<H: RuntimeHost>(&mut self, host: &H) -> Result<(), RuntimeExecutorError> {\n""",
    """    #[cfg(any(windows, test))]\n    pub(crate) fn verify_current<H: RuntimeHost>(\n        &mut self,\n        host: &H,\n    ) -> Result<(), RuntimeExecutorError> {\n""",
)
text = text.replace(
    """    pub fn resume_after_reboot<H: RuntimeHost>(\n""",
    """    #[cfg(any(windows, test))]\n    pub(crate) fn resume_after_reboot<H: RuntimeHost>(\n""",
)
text = text.replace(
    """    pub fn reprobe_after_block<H: RuntimeHost>(\n""",
    """    #[cfg(any(windows, test))]\n    pub(crate) fn reprobe_after_block<H: RuntimeHost>(\n""",
)
text = text.replace(
    """    fn validate_preflight(\n""",
    """    #[cfg(any(windows, test))]\n    fn validate_preflight(\n""",
)
text = text.replace(
    """    fn verification_observation(&self, inventory: &neo_runtime::RuntimeInventory) -> Observation {\n""",
    """    #[cfg(any(windows, test))]\n    fn verification_observation(&self, inventory: &neo_runtime::RuntimeInventory) -> Observation {\n""",
)
text = text.replace(
    """    fn cleanup_after_execution(&mut self, store: &VaultStore, session: &VaultSegment) {\n""",
    """    #[cfg(any(windows, test))]\n    fn cleanup_after_execution(&mut self, store: &VaultStore, session: &VaultSegment) {\n""",
)
text = text.replace(
    """fn unique_staging_session() -> Result<VaultSegment, RuntimeExecutorError> {\n""",
    """#[cfg(any(windows, test))]\nfn unique_staging_session() -> Result<VaultSegment, RuntimeExecutorError> {\n""",
)
old = """        let invocation = RuntimeInvocation {\n            installer: self.plan.execution.installer,\n            payload: staged,\n            expected_sha256: self.plan.package_sha256.clone(),\n            arguments: self.plan.execution_args()?,\n        };\n        invocation.validate()?;\n\n        self.checkpoint.begin_apply()?;\n        self.checkpoint\n            .assert_action_pending(&self.plan.action.id)?;\n"""
new = """        let invocation = match self.plan.execution_args().and_then(|arguments| {\n            let invocation = RuntimeInvocation {\n                installer: self.plan.execution.installer,\n                payload: staged,\n                expected_sha256: self.plan.package_sha256.clone(),\n                arguments,\n            };\n            invocation.validate()?;\n            Ok(invocation)\n        }) {\n            Ok(invocation) => invocation,\n            Err(error) => {\n                let _ = store.cleanup_staging(&staging_session);\n                return Err(error);\n            }\n        };\n\n        if let Err(error) = self.checkpoint.begin_apply() {\n            let _ = store.cleanup_staging(&staging_session);\n            return Err(error.into());\n        }\n        if let Err(error) = self.checkpoint.assert_action_pending(&self.plan.action.id) {\n            let _ = store.cleanup_staging(&staging_session);\n            return Err(error.into());\n        }\n"""
if text.count(old) != 1:
    raise SystemExit("executor staging anchor mismatch")
text = text.replace(old, new, 1)
executor.write_text(text, encoding="utf-8")

# Windows: crate-private host and bounded same-session mutex wait.
replace_once(
    "crates/neo-runtime-executor/src/windows.rs",
    "pub struct WindowsRuntimeHost;",
    "pub(crate) struct WindowsRuntimeHost;",
)
replace_once(
    "crates/neo-runtime-executor/src/windows.rs",
    """use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};\n""",
    """use windows::Win32::Foundation::{\n    CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT,\n};\n""",
)
replace_once(
    "crates/neo-runtime-executor/src/windows.rs",
    """    CreateMutexW, ReleaseMutex, WaitForSingleObject, INFINITE,\n""",
    """    CreateMutexW, ReleaseMutex, WaitForSingleObject,\n""",
)
replace_once(
    "crates/neo-runtime-executor/src/windows.rs",
    """const RUNTIME_MUTEX_NAME: &str = \"Local\\\\THETECHGUY.NeoDriver.RuntimeExecutor.v1\";\n""",
    """const RUNTIME_MUTEX_NAME: &str = \"Local\\\\THETECHGUY.NeoDriver.RuntimeExecutor.v1\";\nconst RUNTIME_MUTEX_TIMEOUT_MS: u32 = 300_000;\n""",
)
replace_once(
    "crates/neo-runtime-executor/src/windows.rs",
    """        let wait = unsafe { WaitForSingleObject(handle, INFINITE) };\n        if wait == WAIT_OBJECT_0 || wait == WAIT_ABANDONED {\n            Ok(Self {\n                handle,\n                acquired: true,\n            })\n        } else {\n            unsafe {\n                let _ = CloseHandle(handle);\n            }\n            Err(RuntimeExecutorError::Host(format!(\n                \"runtime executor mutex wait failed with status {wait:?}\"\n            )))\n        }\n""",
    """        let wait = unsafe { WaitForSingleObject(handle, RUNTIME_MUTEX_TIMEOUT_MS) };\n        if wait == WAIT_OBJECT_0 || wait == WAIT_ABANDONED {\n            Ok(Self {\n                handle,\n                acquired: true,\n            })\n        } else {\n            unsafe {\n                let _ = CloseHandle(handle);\n            }\n            if wait == WAIT_TIMEOUT {\n                return Err(RuntimeExecutorError::Host(format!(\n                    \"runtime executor mutex wait timed out after {RUNTIME_MUTEX_TIMEOUT_MS} ms\"\n                )));\n            }\n            Err(RuntimeExecutorError::Host(format!(\n                \"runtime executor mutex wait failed with status {wait:?}\"\n            )))\n        }\n""",
)

# Adversarial Serde regressions for path and duplicate-evidence authority.
tests = ROOT / "crates/neo-runtime-executor/src/tests.rs"
text = tests.read_text(encoding="utf-8")
append = r'''

#[test]
fn persisted_relative_application_root_is_rejected() {
    let fixture = fixture(spec());
    let prepared = prepared(&fixture, &fixture.missing);
    let mut value = serde_json::to_value(&prepared.plan).unwrap();
    value["application_root"] = serde_json::json!("relative-neo-root");
    assert!(serde_json::from_value::<RuntimeExecutionPlan>(value).is_err());
}

#[test]
fn persisted_duplicate_package_evidence_key_is_rejected() {
    let fixture = fixture(spec());
    let prepared = prepared(&fixture, &fixture.missing);
    let mut value = serde_json::to_value(&prepared.plan).unwrap();
    value["action"]["evidence"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "key": "package_id",
            "value": "neo.attacker.package",
            "source": "phase8-tamper"
        }));
    assert!(serde_json::from_value::<RuntimeExecutionPlan>(value).is_err());
}
'''
if "fn persisted_relative_application_root_is_rejected()" in text:
    raise SystemExit("tests already patched")
tests.write_text(text.rstrip() + append + "\n", encoding="utf-8")

# Static gate: prove opaque capability + crate-private raw adapter boundary and
# the newly reviewed safety conditions.
static = ROOT / "tools/phase8_static_review.py"
text = static.read_text(encoding="utf-8")
start = text.index("    closed_library_execution_surface = (")
end = text.index("\n\n    return [", start)
replacement = '''    closed_library_execution_surface = (\n        "pub use executor::{RuntimeExecutionSession, RuntimeExecutorCapability};" in LIB\n        and "pub(crate) trait RuntimeHost" in EXECUTOR\n        and "pub(crate) struct RuntimeInvocation" in EXECUTOR\n        and "pub(crate) struct RuntimeProcessResult" in EXECUTOR\n        and "pub(crate) struct WindowsRuntimeHost" in EXECUTOR\n        and "pub use host::RuntimeHost;" not in LIB\n        and "pub use windows::WindowsRuntimeHost;" not in LIB\n        and "pub(crate) _private: ()" in EXECUTOR\n        and "impl RuntimeExecutorCapability" not in EXECUTOR\n        and "authorize_with_capability" in EXECUTOR\n        and "apply_windows" in EXECUTOR\n    )'''
text = text[:start] + replacement + text[end:]
text = text.replace(
    'Lane(10, "preflight-drift", contains_all(EXECUTOR, ["validate_preflight", "windows_build", "canonical_arch", "observation_matches_baseline", "BaselineDrift", "HostDrift"]) and "preflight_drift_blocks_before_applying" in TESTS, "build/architecture/component baseline drift blocks before mutation"),',
    'Lane(10, "preflight-drift", contains_all(EXECUTOR, ["validate_preflight", "windows_build", "canonical_arch", "observation_matches_baseline", "BaselineDrift", "HostDrift", "application_root.is_absolute"]) and "preflight_drift_blocks_before_applying" in TESTS and "persisted_relative_application_root_is_rejected" in TESTS, "absolute application root plus build/architecture/component baseline are re-proven before mutation"),'
)
text = text.replace(
    'Lane(13, "msi-operation-guard", contains_all(CATALOGUE, ["InvalidMsiRuntimeArgument", "is_msi_property_assignment", "RuntimeInstallerKind::Msi"]) and "msi_arguments_cannot_replace_neos_fixed_operation_switches" in CATALOGUE, "MSI custom arguments cannot replace Neo\'s fixed install operation"),',
    'Lane(13, "msi-operation-guard", contains_all(CATALOGUE, ["InvalidMsiRuntimeArgument", "is_msi_property_assignment", "property_value.is_empty", "RuntimeInstallerKind::Msi"]) and "msi_arguments_cannot_replace_neos_fixed_operation_switches" in CATALOGUE and "msi_bare_empty_property_is_rejected" in CATALOGUE, "MSI custom arguments cannot replace Neo\'s fixed install operation or smuggle a bare empty property assignment"),'
)
text = text.replace(
    'Lane(14, "session-process-serialization", contains_all(EXECUTOR, ["RUNTIME_MUTEX_NAME", "Local\\\\\\\\THETECHGUY.NeoDriver.RuntimeExecutor.v1", "CreateMutexW", "WaitForSingleObject", "ReleaseMutex"]) and "runtime-executor.lock" not in EXECUTOR and contains_all(REVIEW, ["within one Windows session", "does not claim system-wide cross-session serialization"]), "Windows runtime execution uses one Local named mutex for same-session cross-process serialization and makes no system-wide claim"),',
    'Lane(14, "session-process-serialization", contains_all(EXECUTOR, ["RUNTIME_MUTEX_NAME", "RUNTIME_MUTEX_TIMEOUT_MS", "WAIT_TIMEOUT", "Local\\\\\\\\THETECHGUY.NeoDriver.RuntimeExecutor.v1", "CreateMutexW", "WaitForSingleObject", "ReleaseMutex"]) and "INFINITE" not in EXECUTOR and "runtime-executor.lock" not in EXECUTOR and contains_all(REVIEW, ["within one Windows session", "does not claim system-wide cross-session serialization"]), "Windows runtime execution uses a bounded Local named-mutex wait for same-session cross-process serialization and makes no system-wide claim"),'
)
text = text.replace(
    'Lane(16, "typed-exit-reboot", contains_all(CATALOGUE, ["success_exit_codes", "reboot_exit_codes", "RuntimeRebootCodeNotSuccessful"]) and contains_all(EXECUTOR, ["success_exit_codes.contains", "reboot_exit_codes.contains"]), "success and reboot exit codes are explicit typed catalogue authority"),',
    'Lane(16, "typed-exit-reboot", contains_all(CATALOGUE, ["success_exit_codes", "reboot_exit_codes", "RuntimeRebootCodeNotSuccessful", "windows_high_bit_exit_code_bit_pattern_is_accepted"]) and contains_all(EXECUTOR, ["success_exit_codes.contains", "reboot_exit_codes.contains"]), "success/reboot exit codes preserve Windows 32-bit status bit patterns and remain explicit typed catalogue authority"),'
)
text = text.replace(
    'and "irreversible_acknowledgements" in TESTS and "No test or green CI run is allowed to imply that a real runtime installer was executed on CI." in REVIEW, "Phase 8 keeps session/host/invocation/Windows execution authority crate-private, requires irreversible acknowledgement internally, and exposes only read-only planning/validation publicly"),',
    'and "irreversible_acknowledgements" in TESTS and "persisted_duplicate_package_evidence_key_is_rejected" in TESTS and "No test or green CI run is allowed to imply that a real runtime installer was executed on CI." in REVIEW, "Phase 8 exposes mutation only through a non-constructible opaque capability; raw host/invocation/Windows adapters remain crate-private and evidence keys are unique"),'
)
static.write_text(text, encoding="utf-8")

print("Phase 8 review corrections and capability boundary applied")

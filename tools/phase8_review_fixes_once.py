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

# Model: absolute Builder/portable root and strict unique evidence-key authority.
replace_once(
    "crates/neo-runtime-executor/src/model.rs",
    """        if self.windows_build == 0 {\n            return Err(RuntimeExecutorError::InvalidPlan(\n                \"Windows build must be greater than zero\".to_string(),\n            ));\n        }\n        let Some(architecture) = canonical_arch(&self.architecture) else {\n""",
    """        if self.windows_build == 0 {\n            return Err(RuntimeExecutorError::InvalidPlan(\n                \"Windows build must be greater than zero\".to_string(),\n            ));\n        }\n        if !self.application_root.is_absolute() {\n            return Err(RuntimeExecutorError::InvalidPlan(\n                \"runtime execution plan application_root must be an absolute path\".to_string(),\n            ));\n        }\n        let Some(architecture) = canonical_arch(&self.architecture) else {\n""",
)
replace_once(
    "crates/neo-runtime-executor/src/model.rs",
    """    let matches = action\n        .evidence\n        .iter()\n        .filter(|evidence| evidence.key == key && evidence.value == expected)\n        .count();\n    if matches != 1 {\n        return Err(RuntimeExecutorError::ActionMismatch(format!(\n            \"expected exactly one {key} evidence item matching {expected}\"\n        )));\n    }\n""",
    """    let mut keyed = action.evidence.iter().filter(|evidence| evidence.key == key);\n    let Some(evidence) = keyed.next() else {\n        return Err(RuntimeExecutorError::ActionMismatch(format!(\n            \"expected exactly one {key} evidence item matching {expected}\"\n        )));\n    };\n    if evidence.value != expected || keyed.next().is_some() {\n        return Err(RuntimeExecutorError::ActionMismatch(format!(\n            \"expected exactly one {key} evidence item matching {expected}\"\n        )));\n    }\n""",
)

# Executor: every post-staging preparation/transition failure cleans marker-owned staging.
replace_once(
    "crates/neo-runtime-executor/src/executor.rs",
    """        let invocation = RuntimeInvocation {\n            installer: self.plan.execution.installer,\n            payload: staged,\n            expected_sha256: self.plan.package_sha256.clone(),\n            arguments: self.plan.execution_args()?,\n        };\n        invocation.validate()?;\n\n        self.checkpoint.begin_apply()?;\n        self.checkpoint\n            .assert_action_pending(&self.plan.action.id)?;\n""",
    """        let invocation = match self.plan.execution_args().and_then(|arguments| {\n            let invocation = RuntimeInvocation {\n                installer: self.plan.execution.installer,\n                payload: staged,\n                expected_sha256: self.plan.package_sha256.clone(),\n                arguments,\n            };\n            invocation.validate()?;\n            Ok(invocation)\n        }) {\n            Ok(invocation) => invocation,\n            Err(error) => {\n                let _ = store.cleanup_staging(&staging_session);\n                return Err(error);\n            }\n        };\n\n        if let Err(error) = self.checkpoint.begin_apply() {\n            let _ = store.cleanup_staging(&staging_session);\n            return Err(error.into());\n        }\n        if let Err(error) = self.checkpoint.assert_action_pending(&self.plan.action.id) {\n            let _ = store.cleanup_staging(&staging_session);\n            return Err(error.into());\n        }\n""",
)

# Windows: bound same-session mutex wait instead of blocking forever.
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

print("Phase 8 review corrections applied")

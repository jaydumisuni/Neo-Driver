#!/usr/bin/env python3
from pathlib import Path


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise SystemExit(f"anchor mismatch in {path}: {old[:140]!r}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def replace_all(path: Path, old: str, new: str, minimum: int = 1) -> None:
    text = path.read_text(encoding="utf-8")
    if text.count(old) < minimum:
        raise SystemExit(f"missing expected text in {path}: {old[:140]!r}")
    path.write_text(text.replace(old, new), encoding="utf-8")


# Outcome-aware legacy ApplyRecord deserialization.
plan = Path("crates/neo-transaction/src/plan.rs")
replace_once(
    plan,
    '''#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyRecord {
    pub action_id: String,
    pub outcome: ApplyOutcome,
    pub detail: String,
    #[serde(default = "default_machine_changed")]
    pub machine_changed: bool,
    #[serde(default)]
    pub reboot_required: bool,
}

fn default_machine_changed() -> bool {
    true
}
''',
    '''#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "ApplyRecordWire")]
pub struct ApplyRecord {
    pub action_id: String,
    pub outcome: ApplyOutcome,
    pub detail: String,
    pub machine_changed: bool,
    #[serde(default)]
    pub reboot_required: bool,
}

#[derive(Debug, Deserialize)]
struct ApplyRecordWire {
    action_id: String,
    outcome: ApplyOutcome,
    detail: String,
    #[serde(default)]
    machine_changed: Option<bool>,
    #[serde(default)]
    reboot_required: bool,
}

impl From<ApplyRecordWire> for ApplyRecord {
    fn from(value: ApplyRecordWire) -> Self {
        let machine_changed = value
            .machine_changed
            .unwrap_or(value.outcome == ApplyOutcome::Success);
        Self {
            action_id: value.action_id,
            outcome: value.outcome,
            detail: value.detail,
            machine_changed,
            reboot_required: value.reboot_required,
        }
    }
}
''',
)

# Reboot checkpoint kind derives from enclosing transaction stage, never JSON resume_stage.
checkpoint = Path("crates/neo-transaction/src/checkpoint.rs")
replace_once(
    checkpoint,
    '''        let expected = match self.resume_stage {
            TransactionStage::Verifying => Self::for_apply_checkpoint(checkpoint),
            TransactionStage::RolledBack => Self::for_rollback_checkpoint(checkpoint),
            _ => return Err(TransactionError::RebootCheckpointMismatch),
        };
''',
    '''        let expected = match checkpoint.stage {
            TransactionStage::AwaitingReboot
            | TransactionStage::Verifying
            | TransactionStage::Complete
            | TransactionStage::Blocked => Self::for_apply_checkpoint(checkpoint),
            TransactionStage::AwaitingRollbackReboot | TransactionStage::RolledBack => {
                Self::for_rollback_checkpoint(checkpoint)
            }
            _ => return Err(TransactionError::RebootCheckpointMismatch),
        };
''',
)

# Transaction compatibility/tamper regressions.
ttests = Path("crates/neo-transaction/src/tests.rs")
marker = '''#[test]
fn rejected_action_cannot_enter_transaction() {
'''
text = ttests.read_text(encoding="utf-8")
if text.count(marker) != 1:
    raise SystemExit("transaction compatibility insertion anchor mismatch")
added = '''#[test]
fn legacy_apply_record_preserves_outcome_based_change_semantics() {
    let success: ApplyRecord = serde_json::from_str(
        r#"{"action_id":"a","outcome":"success","detail":"legacy success","reboot_required":false}"#,
    )
    .unwrap();
    let failure: ApplyRecord = serde_json::from_str(
        r#"{"action_id":"a","outcome":"failure","detail":"legacy failure","reboot_required":false}"#,
    )
    .unwrap();
    assert!(success.machine_changed);
    assert!(!failure.machine_changed);
}

'''
ttests.write_text(text.replace(marker, added + marker, 1), encoding="utf-8")
marker = '''#[test]
fn failed_post_reboot_probe_blocks_continuation() {
'''
text = ttests.read_text(encoding="utf-8")
if text.count(marker) != 1:
    raise SystemExit("reboot tamper insertion anchor mismatch")
added = '''#[test]
fn persisted_apply_reboot_checkpoint_cannot_be_rebound_as_rollback() {
    let mut action = transaction_action();
    action.action.reboot = RebootRequirement::Required;
    let plan = TransactionPlan::new("TX", 1, "MISSION", vec![action]).unwrap();
    let auth = authorization(&plan);
    let mut checkpoint = TransactionCheckpoint::new(plan).unwrap();
    checkpoint
        .capture_baseline(baseline(CapturedValue::Present("0".to_string())))
        .unwrap();
    checkpoint.authorize(auth).unwrap();
    checkpoint.begin_apply().unwrap();
    checkpoint
        .record_apply_result(ApplyRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "future executor reported success".to_string(),
            machine_changed: true,
            reboot_required: false,
        })
        .unwrap();
    let mut value = serde_json::to_value(&checkpoint).unwrap();
    value["reboot_checkpoint"]["resume_stage"] = serde_json::json!("rolled_back");
    value["reboot_checkpoint"]["expected_post_reboot"] =
        serde_json::to_value(vec![rollback_predicate()]).unwrap();
    assert!(serde_json::from_value::<TransactionCheckpoint>(value).is_err());
}

'''
ttests.write_text(text.replace(marker, added + marker, 1), encoding="utf-8")

# Directly deserialized plans cannot use lexical parent traversal.
model = Path("crates/neo-driverstore/src/model.rs")
replace_once(model, 'use std::path::{Path, PathBuf};\n', 'use std::path::{Component, Path, PathBuf};\n')
replace_once(
    model,
    '''        if !self.source_inf.starts_with(&self.package_root) {
            return Err(DriverStoreError::UnsafeInfPath);
        }
''',
    '''        if !self.source_inf.starts_with(&self.package_root)
            || self
                .package_root
                .components()
                .chain(self.source_inf.components())
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(DriverStoreError::UnsafeInfPath);
        }
''',
)

# Host build is authoritative.
host = Path("crates/neo-driverstore/src/host.rs")
replace_once(
    host,
    '''pub trait DriverHost {
    /// Read-only inventory of present devices and their active bindings.
''',
    '''pub trait DriverHost {
    /// Read the actual host Windows build from trusted system state.
    fn windows_build(&self) -> Result<u32, DriverStoreError>;

    /// Read-only inventory of present devices and their active bindings.
''',
)
error = Path("crates/neo-driverstore/src/error.rs")
replace_once(
    error,
    '''    #[error("driver blast radius changed after authority; apply blocked")]
    ImpactDrift,
''',
    '''    #[error("driver blast radius changed after authority; apply blocked")]
    ImpactDrift,
    #[error("requested Windows build {requested} does not match actual host build {actual}")]
    WindowsBuildMismatch { requested: u32, actual: u32 },
''',
)
planner = Path("crates/neo-driverstore/src/plan.rs")
replace_once(
    planner,
    '''    catalogue.validate()?;
''',
    '''    catalogue.validate()?;
    let actual_windows_build = host.windows_build()?;
    if actual_windows_build != windows_build {
        return Err(DriverStoreError::WindowsBuildMismatch {
            requested: windows_build,
            actual: actual_windows_build,
        });
    }
''',
)

# Stage failures remain conservatively changed until exact identity is recovered/proven.
executor = Path("crates/neo-driverstore/src/executor.rs")
replace_once(
    executor,
    '''        let mut operational_error: Option<String> = None;
        if self.target_package.is_none() {
''',
    '''        let mut operational_error: Option<String> = None;
        let mut staging_attempted = false;
        if self.target_package.is_none() {
            staging_attempted = true;
''',
)
replace_once(
    executor,
    '''                    if let Ok(Some(package)) = host.find_equivalent_package(
                        &self.driver_plan.source_inf,
                        std::slice::from_ref(&self.driver_plan.expected_signature.catalog_file),
                    ) {
                        self.target_package = Some(package);
                    }
''',
    '''                    match host.find_equivalent_package(
                        &self.driver_plan.source_inf,
                        std::slice::from_ref(&self.driver_plan.expected_signature.catalog_file),
                    ) {
                        Ok(Some(package)) => self.target_package = Some(package),
                        Ok(None) => {}
                        Err(recovery_error) => {
                            operational_error = Some(format!(
                                "driver staging failed: {error}; package identity recovery failed: {recovery_error}"
                            ));
                        }
                    }
''',
)
replace_once(
    executor,
    '''        if operational_error.is_none() {
            if let Some(package) = self.target_package.as_ref() {
                if let Err(error) = self.validate_target_package(host, package) {
                    operational_error =
                        Some(format!("staged package verification failed: {error}"));
                }
            } else {
                operational_error =
                    Some("staging produced no recoverable package identity".to_string());
            }
        }
''',
    '''        if let Some(package) = self.target_package.as_ref() {
            if let Err(error) = self.validate_target_package(host, package) {
                operational_error = Some(match operational_error {
                    Some(existing) => format!("{existing}; staged package verification failed: {error}"),
                    None => format!("staged package verification failed: {error}"),
                });
            }
        } else if operational_error.is_none() {
            operational_error =
                Some("staging produced no recoverable package identity".to_string());
        }
''',
)
replace_once(executor, 'self.store_matches_baseline(host)', 'self.store_matches_baseline(host, staging_attempted)')
replace_once(
    executor,
    '''    fn store_matches_baseline<H: DriverHost>(&self, host: &H) -> Result<bool, DriverStoreError> {
''',
    '''    fn store_matches_baseline<H: DriverHost>(
        &self,
        host: &H,
        staging_attempted: bool,
    ) -> Result<bool, DriverStoreError> {
''',
)
replace_once(executor, '                None => Ok(true),\n', '                None => Ok(!staging_attempted),\n')
replace_once(
    executor,
    '''        if sha256_file(&self.driver_plan.source_inf)? != self.driver_plan.source_inf_sha256 {
''',
    '''        let actual_windows_build = host.windows_build()?;
        if actual_windows_build != self.driver_plan.windows_build {
            return Err(DriverStoreError::WindowsBuildMismatch {
                requested: self.driver_plan.windows_build,
                actual: actual_windows_build,
            });
        }
        if sha256_file(&self.driver_plan.source_inf)? != self.driver_plan.source_inf_sha256 {
''',
)

# Trusted Windows directory, canonical DEVPROPKEY, trusted registry build.
windows = Path("crates/neo-driverstore/src/windows.rs")
replace_once(
    windows,
    'use windows::Win32::Devices::Properties::{DEVPKEY_Device_DriverInfPath, DEVPROPTYPE};\nuse windows::Win32::Foundation::ERROR_NO_MORE_ITEMS;\n',
    '''use windows::Win32::Devices::Properties::{DEVPKEY_Device_DriverInfPath, DEVPROPKEY, DEVPROPTYPE};
use windows::Win32::Foundation::ERROR_NO_MORE_ITEMS;
use windows::Win32::System::Registry::{
    RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RRF_ZEROONFAILURE,
};
use windows::Win32::System::SystemInformation::GetWindowsDirectoryW;
''',
)
replace_once(
    windows,
    '''impl DriverHost for WindowsDriverHost {
    fn inventory(&self) -> Result<DriverInventory, DriverStoreError> {
''',
    '''impl DriverHost for WindowsDriverHost {
    fn windows_build(&self) -> Result<u32, DriverStoreError> {
        windows_build_number()
    }

    fn inventory(&self) -> Result<DriverInventory, DriverStoreError> {
''',
)
replace_once(windows, '    property: &windows::Win32::Foundation::DEVPROPKEY,\n', '    property: &DEVPROPKEY,\n')
replace_once(
    windows,
    '''fn windows_inf_dir() -> Result<PathBuf, DriverStoreError> {
    let root = std::env::var_os("WINDIR")
        .ok_or_else(|| DriverStoreError::Windows("WINDIR is not defined".to_string()))?;
    Ok(PathBuf::from(root).join("INF"))
}
''',
    '''fn windows_inf_dir() -> Result<PathBuf, DriverStoreError> {
    let mut buffer = vec![0u16; 260];
    loop {
        let length = unsafe { GetWindowsDirectoryW(Some(&mut buffer)) } as usize;
        if length == 0 {
            return Err(last_error("GetWindowsDirectoryW"));
        }
        if length < buffer.len() {
            return Ok(PathBuf::from(String::from_utf16_lossy(&buffer[..length])).join("INF"));
        }
        buffer.resize(length + 1, 0);
    }
}

fn windows_build_number() -> Result<u32, DriverStoreError> {
    let subkey = wide_string(r"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion");
    let value_name = wide_string("CurrentBuildNumber");
    let flags = RRF_RT_REG_SZ | RRF_ZEROONFAILURE;
    let mut bytes = 0u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
            flags,
            None,
            None,
            Some(&mut bytes),
        )
    };
    if status.0 != 0 || bytes < 2 {
        return Err(DriverStoreError::Windows(format!(
            "RegGetValueW CurrentBuildNumber sizing failed: {}",
            status.0
        )));
    }
    let mut buffer = vec![0u16; (bytes as usize).div_ceil(2)];
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
            flags,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    if status.0 != 0 {
        return Err(DriverStoreError::Windows(format!(
            "RegGetValueW CurrentBuildNumber failed: {}",
            status.0
        )));
    }
    utf16_array(&buffer)
        .trim()
        .parse::<u32>()
        .map_err(|error| DriverStoreError::Windows(format!("invalid CurrentBuildNumber: {error}")))
}
''',
)

# Fake host and review regressions.
dtests = Path("crates/neo-driverstore/src/tests.rs")
replace_once(
    dtests,
    '''    fail_inventory_call: Option<usize>,
}
''',
    '''    fail_inventory_call: Option<usize>,
    windows_build: u32,
    stage_error_after_insert: bool,
    hide_equivalent: bool,
}
''',
)
replace_once(
    dtests,
    '''                fail_inventory_call: None,
            }),
''',
    '''                fail_inventory_call: None,
                windows_build: 26100,
                stage_error_after_insert: false,
                hide_equivalent: false,
            }),
''',
)
replace_once(
    dtests,
    '''impl DriverHost for FakeHost {
    fn inventory(&self) -> Result<DriverInventory, DriverStoreError> {
''',
    '''impl DriverHost for FakeHost {
    fn windows_build(&self) -> Result<u32, DriverStoreError> {
        Ok(self.state.borrow().windows_build)
    }

    fn inventory(&self) -> Result<DriverInventory, DriverStoreError> {
''',
)
replace_once(
    dtests,
    '''        Ok(self.state.borrow().packages.get("oem42.inf").cloned())
''',
    '''        let state = self.state.borrow();
        if state.hide_equivalent {
            Ok(None)
        } else {
            Ok(state.packages.get("oem42.inf").cloned())
        }
''',
)
replace_once(
    dtests,
    '''        state
            .packages
            .insert(package.published_inf.to_ascii_lowercase(), package.clone());
        Ok(package)
''',
    '''        state
            .packages
            .insert(package.published_inf.to_ascii_lowercase(), package.clone());
        if state.stage_error_after_insert {
            Err(DriverStoreError::Windows(
                "synthetic staging failure after Driver Store mutation".to_string(),
            ))
        } else {
            Ok(package)
        }
''',
)
marker = '''#[test]
fn source_byte_drift_blocks_before_staging() {
'''
text = dtests.read_text(encoding="utf-8")
if text.count(marker) != 1:
    raise SystemExit("driverstore review regression insertion anchor mismatch")
added = '''#[test]
fn planner_rejects_caller_build_that_does_not_match_host() {
    let fixture = Fixture::new(None);
    fixture.host.configure(|state| state.windows_build = 26200);
    let error = prepare_driver_install(
        &fixture.host,
        &fixture_catalogue(),
        &DriverInstallRequest {
            package_root: fixture.root.clone(),
            package_id: "neo.fixture.driver".to_string(),
            inf_path: "drivers/fixture.inf".to_string(),
            architecture: "x64".to_string(),
            windows_build: 26100,
            action_id: "install.fixture.driver".to_string(),
            mission_id: "mission.fixture".to_string(),
        },
    )
    .unwrap_err();
    assert!(matches!(error, DriverStoreError::WindowsBuildMismatch { .. }));
}

#[test]
fn deserialized_driver_plan_rejects_parent_traversal() {
    let fixture = Fixture::new(None);
    let prepared = fixture.prepare();
    let mut value = serde_json::to_value(&prepared.driver_plan).unwrap();
    value["source_inf"] = serde_json::Value::String(
        fixture
            .root
            .join("drivers")
            .join("..")
            .join("evil.inf")
            .to_string_lossy()
            .into_owned(),
    );
    assert!(serde_json::from_value::<DriverInstallPlan>(value).is_err());
}

#[test]
fn preflight_rejects_host_build_drift_after_authority() {
    let fixture = Fixture::new(None);
    let mut session = fixture.session();
    fixture.host.configure(|state| state.windows_build = 26200);
    let error = session.apply(&fixture.host).unwrap_err();
    assert!(matches!(error, DriverStoreError::WindowsBuildMismatch { .. }));
    assert_eq!(session.transaction().stage(), TransactionStage::Authorized);
}

#[test]
fn staging_failure_with_recovered_identity_routes_rollback() {
    let fixture = Fixture::new(None);
    let mut session = fixture.session();
    fixture.host.configure(|state| state.stage_error_after_insert = true);
    session.apply(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::RollingBack);
    assert!(session.target_package().is_some());
    session.rollback(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::RolledBack);
}

#[test]
fn staging_failure_without_recoverable_identity_never_claims_no_change() {
    let fixture = Fixture::new(None);
    let mut session = fixture.session();
    fixture.host.configure(|state| {
        state.stage_error_after_insert = true;
        state.hide_equivalent = true;
    });
    session.apply(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::RollingBack);
    session.rollback(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::Failed);
}

'''
dtests.write_text(text.replace(marker, added + marker, 1), encoding="utf-8")

# Phase 5 static scanner never uses tests.rs to satisfy production contract lanes.
review = Path("tools/phase5_static_review.py")
replace_once(
    review,
    '''DRIVERSTORE = "\\n".join(
    path.read_text(encoding="utf-8") for path in sorted(DRIVERSTORE_DIR.rglob("*.rs"))
)
''',
    '''PRODUCTION = "\\n".join(
    path.read_text(encoding="utf-8")
    for path in sorted(DRIVERSTORE_DIR.rglob("*.rs"))
    if path.name != "tests.rs"
)
''',
)
replace_all(review, "DRIVERSTORE,", "PRODUCTION,")
replace_all(review, " in DRIVERSTORE", " in PRODUCTION")
replace_once(
    review,
    '''        "direct_session_deserialization_cannot_rebind_transaction",
''',
    '''        "direct_session_deserialization_cannot_rebind_transaction",
        "deserialized_driver_plan_rejects_parent_traversal",
        "planner_rejects_caller_build_that_does_not_match_host",
        "preflight_rejects_host_build_drift_after_authority",
        "staging_failure_with_recovered_identity_routes_rollback",
        "staging_failure_without_recoverable_identity_never_claims_no_change",
''',
)
replace_once(
    review,
    '''                    "source_inf_sha256",
                    "verify_inf_signature",
''',
    '''                    "source_inf_sha256",
                    "windows_build",
                    "WindowsBuildMismatch",
                    "verify_inf_signature",
''',
)
replace_once(
    review,
    '''                    "post-mutation Driver Store probe failed",
''',
    '''                    "post-mutation Driver Store probe failed",
                    "staging_attempted",
''',
)

# Registry feature for trusted host build query.
cargo = Path("Cargo.toml")
replace_once(
    cargo,
    '''    "Win32_System_Diagnostics_Debug",
    "Win32_System_SystemInformation",
''',
    '''    "Win32_System_Diagnostics_Debug",
    "Win32_System_Registry",
    "Win32_System_SystemInformation",
''',
)

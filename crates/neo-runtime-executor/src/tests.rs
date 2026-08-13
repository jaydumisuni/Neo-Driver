use super::*;
use neo_catalogue::{
    Catalogue, PackageKind, PackageManifest, Provenance, RebootRequirement, RedistributionPolicy,
    RuntimeExecutionSpec, RuntimeInstallerKind, RuntimeVerificationRule, SecurityRequirements,
    WindowsApplicability,
};
use neo_runtime::{
    RuntimeComponent, RuntimeInventory, RuntimeObservation, RuntimePackageBinding, RuntimePolicy,
    RuntimeProfile, RuntimeState,
};
use neo_transaction::{
    ActionAcknowledgement, TransactionAuthorization, TransactionStage,
};
use neo_vault::{sha256_file, PackClass, VaultLayout, VaultMode, VaultSegment, VaultStore};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(1);

struct FakeHost {
    inventories: RefCell<VecDeque<Result<RuntimeInventory, String>>>,
    process: RefCell<Option<RuntimeProcessResult>>,
}

impl FakeHost {
    fn new(
        inventories: impl IntoIterator<Item = Result<RuntimeInventory, String>>,
        process: RuntimeProcessResult,
    ) -> Self {
        Self {
            inventories: RefCell::new(inventories.into_iter().collect()),
            process: RefCell::new(Some(process)),
        }
    }

    fn probe_only(inventories: impl IntoIterator<Item = Result<RuntimeInventory, String>>) -> Self {
        Self::new(
            inventories,
            RuntimeProcessResult::start_failed("process not expected"),
        )
    }
}

impl RuntimeHost for FakeHost {
    fn inventory(&self) -> Result<RuntimeInventory, RuntimeExecutorError> {
        self.inventories
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| Err("no fake inventory remaining".to_string()))
            .map_err(RuntimeExecutorError::Host)
    }

    fn execute(
        &self,
        invocation: &RuntimeInvocation,
    ) -> Result<RuntimeProcessResult, RuntimeExecutorError> {
        invocation.validate()?;
        self.process
            .borrow_mut()
            .take()
            .ok_or_else(|| RuntimeExecutorError::Host("process already consumed".to_string()))
    }
}

struct Fixture {
    root: PathBuf,
    catalogue: Catalogue,
    policy: RuntimePolicy,
    missing: RuntimeInventory,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn fixture(spec: RuntimeExecutionSpec) -> Fixture {
    let sequence = NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "neo-phase8-{}-{sequence}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("fixture-redist.exe");
    fs::write(&source, b"neo-runtime-executor-fixture-v1").unwrap();
    let digest = sha256_file(&source).unwrap();

    let package = PackageManifest {
        package_id: "neo.fixture.vcredist.x86".to_string(),
        name: "Fixture VC++ Runtime".to_string(),
        vendor: "Neo Fixture Vendor".to_string(),
        version: "1.2.3".to_string(),
        kind: PackageKind::Runtime,
        provenance: Provenance {
            source_name: "phase8 fixture".to_string(),
            source_url: None,
            sha256: digest.to_string(),
            redistribution: RedistributionPolicy::Allowed,
        },
        windows: WindowsApplicability {
            architectures: vec!["x64".to_string()],
            minimum_build: Some(19041),
            maximum_build: None,
        },
        driver_artifacts: vec![],
        runtime_execution: Some(spec),
        dependencies: vec![],
        conflicts: vec![],
        security: SecurityRequirements::default(),
        reboot: RebootRequirement::None,
    };
    let catalogue = Catalogue {
        packages: vec![package],
    };
    catalogue.validate().unwrap();
    let policy = RuntimePolicy {
        bindings: vec![RuntimePackageBinding {
            component: RuntimeComponent::VcRedist2015PlusX86,
            package_id: "neo.fixture.vcredist.x86".to_string(),
        }],
    };
    policy.validate(&catalogue).unwrap();

    let layout = VaultLayout::new(VaultMode::Portable, &root).unwrap();
    let store = VaultStore::new(layout);
    store
        .import_pack_file(
            PackClass::Runtime,
            &source,
            &VaultSegment::new("neo.fixture.vcredist.x86").unwrap(),
            &VaultSegment::new("1.2.3").unwrap(),
            &digest,
        )
        .unwrap();

    Fixture {
        root,
        catalogue,
        policy,
        missing: inventory(RuntimeState::Missing, None),
    }
}

fn spec() -> RuntimeExecutionSpec {
    RuntimeExecutionSpec {
        installer: RuntimeInstallerKind::Exe,
        unattended: true,
        install_args: vec!["/quiet".to_string(), "/norestart".to_string()],
        repair_args: Some(vec!["/repair".to_string(), "/quiet".to_string()]),
        success_exit_codes: vec![0, 3010],
        reboot_exit_codes: vec![3010],
        verification: RuntimeVerificationRule::InstalledState,
    }
}

fn inventory(state: RuntimeState, version: Option<&str>) -> RuntimeInventory {
    RuntimeInventory {
        windows_build: 26100,
        architecture: "x64".to_string(),
        observations: vec![RuntimeObservation {
            component: RuntimeComponent::VcRedist2015PlusX86,
            state,
            detected_version: version.map(str::to_string),
            source: "phase8-fake-probe".to_string(),
            details: vec![],
        }],
    }
}

fn prepared(fixture: &Fixture, inventory: &RuntimeInventory) -> PreparedRuntimeExecution {
    prepare_runtime_execution(
        "phase8-fixture-mission",
        "phase8-fixture-transaction",
        RuntimeProfile::Technician,
        RuntimeComponent::VcRedist2015PlusX86,
        inventory,
        &fixture.catalogue,
        &fixture.policy,
        fixture.root.clone(),
        VaultMode::Portable,
    )
    .unwrap()
}

fn authorized_session(prepared: PreparedRuntimeExecution) -> RuntimeExecutionSession {
    let mut session = RuntimeExecutionSession::new(prepared).unwrap();
    let action_id = session.plan.action.id.clone();
    let authorization = TransactionAuthorization {
        plan_fingerprint: session.checkpoint.plan_fingerprint().to_string(),
        approved_action_ids: vec![action_id.clone()],
        manual_override_action_ids: vec![],
        high_risk_ack_action_ids: vec![],
        irreversible_acknowledgements: vec![ActionAcknowledgement {
            action_id,
            reason: "fixture explicitly accepts the absence of generic runtime rollback"
                .to_string(),
        }],
    };
    session.authorize(authorization).unwrap();
    session
}

#[test]
fn missing_runtime_prepares_exact_irreversible_install() {
    let fixture = fixture(spec());
    let prepared = prepared(&fixture, &fixture.missing);
    assert_eq!(prepared.plan.operation, RuntimeExecutionOperation::Install);
    assert_eq!(prepared.plan.package_kind, PackageKind::Runtime);
    assert_eq!(prepared.transaction_plan.actions().len(), 1);
    assert!(!prepared.plan.action.rollback_available);
    assert_eq!(
        prepared.plan.payload_path().unwrap(),
        VaultLayout::new(VaultMode::Portable, &fixture.root)
            .unwrap()
            .runtime_pack_destination(
                &prepared.plan.package_id,
                &prepared.plan.package_version,
                prepared.plan.package_sha256.as_str()
            )
    );
}

#[test]
fn broken_runtime_uses_explicit_repair_arguments() {
    let fixture = fixture(spec());
    let broken = inventory(RuntimeState::Broken, Some("broken"));
    let prepared = prepared(&fixture, &broken);
    assert_eq!(prepared.plan.operation, RuntimeExecutionOperation::Repair);
    assert_eq!(
        prepared.plan.execution_args().unwrap(),
        vec!["/repair".to_string(), "/quiet".to_string()]
    );
}

#[test]
fn repair_without_repair_contract_fails_closed() {
    let mut execution = spec();
    execution.repair_args = None;
    let fixture = fixture(execution);
    let broken = inventory(RuntimeState::Broken, None);
    assert!(matches!(
        prepare_runtime_execution(
            "mission",
            "transaction",
            RuntimeProfile::Technician,
            RuntimeComponent::VcRedist2015PlusX86,
            &broken,
            &fixture.catalogue,
            &fixture.policy,
            fixture.root.clone(),
            VaultMode::Portable,
        ),
        Err(RuntimeExecutorError::MissingRepairArguments(_))
    ));
}

#[test]
fn persisted_operation_state_tamper_is_rejected() {
    let fixture = fixture(spec());
    let prepared = prepared(&fixture, &fixture.missing);
    let mut value = serde_json::to_value(&prepared.plan).unwrap();
    value["baseline"]["state"] = serde_json::json!("installed");
    assert!(serde_json::from_value::<RuntimeExecutionPlan>(value).is_err());
}

#[test]
fn persisted_dependency_authority_tamper_is_rejected() {
    let fixture = fixture(spec());
    let prepared = prepared(&fixture, &fixture.missing);
    let mut value = serde_json::to_value(&prepared.plan).unwrap();
    value["package_dependencies"] = serde_json::json!(["neo.unproven.edge"]);
    assert!(serde_json::from_value::<RuntimeExecutionPlan>(value).is_err());
}

#[test]
fn preflight_drift_blocks_before_applying() {
    let fixture = fixture(spec());
    let mut session = authorized_session(prepared(&fixture, &fixture.missing));
    let changed = inventory(RuntimeState::Installed, Some("1.2.3"));
    let host = FakeHost::new(
        [Ok(changed)],
        RuntimeProcessResult::exited(0, "must not execute"),
    );
    assert!(matches!(
        session.apply(&host),
        Err(RuntimeExecutorError::BaselineDrift(_))
    ));
    assert_eq!(session.checkpoint.stage(), TransactionStage::Authorized);
}

#[test]
fn successful_exit_requires_and_passes_reprobe_before_complete() {
    let fixture = fixture(spec());
    let mut session = authorized_session(prepared(&fixture, &fixture.missing));
    let host = FakeHost::new(
        [
            Ok(fixture.missing.clone()),
            Ok(inventory(RuntimeState::Installed, Some("1.2.3"))),
        ],
        RuntimeProcessResult::exited(0, "fixture installer success"),
    );
    session.apply(&host).unwrap();
    assert_eq!(session.checkpoint.stage(), TransactionStage::Complete);
}

#[test]
fn exit_code_zero_without_runtime_postcondition_fails() {
    let fixture = fixture(spec());
    let mut session = authorized_session(prepared(&fixture, &fixture.missing));
    let host = FakeHost::new(
        [Ok(fixture.missing.clone()), Ok(fixture.missing.clone())],
        RuntimeProcessResult::exited(0, "process success is not verification"),
    );
    session.apply(&host).unwrap();
    assert_eq!(session.checkpoint.stage(), TransactionStage::Failed);
}

#[test]
fn transient_probe_error_leaves_verification_retryable() {
    let fixture = fixture(spec());
    let mut session = authorized_session(prepared(&fixture, &fixture.missing));
    let host = FakeHost::new(
        [
            Ok(fixture.missing.clone()),
            Err("temporary registry probe failure".to_string()),
        ],
        RuntimeProcessResult::exited(0, "fixture installer success"),
    );
    assert!(matches!(
        session.apply(&host),
        Err(RuntimeExecutorError::Host(_))
    ));
    assert_eq!(session.checkpoint.stage(), TransactionStage::Verifying);

    let retry = FakeHost::probe_only([Ok(inventory(
        RuntimeState::Installed,
        Some("1.2.3"),
    ))]);
    session.verify_current(&retry).unwrap();
    assert_eq!(session.checkpoint.stage(), TransactionStage::Complete);
}

#[test]
fn reboot_exit_uses_persistent_checkpoint_and_reprobe() {
    let fixture = fixture(spec());
    let mut session = authorized_session(prepared(&fixture, &fixture.missing));
    let host = FakeHost::new(
        [Ok(fixture.missing.clone())],
        RuntimeProcessResult::exited(3010, "fixture reboot required"),
    );
    session.apply(&host).unwrap();
    assert_eq!(session.checkpoint.stage(), TransactionStage::AwaitingReboot);

    let post_reboot = FakeHost::probe_only([
        Ok(inventory(RuntimeState::Installed, Some("1.2.3"))),
        Ok(inventory(RuntimeState::Installed, Some("1.2.3"))),
    ]);
    session.resume_after_reboot(&post_reboot).unwrap();
    assert_eq!(session.checkpoint.stage(), TransactionStage::Complete);
}

#[test]
fn post_reboot_host_drift_blocks_then_fails_without_fake_rollback() {
    let fixture = fixture(spec());
    let mut session = authorized_session(prepared(&fixture, &fixture.missing));
    let host = FakeHost::new(
        [Ok(fixture.missing.clone())],
        RuntimeProcessResult::exited(3010, "fixture reboot required"),
    );
    session.apply(&host).unwrap();

    let mut drift = inventory(RuntimeState::Installed, Some("1.2.3"));
    drift.windows_build = 99999;
    let post_reboot = FakeHost::probe_only([Ok(drift.clone())]);
    session.resume_after_reboot(&post_reboot).unwrap();
    assert_eq!(session.checkpoint.stage(), TransactionStage::Blocked);

    let retry = FakeHost::probe_only([Ok(drift)]);
    session.reprobe_after_block(&retry).unwrap();
    assert_eq!(session.checkpoint.stage(), TransactionStage::Failed);
}

#[test]
fn started_failed_installer_is_conservatively_recorded_changed() {
    let fixture = fixture(spec());
    let mut session = authorized_session(prepared(&fixture, &fixture.missing));
    let host = FakeHost::new(
        [Ok(fixture.missing.clone())],
        RuntimeProcessResult::exited(1603, "fixture installer failed after start"),
    );
    session.apply(&host).unwrap();
    assert_eq!(session.checkpoint.stage(), TransactionStage::Failed);
    let checkpoint = serde_json::to_value(&session.checkpoint).unwrap();
    assert_eq!(checkpoint["apply_records"][0]["machine_changed"], true);
}

#[test]
fn process_not_started_records_no_machine_change() {
    let fixture = fixture(spec());
    let mut session = authorized_session(prepared(&fixture, &fixture.missing));
    let host = FakeHost::new(
        [Ok(fixture.missing.clone())],
        RuntimeProcessResult::start_failed("fixture process creation failed"),
    );
    session.apply(&host).unwrap();
    assert_eq!(session.checkpoint.stage(), TransactionStage::Failed);
    let checkpoint = serde_json::to_value(&session.checkpoint).unwrap();
    assert_eq!(checkpoint["apply_records"][0]["machine_changed"], false);
}

#[test]
fn exact_version_verification_rejects_wrong_installed_version() {
    let mut execution = spec();
    execution.verification = RuntimeVerificationRule::ExactDetectedVersion {
        value: "1.2.3".to_string(),
    };
    let fixture = fixture(execution);
    let mut session = authorized_session(prepared(&fixture, &fixture.missing));
    let host = FakeHost::new(
        [
            Ok(fixture.missing.clone()),
            Ok(inventory(RuntimeState::Installed, Some("1.2.4"))),
        ],
        RuntimeProcessResult::exited(0, "fixture installer success"),
    );
    session.apply(&host).unwrap();
    assert_eq!(session.checkpoint.stage(), TransactionStage::Failed);
}

use super::*;
use neo_catalogue::{
    Catalogue, DriverArtifact, InfModelEntry, PackageKind, PackageManifest, Provenance,
    RebootRequirement as CatalogueRebootRequirement, RedistributionPolicy, SecurityRequirements,
    SignatureEvidence, SignatureStatus, WindowsApplicability,
};
use neo_device::{DeviceRecord, DriverBinding, OpaqueDeviceId, OrderedDeviceIds};
use neo_transaction::{TransactionAuthorization, TransactionStage};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
struct FakeState {
    inventory: DriverInventory,
    compatible: Vec<String>,
    signature: VerifiedInfSignature,
    packages: BTreeMap<String, StoredDriverPackage>,
    stage_package: StoredDriverPackage,
    install_changes: bool,
    install_error: bool,
    install_reboot: bool,
    target_problem_code: Option<u32>,
    restore_reboot: bool,
    baseline_bindings: BTreeMap<String, DriverBindingBaseline>,
    stage_calls: usize,
    inventory_calls: usize,
    fail_inventory_call: Option<usize>,
    windows_build: u32,
    stage_error_after_insert: bool,
    hide_equivalent: bool,
}

struct FakeHost {
    state: RefCell<FakeState>,
}

impl FakeHost {
    fn new(root: &Path, problem_code: Option<u32>) -> Self {
        let device = fixture_device("USB\\VID_1234&PID_5678\\A", problem_code);
        let baseline = DriverBindingBaseline {
            binding: device.active_driver.clone().unwrap(),
            problem_code,
        };
        let store_dir = root.join("fake-driver-store");
        fs::create_dir_all(&store_dir).unwrap();
        let staged_inf = store_dir.join("oem42.inf");
        let baseline_inf = store_dir.join("oem1.inf");
        fs::write(
            &baseline_inf,
            b"baseline driver inf bytes
",
        )
        .unwrap();
        let baseline_package = StoredDriverPackage {
            published_inf: "oem1.inf".to_string(),
            driver_store_inf: baseline_inf,
        };
        Self {
            state: RefCell::new(FakeState {
                inventory: DriverInventory {
                    devices: vec![device],
                },
                compatible: vec!["USB\\VID_1234&PID_5678\\A".to_string()],
                signature: VerifiedInfSignature {
                    catalog_file: "fixture.cat".to_string(),
                    signer: "Neo Fixture Signer".to_string(),
                    signer_version: Some("1".to_string()),
                },
                packages: BTreeMap::from([("oem1.inf".to_string(), baseline_package)]),
                stage_package: StoredDriverPackage {
                    published_inf: "oem42.inf".to_string(),
                    driver_store_inf: staged_inf,
                },
                install_changes: true,
                install_error: false,
                install_reboot: false,
                target_problem_code: None,
                restore_reboot: false,
                baseline_bindings: BTreeMap::from([(
                    "usb\\vid_1234&pid_5678\\a".to_string(),
                    baseline,
                )]),
                stage_calls: 0,
                inventory_calls: 0,
                fail_inventory_call: None,
                windows_build: 26100,
                stage_error_after_insert: false,
                hide_equivalent: false,
            }),
        }
    }

    fn configure(&self, configure: impl FnOnce(&mut FakeState)) {
        configure(&mut self.state.borrow_mut());
    }
}

impl DriverHost for FakeHost {
    fn windows_build(&self) -> Result<u32, DriverStoreError> {
        Ok(self.state.borrow().windows_build)
    }

    fn inventory(&self) -> Result<DriverInventory, DriverStoreError> {
        let mut state = self.state.borrow_mut();
        state.inventory_calls += 1;
        if state.fail_inventory_call == Some(state.inventory_calls) {
            state.fail_inventory_call = None;
            return Err(DriverStoreError::Windows(
                "synthetic inventory failure".to_string(),
            ));
        }
        Ok(state.inventory.clone())
    }

    fn compatible_present_devices(&self, _inf: &Path) -> Result<Vec<String>, DriverStoreError> {
        Ok(self.state.borrow().compatible.clone())
    }

    fn verify_inf_signature(&self, _inf: &Path) -> Result<VerifiedInfSignature, DriverStoreError> {
        Ok(self.state.borrow().signature.clone())
    }

    fn find_equivalent_package(
        &self,
        _source_inf: &Path,
        _catalogue_files: &[String],
    ) -> Result<Option<StoredDriverPackage>, DriverStoreError> {
        let state = self.state.borrow();
        if state.hide_equivalent {
            Ok(None)
        } else {
            Ok(state.packages.get("oem42.inf").cloned())
        }
    }

    fn resolve_published_package(
        &self,
        published_inf: &str,
    ) -> Result<Option<StoredDriverPackage>, DriverStoreError> {
        Ok(self
            .state
            .borrow()
            .packages
            .get(&published_inf.to_ascii_lowercase())
            .cloned())
    }

    fn stage_driver(&self, source_inf: &Path) -> Result<StoredDriverPackage, DriverStoreError> {
        let mut state = self.state.borrow_mut();
        state.stage_calls += 1;
        fs::copy(source_inf, &state.stage_package.driver_store_inf)?;
        let package = state.stage_package.clone();
        state
            .packages
            .insert(package.published_inf.to_ascii_lowercase(), package.clone());
        if state.stage_error_after_insert {
            Err(DriverStoreError::Windows(
                "synthetic staging failure after Driver Store mutation".to_string(),
            ))
        } else {
            Ok(package)
        }
    }

    fn install_best_match(
        &self,
        instance_id: &str,
    ) -> Result<DriverBackendResult, DriverStoreError> {
        let mut state = self.state.borrow_mut();
        let package = state
            .packages
            .get("oem42.inf")
            .cloned()
            .ok_or_else(|| DriverStoreError::Windows("target package missing".to_string()))?;
        if state.install_changes {
            let target_problem_code = state.target_problem_code;
            let device = state
                .inventory
                .devices
                .iter_mut()
                .find(|device| {
                    device
                        .instance_id
                        .as_str()
                        .eq_ignore_ascii_case(instance_id)
                })
                .ok_or_else(|| {
                    DriverStoreError::Windows(format!("device disappeared: {instance_id}"))
                })?;
            let mut binding = device.active_driver.clone().unwrap_or_default();
            binding.published_name = Some(package.published_inf.clone());
            binding.original_name = Some("fixture.inf".to_string());
            binding.provider = Some("Neo Fixture Vendor".to_string());
            binding.version = Some("2.0.0.0".to_string());
            binding.signer = Some("Neo Fixture Signer".to_string());
            binding.catalog_file = Some("fixture.cat".to_string());
            device.active_driver = Some(binding);
            device.problem_code = target_problem_code;
        }
        if state.install_error {
            return Err(DriverStoreError::Windows(
                "synthetic install failure".to_string(),
            ));
        }
        Ok(DriverBackendResult {
            reboot_required: state.install_reboot,
        })
    }

    fn restore_specific_driver(
        &self,
        instance_id: &str,
        published_inf: &str,
    ) -> Result<DriverBackendResult, DriverStoreError> {
        let mut state = self.state.borrow_mut();
        let identity = instance_id.to_ascii_lowercase();
        let baseline = state
            .baseline_bindings
            .get(&identity)
            .cloned()
            .ok_or_else(|| DriverStoreError::RollbackBindingFailure(instance_id.to_string()))?;
        if !baseline
            .binding
            .published_name
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case(published_inf))
        {
            return Err(DriverStoreError::RollbackBindingFailure(
                instance_id.to_string(),
            ));
        }
        let device = state
            .inventory
            .devices
            .iter_mut()
            .find(|device| {
                device
                    .instance_id
                    .as_str()
                    .eq_ignore_ascii_case(instance_id)
            })
            .ok_or_else(|| DriverStoreError::RollbackBindingFailure(instance_id.to_string()))?;
        device.active_driver = Some(baseline.binding);
        device.problem_code = baseline.problem_code;
        Ok(DriverBackendResult {
            reboot_required: state.restore_reboot,
        })
    }

    fn remove_published_package(&self, published_inf: &str) -> Result<(), DriverStoreError> {
        let mut state = self.state.borrow_mut();
        if state.inventory.devices.iter().any(|device| {
            device
                .active_driver
                .as_ref()
                .and_then(|binding| binding.published_name.as_deref())
                .is_some_and(|value| value.eq_ignore_ascii_case(published_inf))
        }) {
            return Err(DriverStoreError::Windows(
                "package still in use".to_string(),
            ));
        }
        state.packages.remove(&published_inf.to_ascii_lowercase());
        Ok(())
    }
}

fn fixture_device(instance_id: &str, problem_code: Option<u32>) -> DeviceRecord {
    DeviceRecord {
        instance_id: OpaqueDeviceId::new(instance_id).unwrap(),
        description: Some("Fixture USB Device".to_string()),
        manufacturer: Some("Neo Fixture Vendor".to_string()),
        class_name: Some("USBDevice".to_string()),
        class_guid: None,
        problem_code,
        disabled: Some(false),
        ids: OrderedDeviceIds {
            hardware_ids: vec![OpaqueDeviceId::new("USB\\VID_1234&PID_5678").unwrap()],
            compatible_ids: vec![OpaqueDeviceId::new("USB\\Class_FF").unwrap()],
        },
        active_driver: Some(DriverBinding {
            published_name: Some("oem1.inf".to_string()),
            original_name: Some("baseline.inf".to_string()),
            provider: Some("Baseline Vendor".to_string()),
            class_name: Some("USBDevice".to_string()),
            class_guid: None,
            version: Some("1.0.0.0".to_string()),
            date: Some("2025-01-01".to_string()),
            signer: Some("Baseline Signer".to_string()),
            catalog_file: Some("baseline.cat".to_string()),
            service: Some("baseline".to_string()),
        }),
        upper_filters: vec![],
        lower_filters: vec![],
    }
}

fn fixture_catalogue() -> Catalogue {
    Catalogue {
        packages: vec![PackageManifest {
            package_id: "neo.fixture.driver".to_string(),
            name: "Neo Fixture Driver".to_string(),
            vendor: "Neo Fixture Vendor".to_string(),
            version: "2.0.0".to_string(),
            kind: PackageKind::InfDriverBundle,
            provenance: Provenance {
                source_name: "fixture".to_string(),
                source_url: None,
                sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .to_string(),
                redistribution: RedistributionPolicy::Unknown,
            },
            windows: WindowsApplicability {
                architectures: vec!["x64".to_string()],
                minimum_build: Some(19041),
                maximum_build: None,
            },
            driver_artifacts: vec![DriverArtifact {
                inf_path: "drivers/fixture.inf".to_string(),
                models: vec![InfModelEntry {
                    hardware_id: Some(OpaqueDeviceId::new("USB\\VID_1234&PID_5678").unwrap()),
                    compatible_ids: vec![OpaqueDeviceId::new("USB\\Class_FF").unwrap()],
                }],
                catalog_files: vec!["drivers/fixture.cat".to_string()],
                provider: Some("Neo Fixture Vendor".to_string()),
                driver_version: Some("2.0.0.0".to_string()),
                driver_date: Some("2026-08-01".to_string()),
                signature: SignatureEvidence {
                    status: SignatureStatus::Verified,
                    signer: Some("Neo Fixture Signer".to_string()),
                    verification_note: Some("fixture".to_string()),
                },
            }],
            dependencies: vec![],
            conflicts: vec![],
            security: SecurityRequirements::default(),
            reboot: CatalogueRebootRequirement::Recommended,
        }],
    }
}

struct Fixture {
    root: PathBuf,
    host: FakeHost,
}

impl Fixture {
    fn new(problem_code: Option<u32>) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("neo-driverstore-{}-{nonce}", std::process::id()));
        fs::create_dir_all(root.join("drivers")).unwrap();
        fs::write(root.join("drivers/fixture.inf"), b"neo fixture inf bytes\n").unwrap();
        fs::write(root.join("drivers/fixture.cat"), b"fixture cat\n").unwrap();
        let host = FakeHost::new(&root, problem_code);
        Self { root, host }
    }

    fn prepare(&self) -> PreparedDriverInstall {
        prepare_driver_install(
            &self.host,
            &fixture_catalogue(),
            &DriverInstallRequest {
                package_root: self.root.clone(),
                package_id: "neo.fixture.driver".to_string(),
                inf_path: "drivers/fixture.inf".to_string(),
                architecture: "x64".to_string(),
                windows_build: 26100,
                action_id: "install.fixture.driver".to_string(),
                mission_id: "mission.fixture".to_string(),
            },
        )
        .unwrap()
    }

    fn session(&self) -> DriverInstallSession {
        let prepared = self.prepare();
        let mut session = DriverInstallSession::new(prepared).unwrap();
        let authorization = TransactionAuthorization {
            plan_fingerprint: session.transaction().plan_fingerprint().to_string(),
            approved_action_ids: vec!["install.fixture.driver".to_string()],
            manual_override_action_ids: vec![],
            high_risk_ack_action_ids: vec![],
            irreversible_acknowledgements: vec![],
        };
        session.authorize(authorization).unwrap();
        session
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn planner_binds_windows_impact_to_catalogue_impact() {
    let fixture = Fixture::new(None);
    fixture.host.configure(|state| {
        state
            .compatible
            .push("USB\\VID_9999&PID_0001\\B".to_string());
        let mut incompatible = fixture_device("USB\\VID_9999&PID_0001\\B", None);
        incompatible.ids = OrderedDeviceIds {
            hardware_ids: vec![OpaqueDeviceId::new("USB\\VID_9999&PID_0001").unwrap()],
            compatible_ids: vec![OpaqueDeviceId::new("USB\\Class_00").unwrap()],
        };
        state.inventory.devices.push(incompatible);
    });
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
    assert!(matches!(error, DriverStoreError::CatalogueImpactMismatch));
}

#[test]
fn planner_refuses_missing_baseline_driver_package() {
    let fixture = Fixture::new(None);
    fixture.host.configure(|state| state.packages.clear());
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
    assert!(matches!(error, DriverStoreError::MissingBaselinePackage(_)));
}

#[test]
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
    assert!(matches!(
        error,
        DriverStoreError::WindowsBuildMismatch { .. }
    ));
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
    assert!(matches!(
        error,
        DriverStoreError::WindowsBuildMismatch { .. }
    ));
    assert_eq!(session.transaction().stage(), TransactionStage::Authorized);
}

#[test]
fn staging_failure_with_recovered_identity_routes_rollback() {
    let fixture = Fixture::new(None);
    let mut session = fixture.session();
    fixture
        .host
        .configure(|state| state.stage_error_after_insert = true);
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

#[test]
fn source_byte_drift_blocks_before_staging() {
    let fixture = Fixture::new(None);
    let mut session = fixture.session();
    fs::write(
        fixture.root.join("drivers/fixture.inf"),
        b"tampered after authority\n",
    )
    .unwrap();
    let error = session.apply(&fixture.host).unwrap_err();
    assert!(matches!(error, DriverStoreError::PrestateDrift));
    assert_eq!(fixture.host.state.borrow().stage_calls, 0);
    assert_eq!(session.transaction().stage(), TransactionStage::Authorized);
}

#[test]
fn healthy_target_install_reaches_complete() {
    let fixture = Fixture::new(None);
    let mut session = fixture.session();
    session.apply(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::Complete);
    let target = session.target_package().unwrap().published_inf.clone();
    assert!(fixture
        .host
        .state
        .borrow()
        .inventory
        .devices
        .iter()
        .all(|device| device
            .active_driver
            .as_ref()
            .and_then(|binding| binding.published_name.as_deref())
            == Some(target.as_str())));
}

#[test]
fn healthy_windows_noop_cleans_new_store_package_and_completes() {
    let fixture = Fixture::new(None);
    fixture
        .host
        .configure(|state| state.install_changes = false);
    let mut session = fixture.session();
    session.apply(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::Complete);
    let packages = &fixture.host.state.borrow().packages;
    assert!(packages.contains_key("oem1.inf"));
    assert!(!packages.contains_key("oem42.inf"));
}

#[test]
fn unhealthy_windows_noop_fails_without_leaving_staged_package() {
    let fixture = Fixture::new(Some(28));
    fixture
        .host
        .configure(|state| state.install_changes = false);
    let mut session = fixture.session();
    session.apply(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::Failed);
    let packages = &fixture.host.state.borrow().packages;
    assert!(packages.contains_key("oem1.inf"));
    assert!(!packages.contains_key("oem42.inf"));
}

#[test]
fn post_mutation_inventory_failure_routes_conservative_rollback() {
    let fixture = Fixture::new(None);
    let mut session = fixture.session();
    fixture.host.configure(|state| {
        state.fail_inventory_call = Some(state.inventory_calls + 2);
    });
    session.apply(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::RollingBack);
    session.rollback(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::RolledBack);
}

#[test]
fn transient_verification_probe_can_be_retried() {
    let fixture = Fixture::new(None);
    let mut session = fixture.session();
    fixture.host.configure(|state| {
        state.fail_inventory_call = Some(state.inventory_calls + 3);
    });
    let error = session.apply(&fixture.host).unwrap_err();
    assert!(error.to_string().contains("synthetic inventory failure"));
    assert_eq!(session.transaction().stage(), TransactionStage::Verifying);
    session.verify_current(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::Complete);
}

#[test]
fn backend_failure_after_binding_change_routes_exact_rollback() {
    let fixture = Fixture::new(None);
    fixture.host.configure(|state| state.install_error = true);
    let mut session = fixture.session();
    session.apply(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::RollingBack);
    session.rollback(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::RolledBack);
    let packages = &fixture.host.state.borrow().packages;
    assert!(packages.contains_key("oem1.inf"));
    assert!(!packages.contains_key("oem42.inf"));
    assert_eq!(
        fixture.host.state.borrow().inventory.devices[0]
            .active_driver
            .as_ref()
            .unwrap()
            .published_name
            .as_deref(),
        Some("oem1.inf")
    );
}

#[test]
fn unhealthy_target_binding_routes_rollback() {
    let fixture = Fixture::new(None);
    fixture
        .host
        .configure(|state| state.target_problem_code = Some(10));
    let mut session = fixture.session();
    session.apply(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::RollingBack);
    session.rollback(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::RolledBack);
}

#[test]
fn runtime_install_reboot_is_persisted_and_reproven() {
    let fixture = Fixture::new(None);
    fixture.host.configure(|state| state.install_reboot = true);
    let mut session = fixture.session();
    session.apply(&fixture.host).unwrap();
    assert_eq!(
        session.transaction().stage(),
        TransactionStage::AwaitingReboot
    );
    let serialized = serde_json::to_string(&session).unwrap();
    let mut recovered = DriverInstallSession::from_json_str(&serialized).unwrap();
    recovered.resume_after_reboot(&fixture.host).unwrap();
    assert_eq!(recovered.transaction().stage(), TransactionStage::Complete);
}

#[test]
fn rollback_reboot_defers_store_removal_until_binding_is_restored() {
    let fixture = Fixture::new(None);
    fixture.host.configure(|state| {
        state.install_error = true;
        state.restore_reboot = true;
    });
    let mut session = fixture.session();
    session.apply(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::RollingBack);
    session.rollback(&fixture.host).unwrap();
    assert_eq!(
        session.transaction().stage(),
        TransactionStage::AwaitingRollbackReboot
    );
    assert!(fixture
        .host
        .state
        .borrow()
        .packages
        .contains_key("oem42.inf"));
    session.resume_after_rollback_reboot(&fixture.host).unwrap();
    assert_eq!(session.transaction().stage(), TransactionStage::RolledBack);
    let packages = &fixture.host.state.borrow().packages;
    assert!(packages.contains_key("oem1.inf"));
    assert!(!packages.contains_key("oem42.inf"));
}

#[test]
fn direct_session_deserialization_cannot_rebind_transaction() {
    let fixture = Fixture::new(None);
    let session = fixture.session();
    let mut value = serde_json::to_value(&session).unwrap();
    value["driver_plan"]["action_id"] = serde_json::Value::String("other.action".to_string());
    let error = serde_json::from_value::<DriverInstallSession>(value).unwrap_err();
    assert!(error.to_string().contains("session state"));
}

use crate::engine::{apply_with_host, authorize_with_host, DebloatHost};
use crate::{prepare_debloat_execution, DebloatExecutionError};
use neo_debloat::{DebloatCatalogue, DebloatEvidence, DebloatProfile};
use neo_debloat_plan::{
    prepare_debloat_transaction_from_evidence, ExactAppxInventory, ExactPackageDependency,
    ExactPackageIdentity,
};
use neo_transaction::{TransactionAuthorization, TransactionStage};

const MAIN_FULL: &str = "Contoso.Phase16_1.2.3.4_x64__contoso";
const MAIN_FAMILY: &str = "Contoso.Phase16_contoso";
const DEP_FULL: &str = "Contoso.Framework_1.0.0.0_x64__contoso";
const DEP_FAMILY: &str = "Contoso.Framework_contoso";

#[derive(Debug, Clone, Copy)]
enum RemoveMode {
    MainAndDependency,
    MainOnly,
    DependencyOnly,
    NoChange,
    FailAfterMain,
}

struct FakeHost {
    inventory: ExactAppxInventory,
    baseline_main: ExactPackageIdentity,
    baseline_dependency: ExactPackageIdentity,
    remove_mode: RemoveMode,
    register_fails: bool,
    remove_calls: usize,
    register_calls: usize,
}

impl FakeHost {
    fn new(remove_mode: RemoveMode) -> Self {
        let inventory = inventory();
        Self {
            baseline_main: inventory.current_user[0].clone(),
            baseline_dependency: inventory.current_user[1].clone(),
            inventory,
            remove_mode,
            register_fails: false,
            remove_calls: 0,
            register_calls: 0,
        }
    }

    fn remove_full_name(&mut self, full_name: &str) {
        self.inventory
            .current_user
            .retain(|package| !package.full_name.eq_ignore_ascii_case(full_name));
    }

    fn restore_if_missing(&mut self, package: ExactPackageIdentity) {
        if !self
            .inventory
            .current_user
            .iter()
            .any(|current| current.full_name.eq_ignore_ascii_case(&package.full_name))
        {
            self.inventory.current_user.push(package);
        }
    }

    fn has_current(&self, full_name: &str) -> bool {
        self.inventory
            .current_user
            .iter()
            .any(|package| package.full_name.eq_ignore_ascii_case(full_name))
    }
}

impl DebloatHost for FakeHost {
    fn current_inventory(&self) -> Result<ExactAppxInventory, DebloatExecutionError> {
        Ok(self.inventory.clone())
    }

    fn remove_current_user(
        &mut self,
        package_full_name: &str,
    ) -> Result<(), DebloatExecutionError> {
        self.remove_calls += 1;
        assert_eq!(package_full_name, MAIN_FULL);
        match self.remove_mode {
            RemoveMode::MainAndDependency => {
                self.remove_full_name(MAIN_FULL);
                self.remove_full_name(DEP_FULL);
                Ok(())
            }
            RemoveMode::MainOnly => {
                self.remove_full_name(MAIN_FULL);
                Ok(())
            }
            RemoveMode::DependencyOnly => {
                self.remove_full_name(DEP_FULL);
                Ok(())
            }
            RemoveMode::NoChange => Ok(()),
            RemoveMode::FailAfterMain => {
                self.remove_full_name(MAIN_FULL);
                Err(DebloatExecutionError::NativeDeployment(
                    "synthetic failure after mutation".to_string(),
                ))
            }
        }
    }

    fn register_current_user(
        &mut self,
        package_full_name: &str,
        dependency_full_names: &[String],
    ) -> Result<(), DebloatExecutionError> {
        self.register_calls += 1;
        assert_eq!(package_full_name, MAIN_FULL);
        assert_eq!(dependency_full_names, &[DEP_FULL.to_string()]);
        if self.register_fails {
            return Err(DebloatExecutionError::NativeDeployment(
                "synthetic registration failure".to_string(),
            ));
        }
        self.restore_if_missing(self.baseline_dependency.clone());
        self.restore_if_missing(self.baseline_main.clone());
        Ok(())
    }
}

#[test]
fn phase16_successful_exact_current_user_removal_reaches_complete() {
    let mut session = session();
    let mut host = FakeHost::new(RemoveMode::MainAndDependency);
    authorize(&mut session, &host);

    apply_with_host(&mut session, &mut host).expect("exact current-user removal should complete");

    assert_eq!(session.stage(), TransactionStage::Complete);
    assert_eq!(host.remove_calls, 1);
    assert_eq!(host.register_calls, 0);
    assert!(!host.has_current(MAIN_FULL));
    assert!(!host.has_current(DEP_FULL));
}

#[test]
fn phase16_pre_authority_baseline_drift_fails_closed_without_mutation() {
    let mut session = session();
    let mut host = FakeHost::new(RemoveMode::MainOnly);
    host.remove_full_name(MAIN_FULL);
    let auth = authorization(&session);

    let error = authorize_with_host(&mut session, auth, &host)
        .expect_err("baseline drift must block authority");

    assert!(matches!(error, DebloatExecutionError::BaselineDrift(_)));
    assert_eq!(session.stage(), TransactionStage::BaselineCaptured);
    assert_eq!(host.remove_calls, 0);
    assert_eq!(host.register_calls, 0);
}

#[test]
fn phase16_second_baseline_check_blocks_drift_after_authorization() {
    let mut session = session();
    let mut host = FakeHost::new(RemoveMode::MainOnly);
    authorize(&mut session, &host);
    host.remove_full_name(DEP_FULL);

    let error = apply_with_host(&mut session, &mut host)
        .expect_err("drift after authorization must still block before mutation");

    assert!(matches!(error, DebloatExecutionError::BaselineDrift(_)));
    assert_eq!(session.stage(), TransactionStage::Authorized);
    assert_eq!(host.remove_calls, 0);
    assert_eq!(host.register_calls, 0);
}

#[test]
fn phase16_partial_removal_failure_restores_main_and_dependency_baselines() {
    let mut session = session();
    let mut host = FakeHost::new(RemoveMode::FailAfterMain);
    authorize(&mut session, &host);

    let error = apply_with_host(&mut session, &mut host)
        .expect_err("synthetic mutation failure must be surfaced after rollback");

    assert!(matches!(error, DebloatExecutionError::NativeDeployment(_)));
    assert_eq!(session.stage(), TransactionStage::RolledBack);
    assert_eq!(host.remove_calls, 1);
    assert_eq!(host.register_calls, 1);
    assert!(host.has_current(MAIN_FULL));
    assert!(host.has_current(DEP_FULL));
}

#[test]
fn phase16_postcondition_failure_after_dependency_change_forces_rollback() {
    let mut session = session();
    let mut host = FakeHost::new(RemoveMode::DependencyOnly);
    authorize(&mut session, &host);

    let error = apply_with_host(&mut session, &mut host)
        .expect_err("main package still present means removal is not proven");

    assert!(matches!(error, DebloatExecutionError::Observation(_)));
    assert_eq!(session.stage(), TransactionStage::RolledBack);
    assert_eq!(host.register_calls, 1);
    assert!(host.has_current(MAIN_FULL));
    assert!(host.has_current(DEP_FULL));
}

#[test]
fn phase16_api_success_without_machine_change_does_not_invent_rollback_work() {
    let mut session = session();
    let mut host = FakeHost::new(RemoveMode::NoChange);
    authorize(&mut session, &host);

    let error = apply_with_host(&mut session, &mut host)
        .expect_err("unchanged main package cannot satisfy the absent postcondition");

    assert!(matches!(
        error,
        DebloatExecutionError::InvalidPreparedState(_)
    ));
    assert_eq!(session.stage(), TransactionStage::Failed);
    assert_eq!(host.register_calls, 0);
}

#[test]
fn phase16_rollback_registration_failure_remains_failed_and_unresolved() {
    let mut session = session();
    let mut host = FakeHost::new(RemoveMode::FailAfterMain);
    host.register_fails = true;
    authorize(&mut session, &host);

    let error = apply_with_host(&mut session, &mut host)
        .expect_err("rollback registration failure must remain visible");

    assert!(matches!(error, DebloatExecutionError::NativeDeployment(_)));
    assert_eq!(session.stage(), TransactionStage::Failed);
    assert_eq!(host.register_calls, 1);
    assert!(!host.has_current(MAIN_FULL));
}

#[test]
fn phase16_main_only_removal_keeps_dependency_and_still_verifies() {
    let mut session = session();
    let mut host = FakeHost::new(RemoveMode::MainOnly);
    authorize(&mut session, &host);

    apply_with_host(&mut session, &mut host).expect("main absence is the forward postcondition");

    assert_eq!(session.stage(), TransactionStage::Complete);
    assert!(!host.has_current(MAIN_FULL));
    assert!(host.has_current(DEP_FULL));
}

fn authorize(session: &mut crate::DebloatExecutionSession, host: &FakeHost) {
    let auth = authorization(session);
    authorize_with_host(session, auth, host).expect("authority should bind to exact plan");
    assert_eq!(session.stage(), TransactionStage::Authorized);
}

fn authorization(session: &crate::DebloatExecutionSession) -> TransactionAuthorization {
    TransactionAuthorization {
        plan_fingerprint: session
            .plan()
            .transaction()
            .fingerprint()
            .expect("fingerprint must compute"),
        approved_action_ids: vec![session.plan().step().debloat_id().to_string()],
        manual_override_action_ids: Vec::new(),
        high_risk_ack_action_ids: Vec::new(),
        irreversible_acknowledgements: Vec::new(),
    }
}

fn session() -> crate::DebloatExecutionSession {
    let prepared = prepare_debloat_transaction_from_evidence(
        &catalogue(),
        &evidence(),
        &inventory(),
        DebloatProfile::SafeCleanup,
        &["appx.contoso.phase16".to_string()],
        "mission-phase16",
    )
    .expect("Phase 15 readiness must be proven");
    prepare_debloat_execution(&prepared).expect("Phase 16 should accept frozen Phase 15 state")
}

fn catalogue() -> DebloatCatalogue {
    serde_json::from_str(
        r#"{"items":[{"id":"appx.contoso.phase16","package_id":"Contoso.Phase16","title":"Contoso Phase16","category":"Fixture","description":"Synthetic Phase 16 package","class":"safe_optional","scope":"current_user","risk":"low","recommendation":"optional_component","verdict":"certified","selected_by_default":false,"restore":{"kind":"provisioned_image"},"side_effects":[],"preserve_in_profiles":[]}]}"#,
    )
    .expect("catalogue must validate")
}

fn evidence() -> DebloatEvidence {
    serde_json::from_str(
        r#"{"observations":[{"package_id":"Contoso.Phase16","installed":"present","provisioned":"present","version":"1.2.3.4","source":"phase16-test"}]}"#,
    )
    .expect("evidence must validate")
}

fn inventory() -> ExactAppxInventory {
    let dependency = ExactPackageDependency {
        name: "Contoso.Framework".to_string(),
        full_name: DEP_FULL.to_string(),
        family_name: DEP_FAMILY.to_string(),
    };
    let main = ExactPackageIdentity {
        name: "Contoso.Phase16".to_string(),
        full_name: MAIN_FULL.to_string(),
        family_name: MAIN_FAMILY.to_string(),
        is_framework: false,
        is_resource: false,
        is_bundle: false,
        is_optional: false,
        dependencies: vec![dependency.clone()],
    };
    let framework = ExactPackageIdentity {
        name: dependency.name.clone(),
        full_name: dependency.full_name.clone(),
        family_name: dependency.family_name.clone(),
        is_framework: true,
        is_resource: false,
        is_bundle: false,
        is_optional: false,
        dependencies: Vec::new(),
    };
    ExactAppxInventory::new(
        vec![main.clone(), framework.clone()],
        vec![main, framework],
        "phase16-test-native",
    )
    .expect("inventory must validate")
}

#[test]
fn phase16_capability_remains_opaque_even_in_execution_tests() {
    let capability = crate::DebloatExecutorCapability::for_tests();
    let _ = format!("{capability:?}");
}

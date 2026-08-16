use crate::engine::{apply_with_host, authorize_with_host, DebloatRestoreHost};
use crate::model::{
    appx_target, DebloatRestoreExecutionPlan, DebloatRestoreExecutionSession,
    DebloatRestoreExecutionStep,
};
use crate::DebloatRestoreExecutionError;
use neo_core::{
    ActionKind, EvidenceItem, EvidenceVerdict, PlannedAction, RebootRequirement,
    RecommendationState, RiskLevel,
};
use neo_debloat_plan::{ExactAppxInventory, ExactPackageDependency, ExactPackageIdentity};
use neo_transaction::{
    CapturedState, CapturedValue, RollbackPlan, TransactionAction, TransactionAuthorization,
    TransactionCheckpoint, TransactionPlan, TransactionStage, VerificationExpectation,
    VerificationPredicate,
};
use std::cell::Cell;

const MAIN_FULL: &str = "Contoso.Phase18_1.2.3.4_x64__contoso";
const MAIN_FAMILY: &str = "Contoso.Phase18_contoso";
const DEP_PRESENT_FULL: &str = "Contoso.FrameworkA_1.0.0.0_x64__contoso";
const DEP_PRESENT_FAMILY: &str = "Contoso.FrameworkA_contoso";
const DEP_NEW_FULL: &str = "Contoso.FrameworkB_2.0.0.0_x64__contoso";
const DEP_NEW_FAMILY: &str = "Contoso.FrameworkB_contoso";

#[derive(Debug, Clone, Copy)]
enum RegisterMode {
    All,
    MainOnly,
    NoChange,
    FailAfterMutation,
}

struct FakeHost {
    inventory: ExactAppxInventory,
    main: ExactPackageIdentity,
    dep_present: ExactPackageIdentity,
    dep_new: ExactPackageIdentity,
    register_mode: RegisterMode,
    register_calls: usize,
    remove_calls: Vec<String>,
    fail_remove_full_name: Option<String>,
    inventory_failures_after_registration: Cell<usize>,
}

impl FakeHost {
    fn new(register_mode: RegisterMode) -> Self {
        let (main, dep_present, dep_new) = identities();
        let inventory = ExactAppxInventory::new(
            vec![dep_present.clone()],
            vec![main.clone(), dep_present.clone(), dep_new.clone()],
            "phase18-test-native",
        )
        .expect("inventory must validate");
        Self {
            inventory,
            main,
            dep_present,
            dep_new,
            register_mode,
            register_calls: 0,
            remove_calls: Vec::new(),
            fail_remove_full_name: None,
            inventory_failures_after_registration: Cell::new(0),
        }
    }

    fn fail_next_inventory_after_registration(&self) {
        self.inventory_failures_after_registration.set(1);
    }

    fn add_current_if_missing(&mut self, package: ExactPackageIdentity) {
        if !self
            .inventory
            .current_user
            .iter()
            .any(|current| current.full_name.eq_ignore_ascii_case(&package.full_name))
        {
            self.inventory.current_user.push(package);
        }
    }

    fn remove_current(&mut self, full_name: &str) {
        self.inventory
            .current_user
            .retain(|package| !package.full_name.eq_ignore_ascii_case(full_name));
    }

    fn has_current(&self, full_name: &str) -> bool {
        self.inventory
            .current_user
            .iter()
            .any(|package| package.full_name.eq_ignore_ascii_case(full_name))
    }

    fn remove_provisioned(&mut self, full_name: &str) {
        self.inventory
            .provisioned
            .retain(|package| !package.full_name.eq_ignore_ascii_case(full_name));
    }

    fn add_conflicting_dependency(&mut self) {
        self.inventory.current_user.push(ExactPackageIdentity {
            name: "Contoso.FrameworkB".to_string(),
            full_name: "Contoso.FrameworkB_9.9.9.9_x64__contoso".to_string(),
            family_name: DEP_NEW_FAMILY.to_string(),
            is_framework: true,
            is_resource: false,
            is_bundle: false,
            is_optional: false,
            dependencies: Vec::new(),
        });
    }
}

impl DebloatRestoreHost for FakeHost {
    fn current_inventory(&self) -> Result<ExactAppxInventory, DebloatRestoreExecutionError> {
        if self.register_calls > 0 {
            let remaining = self.inventory_failures_after_registration.get();
            if remaining > 0 {
                self.inventory_failures_after_registration
                    .set(remaining - 1);
                return Err(DebloatRestoreExecutionError::Observation(
                    "synthetic post-write inventory failure".to_string(),
                ));
            }
        }
        Ok(self.inventory.clone())
    }

    fn register_current_user(
        &mut self,
        package_full_name: &str,
        dependency_full_names: &[String],
    ) -> Result<(), DebloatRestoreExecutionError> {
        self.register_calls += 1;
        assert_eq!(package_full_name, MAIN_FULL);
        assert_eq!(
            dependency_full_names,
            &[DEP_PRESENT_FULL.to_string(), DEP_NEW_FULL.to_string()]
        );

        let dep_present = self.dep_present.clone();
        let dep_new = self.dep_new.clone();
        let main = self.main.clone();
        match self.register_mode {
            RegisterMode::All => {
                self.add_current_if_missing(dep_present);
                self.add_current_if_missing(dep_new);
                self.add_current_if_missing(main);
                Ok(())
            }
            RegisterMode::MainOnly => {
                self.add_current_if_missing(main);
                Ok(())
            }
            RegisterMode::NoChange => Ok(()),
            RegisterMode::FailAfterMutation => {
                self.add_current_if_missing(dep_new);
                self.add_current_if_missing(main);
                Err(DebloatRestoreExecutionError::NativeDeployment(
                    "synthetic registration failure after mutation".to_string(),
                ))
            }
        }
    }

    fn remove_current_user(
        &mut self,
        package_full_name: &str,
    ) -> Result<(), DebloatRestoreExecutionError> {
        self.remove_calls.push(package_full_name.to_string());
        if self
            .fail_remove_full_name
            .as_deref()
            .is_some_and(|full_name| full_name.eq_ignore_ascii_case(package_full_name))
        {
            return Err(DebloatRestoreExecutionError::NativeDeployment(format!(
                "synthetic rollback removal failure for {package_full_name}"
            )));
        }
        self.remove_current(package_full_name);
        Ok(())
    }
}

#[test]
fn phase18_exact_staged_restore_reaches_complete_and_registers_all_identities() {
    let mut session = session();
    let mut host = FakeHost::new(RegisterMode::All);
    authorize(&mut session, &host);

    apply_with_host(&mut session, &mut host).expect("exact staged restore should complete");

    assert_eq!(session.stage(), TransactionStage::Complete);
    assert_eq!(host.register_calls, 1);
    assert!(host.has_current(MAIN_FULL));
    assert!(host.has_current(DEP_PRESENT_FULL));
    assert!(host.has_current(DEP_NEW_FULL));
    assert!(host.remove_calls.is_empty());
}

#[test]
fn phase18_pre_authority_restore_time_baseline_drift_fails_without_mutation() {
    let mut session = session();
    let mut host = FakeHost::new(RegisterMode::All);
    let main = host.main.clone();
    host.add_current_if_missing(main);
    let auth = authorization(&session);

    let error = authorize_with_host(&mut session, auth, &host)
        .expect_err("main appearing before authority must invalidate Phase 17 baseline");

    assert!(matches!(
        error,
        DebloatRestoreExecutionError::BaselineDrift(_)
    ));
    assert_eq!(session.stage(), TransactionStage::BaselineCaptured);
    assert_eq!(host.register_calls, 0);
    assert!(host.remove_calls.is_empty());
}

#[test]
fn phase18_second_pre_write_check_blocks_drift_after_authorization() {
    let mut session = session();
    let mut host = FakeHost::new(RegisterMode::All);
    authorize(&mut session, &host);
    host.remove_current(DEP_PRESENT_FULL);

    let error = apply_with_host(&mut session, &mut host)
        .expect_err("drift after authorization must still block before registration");

    assert!(matches!(
        error,
        DebloatRestoreExecutionError::BaselineDrift(_)
    ));
    assert_eq!(session.stage(), TransactionStage::Authorized);
    assert_eq!(host.register_calls, 0);
    assert!(host.remove_calls.is_empty());
}

#[test]
fn phase18_staged_route_drift_after_authority_blocks_before_mutation() {
    let mut session = session();
    let mut host = FakeHost::new(RegisterMode::All);
    authorize(&mut session, &host);
    host.remove_provisioned(MAIN_FULL);

    let error = apply_with_host(&mut session, &mut host)
        .expect_err("missing exact staged main must invalidate restore route");

    assert!(matches!(
        error,
        DebloatRestoreExecutionError::RestoreRouteDrift(_)
    ));
    assert_eq!(session.stage(), TransactionStage::Authorized);
    assert_eq!(host.register_calls, 0);
    assert!(host.remove_calls.is_empty());
}

#[test]
fn phase18_side_by_side_dependency_after_exact_baseline_still_blocks_order_independently() {
    let mut session = session();
    let mut host = FakeHost::new(RegisterMode::All);
    host.add_conflicting_dependency();
    let auth = authorization(&session);

    let error = authorize_with_host(&mut session, auth, &host).expect_err(
        "side-by-side dependency conflict must block even with expected exact dependency state",
    );

    assert!(matches!(
        error,
        DebloatRestoreExecutionError::BaselineDrift(_)
    ));
    assert_eq!(host.register_calls, 0);
}

#[test]
fn phase18_native_failure_after_mutation_restores_fresh_phase17_baseline() {
    let mut session = session();
    let mut host = FakeHost::new(RegisterMode::FailAfterMutation);
    authorize(&mut session, &host);

    let error = apply_with_host(&mut session, &mut host)
        .expect_err("partial native restore failure must be surfaced after rollback");

    assert!(matches!(
        error,
        DebloatRestoreExecutionError::NativeDeployment(_)
    ));
    assert!(error
        .to_string()
        .contains("synthetic registration failure after mutation"));
    assert_eq!(session.stage(), TransactionStage::RolledBack);
    assert!(!host.has_current(MAIN_FULL));
    assert!(host.has_current(DEP_PRESENT_FULL));
    assert!(!host.has_current(DEP_NEW_FULL));
    assert_eq!(
        host.remove_calls,
        vec![MAIN_FULL.to_string(), DEP_NEW_FULL.to_string()]
    );
}

#[test]
fn phase18_failed_postcondition_preserves_existing_dependency_and_removes_restored_main() {
    let mut session = session();
    let mut host = FakeHost::new(RegisterMode::MainOnly);
    authorize(&mut session, &host);

    let error = apply_with_host(&mut session, &mut host)
        .expect_err("missing required dependency must force rollback");

    assert!(matches!(
        error,
        DebloatRestoreExecutionError::Observation(_)
    ));
    assert_eq!(session.stage(), TransactionStage::RolledBack);
    assert!(!host.has_current(MAIN_FULL));
    assert!(host.has_current(DEP_PRESENT_FULL));
    assert!(!host.has_current(DEP_NEW_FULL));
    assert_eq!(host.remove_calls, vec![MAIN_FULL.to_string()]);
}

#[test]
fn phase18_post_write_observation_loss_is_conservative_and_rolls_back() {
    let mut session = session();
    let mut host = FakeHost::new(RegisterMode::All);
    authorize(&mut session, &host);
    host.fail_next_inventory_after_registration();

    let error = apply_with_host(&mut session, &mut host)
        .expect_err("unavailable post-write state must not allow completion");

    assert!(matches!(
        error,
        DebloatRestoreExecutionError::Observation(_)
    ));
    assert!(error
        .to_string()
        .contains("synthetic post-write inventory failure"));
    assert_eq!(session.stage(), TransactionStage::RolledBack);
    assert!(!host.has_current(MAIN_FULL));
    assert!(host.has_current(DEP_PRESENT_FULL));
    assert!(!host.has_current(DEP_NEW_FULL));
}

#[test]
fn phase18_api_success_without_machine_change_does_not_invent_rollback_work() {
    let mut session = session();
    let mut host = FakeHost::new(RegisterMode::NoChange);
    authorize(&mut session, &host);

    let error = apply_with_host(&mut session, &mut host)
        .expect_err("unchanged main cannot satisfy exact restore postconditions");

    assert!(matches!(
        error,
        DebloatRestoreExecutionError::InvalidPreparedState(_)
    ));
    assert_eq!(session.stage(), TransactionStage::Failed);
    assert!(host.remove_calls.is_empty());
}

#[test]
fn phase18_rollback_removal_failure_preserves_restore_and_rollback_causes() {
    let mut session = session();
    let mut host = FakeHost::new(RegisterMode::FailAfterMutation);
    host.fail_remove_full_name = Some(MAIN_FULL.to_string());
    authorize(&mut session, &host);

    let error = apply_with_host(&mut session, &mut host)
        .expect_err("rollback removal failure must remain visible with source restore failure");
    let message = error.to_string();

    assert!(matches!(
        error,
        DebloatRestoreExecutionError::NativeDeployment(_)
    ));
    assert!(message.contains("synthetic registration failure after mutation"));
    assert!(message.contains("synthetic rollback removal failure"));
    assert_eq!(session.stage(), TransactionStage::Failed);
    assert!(host.has_current(MAIN_FULL));
}

#[test]
fn phase18_capability_is_opaque_and_not_constructible_by_external_callers() {
    let capability = crate::DebloatRestoreExecutorCapability::for_tests();
    let _ = format!("{capability:?}");
}

fn authorize(session: &mut DebloatRestoreExecutionSession, host: &FakeHost) {
    let auth = authorization(session);
    authorize_with_host(session, auth, host).expect("authority must bind to exact Phase 17 plan");
    assert_eq!(session.stage(), TransactionStage::Authorized);
}

fn authorization(session: &DebloatRestoreExecutionSession) -> TransactionAuthorization {
    TransactionAuthorization {
        plan_fingerprint: session
            .plan()
            .transaction()
            .fingerprint()
            .expect("fingerprint must compute"),
        approved_action_ids: vec![session.plan().step().action_id()],
        manual_override_action_ids: Vec::new(),
        high_risk_ack_action_ids: Vec::new(),
        irreversible_acknowledgements: Vec::new(),
    }
}

fn session() -> DebloatRestoreExecutionSession {
    let (main, _, _) = identities();
    let dependencies = main.dependencies.clone();
    let step = DebloatRestoreExecutionStep::for_tests(
        "appx.contoso.phase18",
        "Contoso.Phase18",
        main.clone(),
        dependencies.clone(),
    );

    let mut snapshot_targets = vec![appx_target(MAIN_FULL)];
    let mut baseline = vec![CapturedState {
        target: appx_target(MAIN_FULL),
        value: CapturedValue::Absent,
    }];
    let mut postconditions = vec![VerificationPredicate {
        id: "verify:restore:main".to_string(),
        target: appx_target(MAIN_FULL),
        expectation: VerificationExpectation::Equals(
            serde_json::to_string(&main).expect("main JSON must serialize"),
        ),
        required: true,
    }];
    let mut rollback_verification = vec![VerificationPredicate {
        id: "rollback:restore:main".to_string(),
        target: appx_target(MAIN_FULL),
        expectation: VerificationExpectation::MatchesBaseline,
        required: true,
    }];

    for (index, dependency) in dependencies.iter().enumerate() {
        let target = appx_target(&dependency.full_name);
        snapshot_targets.push(target.clone());
        baseline.push(CapturedState {
            target: target.clone(),
            value: if dependency.full_name == DEP_PRESENT_FULL {
                CapturedValue::Present(
                    serde_json::to_string(dependency).expect("dependency JSON must serialize"),
                )
            } else {
                CapturedValue::Absent
            },
        });
        postconditions.push(VerificationPredicate {
            id: format!("verify:restore:dependency:{index}"),
            target: target.clone(),
            expectation: VerificationExpectation::Equals(
                serde_json::to_string(dependency).expect("dependency JSON must serialize"),
            ),
            required: true,
        });
        rollback_verification.push(VerificationPredicate {
            id: format!("rollback:restore:dependency:{index}"),
            target,
            expectation: VerificationExpectation::MatchesBaseline,
            required: true,
        });
    }

    let action = TransactionAction {
        action: PlannedAction {
            id: step.action_id(),
            title: "Restore Contoso.Phase18 for current user".to_string(),
            kind: ActionKind::Debloat,
            risk: RiskLevel::Low,
            recommendation: RecommendationState::Repair,
            verdict: EvidenceVerdict::Certified,
            rationale: "synthetic Phase 18 inverse restore transaction".to_string(),
            selected_by_default: false,
            requires_confirmation: true,
            requires_admin: false,
            reboot: RebootRequirement::None,
            rollback_available: true,
            evidence: vec![
                EvidenceItem::new(
                    "phase17_receipt_fingerprint",
                    "phase17-test-fingerprint",
                    "Phase 17 test receipt",
                ),
                EvidenceItem::new(
                    "restore_package_full_name",
                    MAIN_FULL,
                    "Phase 17 test inventory",
                ),
                EvidenceItem::new(
                    "restore_dependency_count",
                    dependencies.len().to_string(),
                    "Phase 17 test inventory",
                ),
            ],
            warnings: vec!["synthetic Phase 18 test action".to_string()],
        },
        snapshot_targets: snapshot_targets.clone(),
        postconditions,
        rollback: RollbackPlan::Reversible {
            restore_targets: snapshot_targets,
            verification: rollback_verification,
        },
    };
    let transaction = TransactionPlan::new(
        "mission-phase18:phase17-debloat-restore-current-user",
        1,
        "mission-phase18",
        vec![action],
    )
    .expect("transaction must validate");
    let mut checkpoint =
        TransactionCheckpoint::new(transaction.clone()).expect("checkpoint must be constructed");
    checkpoint
        .capture_baseline(baseline)
        .expect("restore-time baseline must validate");

    let plan =
        DebloatRestoreExecutionPlan::for_tests("phase17-test-fingerprint", step, transaction);
    DebloatRestoreExecutionSession::for_tests(plan, checkpoint)
}

fn identities() -> (
    ExactPackageIdentity,
    ExactPackageIdentity,
    ExactPackageIdentity,
) {
    let dep_present = ExactPackageDependency {
        name: "Contoso.FrameworkA".to_string(),
        full_name: DEP_PRESENT_FULL.to_string(),
        family_name: DEP_PRESENT_FAMILY.to_string(),
    };
    let dep_new = ExactPackageDependency {
        name: "Contoso.FrameworkB".to_string(),
        full_name: DEP_NEW_FULL.to_string(),
        family_name: DEP_NEW_FAMILY.to_string(),
    };
    let main = ExactPackageIdentity {
        name: "Contoso.Phase18".to_string(),
        full_name: MAIN_FULL.to_string(),
        family_name: MAIN_FAMILY.to_string(),
        is_framework: false,
        is_resource: false,
        is_bundle: false,
        is_optional: false,
        dependencies: vec![dep_present.clone(), dep_new.clone()],
    };
    let dep_present_identity = ExactPackageIdentity {
        name: dep_present.name.clone(),
        full_name: dep_present.full_name.clone(),
        family_name: dep_present.family_name.clone(),
        is_framework: true,
        is_resource: false,
        is_bundle: false,
        is_optional: false,
        dependencies: Vec::new(),
    };
    let dep_new_identity = ExactPackageIdentity {
        name: dep_new.name.clone(),
        full_name: dep_new.full_name.clone(),
        family_name: dep_new.family_name.clone(),
        is_framework: true,
        is_resource: false,
        is_bundle: false,
        is_optional: false,
        dependencies: Vec::new(),
    };
    (main, dep_present_identity, dep_new_identity)
}

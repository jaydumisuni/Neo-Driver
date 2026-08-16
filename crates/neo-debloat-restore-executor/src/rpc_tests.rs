use super::*;
use neo_debloat_history::DebloatRemovalReceipt;
use neo_debloat_history_store::{
    DebloatHistoryRecordId, DebloatHistoryStore, DEBLOAT_HISTORY_STORE_SCHEMA_VERSION,
};
use neo_debloat_plan::{ExactAppxInventory, ExactPackageIdentity};
use neo_vault::{VaultLayout, VaultMode};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);
const FIXTURE: &str = include_str!("../../../fixtures/debloat/phase19_receipt.json");

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "neo-debloat-restore-rpc-{label}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp root");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Clone)]
struct FakeRestoreHost {
    inventory: ExactAppxInventory,
    fail_register: bool,
    register_calls: usize,
    remove_calls: usize,
}

impl FakeRestoreHost {
    fn new(inventory: ExactAppxInventory) -> Self {
        Self {
            inventory,
            fail_register: false,
            register_calls: 0,
            remove_calls: 0,
        }
    }

    fn add_current_from_provisioned(&mut self, full_name: &str) {
        if self
            .inventory
            .current_user
            .iter()
            .any(|package| package.full_name.eq_ignore_ascii_case(full_name))
        {
            return;
        }
        let identity = self
            .inventory
            .provisioned
            .iter()
            .find(|package| package.full_name.eq_ignore_ascii_case(full_name))
            .cloned()
            .expect("requested restore identity must be staged in fixture inventory");
        self.inventory.current_user.push(identity);
    }
}

impl DebloatRestoreHost for FakeRestoreHost {
    fn current_inventory(&self) -> Result<ExactAppxInventory, DebloatRestoreExecutionError> {
        Ok(self.inventory.clone())
    }

    fn register_current_user(
        &mut self,
        package_full_name: &str,
        dependency_full_names: &[String],
    ) -> Result<(), DebloatRestoreExecutionError> {
        self.register_calls += 1;
        if self.fail_register {
            return Err(DebloatRestoreExecutionError::NativeDeployment(
                "fixture register failure".to_string(),
            ));
        }
        for dependency in dependency_full_names {
            self.add_current_from_provisioned(dependency);
        }
        self.add_current_from_provisioned(package_full_name);
        Ok(())
    }

    fn remove_current_user(
        &mut self,
        package_full_name: &str,
    ) -> Result<(), DebloatRestoreExecutionError> {
        self.remove_calls += 1;
        self.inventory
            .current_user
            .retain(|package| !package.full_name.eq_ignore_ascii_case(package_full_name));
        Ok(())
    }
}

fn fixture_receipt() -> DebloatRemovalReceipt {
    DebloatRemovalReceipt::from_json_str(FIXTURE).expect("Phase 19 receipt fixture must validate")
}

fn fixture_inventory(receipt: &DebloatRemovalReceipt) -> ExactAppxInventory {
    let mut provisioned = vec![receipt.main().clone()];
    for dependency in receipt.dependencies() {
        provisioned.push(ExactPackageIdentity {
            name: dependency.name.clone(),
            full_name: dependency.full_name.clone(),
            family_name: dependency.family_name.clone(),
            is_framework: true,
            is_resource: false,
            is_bundle: false,
            is_optional: false,
            dependencies: Vec::new(),
        });
    }
    ExactAppxInventory::new(Vec::new(), provisioned, "phase20-rpc-fixture")
        .expect("fixture inventory must validate")
}

fn fixture_store(root: &TempRoot) -> (DebloatHistoryStore, DebloatHistoryRecordId) {
    let receipt = fixture_receipt();
    let record_id = DebloatHistoryRecordId::from_receipt(&receipt).expect("valid fixture record id");
    let layout = VaultLayout::new(VaultMode::Installed, root.path()).expect("absolute fixture root");
    let store = DebloatHistoryStore::new(layout);
    store.ensure_layout().expect("create trusted history layout");
    let record_dir = store.records_root().join(record_id.as_str());
    fs::create_dir(&record_dir).expect("create fixture record directory");
    let envelope = json!({
        "schema_version": DEBLOAT_HISTORY_STORE_SCHEMA_VERSION,
        "record_id": record_id.as_str(),
        "receipt": serde_json::to_value(&receipt).expect("serialize fixture receipt"),
    });
    fs::write(
        record_dir.join("receipt.json"),
        serde_json::to_vec_pretty(&envelope).expect("serialize store envelope"),
    )
    .expect("write trusted fixture record");
    store.audit().expect("fixture store must audit cleanly");
    (store, record_id)
}

fn caller(kind: DebloatRestoreRpcCallerKind, principal: &str) -> DebloatRestoreRpcCaller {
    DebloatRestoreRpcCaller {
        kind,
        principal: principal.to_string(),
    }
}

fn context(caller: DebloatRestoreRpcCaller, scopes: &[&str]) -> DebloatRestoreRpcContext {
    DebloatRestoreRpcContext {
        caller,
        granted_scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
    }
}

fn policy(callers: &[DebloatRestoreRpcCaller]) -> DebloatRestoreRpcPolicy {
    DebloatRestoreRpcPolicy::new(callers.to_vec()).expect("valid policy")
}

fn prepare_request(record_id: &DebloatHistoryRecordId, suffix: &str) -> DebloatRestoreRpcPrepareRequest {
    DebloatRestoreRpcPrepareRequest {
        request_id: format!("prepare-{suffix}"),
        mission_id: format!("mission-{suffix}"),
        record_id: record_id.to_string(),
    }
}

fn apply_request(
    prepared: &DebloatRestoreRpcPrepared,
    suffix: &str,
) -> DebloatRestoreRpcApplyRequest {
    DebloatRestoreRpcApplyRequest {
        request_id: format!("apply-{suffix}"),
        session_id: prepared.session_id.clone(),
        plan_fingerprint: prepared.plan_fingerprint.clone(),
        approved_action_ids: vec![prepared.action_id.clone()],
        confirmed: true,
    }
}

fn service(
    root: &TempRoot,
    allowed: &[DebloatRestoreRpcCaller],
) -> (
    DebloatRestoreRpcService,
    DebloatHistoryRecordId,
    ExactAppxInventory,
) {
    let (store, record_id) = fixture_store(root);
    let inventory = fixture_inventory(&fixture_receipt());
    let service = DebloatRestoreRpcService::new(store, policy(allowed), "phase20-test-service")
        .expect("valid service");
    (service, record_id, inventory)
}

#[test]
fn request_json_cannot_deserialize_trusted_caller_or_scope_context() {
    let root = TempRoot::new("request-context");
    let owner = caller(DebloatRestoreRpcCallerKind::Hunter, "owner:john");
    let (_service, record_id, _inventory) = service(&root, &[owner]);
    let injected = json!({
        "request_id": "prepare-injected",
        "mission_id": "mission-injected",
        "record_id": record_id.as_str(),
        "caller": {"kind": "hunter", "principal": "attacker"},
        "granted_scopes": [DEBLOAT_RESTORE_APPLY_PERMISSION_SCOPE]
    });
    assert!(serde_json::from_value::<DebloatRestoreRpcPrepareRequest>(injected).is_err());
}

#[test]
fn policy_and_prepare_scope_fail_before_history_selection() {
    let root = TempRoot::new("policy");
    let owner = caller(DebloatRestoreRpcCallerKind::Hunter, "owner:john");
    let outsider = caller(DebloatRestoreRpcCallerKind::Hunter, "user:outsider");
    let (mut service, record_id, inventory) = service(&root, std::slice::from_ref(&owner));

    let unauthorized = service
        .prepare_with_inventory(
            &context(
                outsider,
                &[DEBLOAT_RESTORE_PREPARE_PERMISSION_SCOPE],
            ),
            prepare_request(&record_id, "unauthorized"),
            &inventory,
        )
        .expect_err("unapproved caller must fail");
    assert_eq!(
        unauthorized.code(),
        DebloatRestoreRpcErrorCode::UnauthorizedCaller
    );

    let denied = service
        .prepare_with_inventory(
            &context(owner, &[]),
            prepare_request(&record_id, "missing-scope"),
            &inventory,
        )
        .expect_err("missing prepare scope must fail");
    assert_eq!(denied.code(), DebloatRestoreRpcErrorCode::PermissionDenied);
    assert_eq!(service.pending_session_count(), 0);
}

#[test]
fn prepare_requires_canonical_store_id_and_returns_exact_phase18_plan() {
    let root = TempRoot::new("prepare");
    let owner = caller(DebloatRestoreRpcCallerKind::Oracle, "oracle:owner");
    let (mut service, record_id, inventory) = service(&root, std::slice::from_ref(&owner));
    let ctx = context(
        owner,
        &[
            DEBLOAT_RESTORE_PREPARE_PERMISSION_SCOPE,
            DEBLOAT_RESTORE_APPLY_PERMISSION_SCOPE,
        ],
    );

    let mut upper = prepare_request(&record_id, "upper");
    upper.record_id = upper.record_id.to_ascii_uppercase();
    let error = service
        .prepare_with_inventory(&ctx, upper, &inventory)
        .expect_err("noncanonical disk authority must fail");
    assert_eq!(error.code(), DebloatRestoreRpcErrorCode::InvalidRequest);

    let prepared = service
        .prepare_with_inventory(&ctx, prepare_request(&record_id, "valid"), &inventory)
        .expect("trusted by-id restore preparation must succeed");
    assert_eq!(prepared.record_id, record_id.as_str());
    assert_eq!(prepared.receipt_fingerprint, record_id.as_str());
    assert_eq!(prepared.stage, TransactionStage::BaselineCaptured);
    assert!(prepared.confirmation_required);
    assert!(prepared.action_id.starts_with("restore:"));
    assert!(!prepared.plan_fingerprint.is_empty());
    assert_eq!(service.pending_session_count(), 1);
}

#[test]
fn newer_prepare_replaces_only_that_callers_older_unconfirmed_session() {
    let root = TempRoot::new("replace");
    let owner = caller(DebloatRestoreRpcCallerKind::Hunter, "owner:john");
    let (mut service, record_id, inventory) = service(&root, std::slice::from_ref(&owner));
    let ctx = context(
        owner,
        &[
            DEBLOAT_RESTORE_PREPARE_PERMISSION_SCOPE,
            DEBLOAT_RESTORE_APPLY_PERMISSION_SCOPE,
        ],
    );
    let first = service
        .prepare_with_inventory(&ctx, prepare_request(&record_id, "first"), &inventory)
        .unwrap();
    let second = service
        .prepare_with_inventory(&ctx, prepare_request(&record_id, "second"), &inventory)
        .unwrap();
    assert_ne!(first.session_id, second.session_id);
    assert_eq!(service.pending_session_count(), 1);
    let error = service
        .validate_apply(&ctx, &apply_request(&first, "old"))
        .expect_err("older session must be invalidated");
    assert_eq!(error.code(), DebloatRestoreRpcErrorCode::SessionNotFound);
}

#[test]
fn apply_requires_same_caller_confirmation_fingerprint_and_exact_action_set() {
    let root = TempRoot::new("apply-gates");
    let owner = caller(DebloatRestoreRpcCallerKind::Hunter, "owner:john");
    let admin = caller(DebloatRestoreRpcCallerKind::Oracle, "admin:oracle");
    let (mut service, record_id, inventory) = service(&root, &[owner.clone(), admin.clone()]);
    let owner_ctx = context(
        owner,
        &[
            DEBLOAT_RESTORE_PREPARE_PERMISSION_SCOPE,
            DEBLOAT_RESTORE_APPLY_PERMISSION_SCOPE,
        ],
    );
    let admin_ctx = context(admin, &[DEBLOAT_RESTORE_APPLY_PERMISSION_SCOPE]);
    let prepared = service
        .prepare_with_inventory(&owner_ctx, prepare_request(&record_id, "gates"), &inventory)
        .unwrap();

    let mut request = apply_request(&prepared, "confirm");
    request.confirmed = false;
    assert_eq!(
        service.validate_apply(&owner_ctx, &request).unwrap_err().code(),
        DebloatRestoreRpcErrorCode::ConfirmationRequired
    );
    assert_eq!(service.pending_session_count(), 1);

    let mut request = apply_request(&prepared, "fingerprint");
    request.plan_fingerprint = "0".repeat(64);
    assert_eq!(
        service.validate_apply(&owner_ctx, &request).unwrap_err().code(),
        DebloatRestoreRpcErrorCode::PlanMismatch
    );

    let mut request = apply_request(&prepared, "actions");
    request.approved_action_ids = vec![prepared.action_id.clone(), "restore:extra".to_string()];
    assert_eq!(
        service.validate_apply(&owner_ctx, &request).unwrap_err().code(),
        DebloatRestoreRpcErrorCode::PlanMismatch
    );

    assert_eq!(
        service
            .validate_apply(&admin_ctx, &apply_request(&prepared, "caller"))
            .unwrap_err()
            .code(),
        DebloatRestoreRpcErrorCode::CallerMismatch
    );
    assert_eq!(service.pending_session_count(), 1);
}

#[test]
fn successful_apply_is_single_use_and_completes_exact_restore() {
    let root = TempRoot::new("success");
    let owner = caller(DebloatRestoreRpcCallerKind::Internal, "internal:test");
    let (mut service, record_id, inventory) = service(&root, std::slice::from_ref(&owner));
    let ctx = context(
        owner,
        &[
            DEBLOAT_RESTORE_PREPARE_PERMISSION_SCOPE,
            DEBLOAT_RESTORE_APPLY_PERMISSION_SCOPE,
        ],
    );
    let prepared = service
        .prepare_with_inventory(&ctx, prepare_request(&record_id, "success"), &inventory)
        .unwrap();
    let request = apply_request(&prepared, "success");
    let mut host = FakeRestoreHost::new(inventory);
    let receipt = service
        .apply_with_test_host(&ctx, request.clone(), &mut host)
        .expect("confirmed exact restore must complete");
    assert_eq!(receipt.stage, TransactionStage::Complete);
    assert_eq!(receipt.record_id, record_id.as_str());
    assert_eq!(receipt.changed_action_ids, vec![prepared.action_id.clone()]);
    assert_eq!(host.register_calls, 1);
    assert!(host.inventory.current_user.iter().any(|package| {
        package
            .full_name
            .eq_ignore_ascii_case(&prepared.package_full_name)
    }));
    assert_eq!(service.pending_session_count(), 0);

    let replay = service
        .apply_with_test_host(&ctx, request, &mut host)
        .expect_err("consumed apply authority must not replay");
    assert_eq!(replay.code(), DebloatRestoreRpcErrorCode::SessionNotFound);
}

#[test]
fn execution_failure_consumes_authority_and_requires_fresh_prepare() {
    let root = TempRoot::new("failure-consume");
    let owner = caller(DebloatRestoreRpcCallerKind::Internal, "internal:test");
    let (mut service, record_id, inventory) = service(&root, std::slice::from_ref(&owner));
    let ctx = context(
        owner,
        &[
            DEBLOAT_RESTORE_PREPARE_PERMISSION_SCOPE,
            DEBLOAT_RESTORE_APPLY_PERMISSION_SCOPE,
        ],
    );
    let prepared = service
        .prepare_with_inventory(&ctx, prepare_request(&record_id, "failure"), &inventory)
        .unwrap();
    let request = apply_request(&prepared, "failure");
    let mut host = FakeRestoreHost::new(inventory);
    host.fail_register = true;
    let error = service
        .apply_with_test_host(&ctx, request.clone(), &mut host)
        .expect_err("fixture deployment failure must surface");
    assert_eq!(error.code(), DebloatRestoreRpcErrorCode::ExecutionFailed);
    assert_eq!(service.pending_session_count(), 0);
    assert_eq!(
        service
            .apply_with_test_host(&ctx, request, &mut host)
            .unwrap_err()
            .code(),
        DebloatRestoreRpcErrorCode::SessionNotFound
    );
}

#[test]
fn sequence_exhaustion_fails_closed_without_destroying_existing_authority() {
    let root = TempRoot::new("sequence");
    let owner = caller(DebloatRestoreRpcCallerKind::Hunter, "owner:john");
    let (mut service, record_id, inventory) = service(&root, std::slice::from_ref(&owner));
    let ctx = context(owner, &[DEBLOAT_RESTORE_PREPARE_PERMISSION_SCOPE]);
    let first = service
        .prepare_with_inventory(&ctx, prepare_request(&record_id, "first"), &inventory)
        .unwrap();
    service.set_next_session_sequence_for_tests(u64::MAX);
    let error = service
        .prepare_with_inventory(&ctx, prepare_request(&record_id, "overflow"), &inventory)
        .expect_err("sequence overflow must fail closed");
    assert_eq!(
        error.code(),
        DebloatRestoreRpcErrorCode::ServiceStateExhausted
    );
    assert_eq!(service.pending_session_count(), 1);
    assert!(service.pending.contains_key(&first.session_id));
}

#[test]
fn history_and_fresh_readiness_errors_are_structurally_classified() {
    let root = TempRoot::new("error-codes");
    let owner = caller(DebloatRestoreRpcCallerKind::Hunter, "owner:john");
    let (mut service, record_id, mut inventory) = service(&root, std::slice::from_ref(&owner));
    let ctx = context(owner, &[DEBLOAT_RESTORE_PREPARE_PERMISSION_SCOPE]);

    let missing = DebloatHistoryRecordId::new("0".repeat(64)).unwrap();
    assert_eq!(
        service
            .prepare_with_inventory(&ctx, prepare_request(&missing, "missing"), &inventory)
            .unwrap_err()
            .code(),
        DebloatRestoreRpcErrorCode::HistoryUnavailable
    );

    inventory.current_user.push(fixture_receipt().main().clone());
    inventory.validate().unwrap();
    assert_eq!(
        service
            .prepare_with_inventory(&ctx, prepare_request(&record_id, "already"), &inventory)
            .unwrap_err()
            .code(),
        DebloatRestoreRpcErrorCode::RestoreNotReady
    );
}

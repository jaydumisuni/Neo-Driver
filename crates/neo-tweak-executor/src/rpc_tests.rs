use super::*;
use crate::model::RegistryTweakSpec;
use neo_core::{EvidenceVerdict, RebootRequirement, RecommendationState, RiskLevel};
use neo_state_plan::{TweakDefinition, TweakOperation, TweakTarget, TweakValue};
use std::cell::Cell;

#[derive(Default)]
struct FakeHost {
    values: BTreeMap<String, RegistrySnapshot>,
    reads: Cell<usize>,
    fail_write: bool,
}

impl FakeHost {
    fn with(id: &str, value: RegistrySnapshot) -> Self {
        let mut host = Self::default();
        host.values.insert(id.to_string(), value);
        host
    }
}

impl TweakHost for FakeHost {
    fn read(&self, spec: RegistryTweakSpec) -> Result<RegistrySnapshot, TweakExecutionError> {
        self.reads.set(self.reads.get() + 1);
        Ok(self
            .values
            .get(spec.id)
            .copied()
            .unwrap_or(RegistrySnapshot::Absent))
    }

    fn write_dword(
        &mut self,
        spec: RegistryTweakSpec,
        value: u32,
    ) -> Result<(), TweakExecutionError> {
        if self.fail_write {
            return Err(TweakExecutionError::Registry(
                "synthetic RPC write failure".to_string(),
            ));
        }
        self.values
            .insert(spec.id.to_string(), RegistrySnapshot::Dword(value));
        Ok(())
    }

    fn restore(
        &mut self,
        spec: RegistryTweakSpec,
        baseline: RegistrySnapshot,
    ) -> Result<(), TweakExecutionError> {
        match baseline {
            RegistrySnapshot::Absent => {
                self.values.remove(spec.id);
            }
            RegistrySnapshot::Dword(_) => {
                self.values.insert(spec.id.to_string(), baseline);
            }
        }
        Ok(())
    }
}

fn definition(id: &str, desired: u32) -> TweakDefinition {
    TweakDefinition {
        id: id.to_string(),
        title: id.to_string(),
        category: "customize_preferences".to_string(),
        benefit: "Exercise the curated MCP/RPC preference.".to_string(),
        tradeoff: "Changes one current-user Explorer preference.".to_string(),
        risk: RiskLevel::Low,
        recommendation: RecommendationState::Recommended,
        verdict: EvidenceVerdict::Certified,
        selected_by_default: false,
        requires_admin: false,
        reboot: RebootRequirement::None,
        target: TweakTarget {
            key: id.to_string(),
        },
        operation: TweakOperation::Set {
            value: TweakValue::U32(desired),
        },
        warnings: vec![],
    }
}

fn service() -> TweakRpcService {
    let catalogue = TweakCatalogue::new(vec![
        definition(crate::SHOW_FILE_EXTENSIONS, 0),
        definition(crate::SHOW_HIDDEN_FILES, 1),
        definition(crate::TASKBAR_CENTERED_ICONS, 1),
    ])
    .unwrap();
    TweakRpcService::new(catalogue, TweakRpcPolicy::new(vec![caller()]).unwrap()).unwrap()
}

fn caller() -> TweakRpcCaller {
    TweakRpcCaller {
        kind: TweakRpcCallerKind::Hunter,
        principal: "hunter.owner".to_string(),
    }
}

fn context(scopes: &[&str]) -> TweakRpcContext {
    TweakRpcContext {
        caller: caller(),
        granted_scopes: scopes.iter().map(|scope| (*scope).to_string()).collect(),
    }
}

fn prepare_request(request_id: &str) -> TweakRpcPrepareRequest {
    TweakRpcPrepareRequest {
        request_id: request_id.to_string(),
        mission_id: "mission-rpc".to_string(),
        selected_ids: vec![crate::SHOW_FILE_EXTENSIONS.to_string()],
    }
}

fn apply_request(prepared: &TweakRpcPrepared, confirmed: bool) -> TweakRpcApplyRequest {
    TweakRpcApplyRequest {
        request_id: "apply-1".to_string(),
        session_id: prepared.session_id.clone(),
        plan_fingerprint: prepared.plan_fingerprint.clone(),
        approved_action_ids: prepared
            .actions
            .iter()
            .map(|action| action.tweak_id.clone())
            .collect(),
        confirmed,
    }
}

#[test]
fn mcp_and_rpc_method_names_are_frozen() {
    assert_eq!(MCP_TWEAK_PREPARE_TOOL, "neo_tweaks_prepare");
    assert_eq!(MCP_TWEAK_APPLY_TOOL, "neo_tweaks_apply");
    assert_eq!(RPC_TWEAK_PREPARE_METHOD, "neo.tweaks.prepare");
    assert_eq!(RPC_TWEAK_APPLY_METHOD, "neo.tweaks.apply");
}

#[test]
fn unauthorized_prepare_is_rejected_before_live_read() {
    let mut service = service();
    let host = FakeHost::with(crate::SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    let foreign = TweakRpcContext {
        caller: TweakRpcCaller {
            kind: TweakRpcCallerKind::Oracle,
            principal: "oracle.other".to_string(),
        },
        granted_scopes: vec![TWEAK_PREPARE_PERMISSION_SCOPE.to_string()],
    };
    let error = service
        .prepare_with_test_host(&foreign, prepare_request("prepare-unauthorized"), &host)
        .unwrap_err();
    assert_eq!(error.code(), TweakRpcErrorCode::UnauthorizedCaller);
    assert_eq!(host.reads.get(), 0);
}

#[test]
fn prepare_requires_exact_prepare_scope() {
    let mut service = service();
    let host = FakeHost::with(crate::SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    let error = service
        .prepare_with_test_host(&context(&[]), prepare_request("prepare-no-scope"), &host)
        .unwrap_err();
    assert_eq!(error.code(), TweakRpcErrorCode::PermissionDenied);
    assert_eq!(host.reads.get(), 0);
}

#[test]
fn prepare_returns_baseline_and_transaction_fingerprint() {
    let mut service = service();
    let host = FakeHost::with(crate::SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    let prepared = service
        .prepare_with_test_host(
            &context(&[TWEAK_PREPARE_PERMISSION_SCOPE]),
            prepare_request("prepare-1"),
            &host,
        )
        .unwrap();
    assert_eq!(prepared.schema_version, NEO_RPC_SCHEMA_VERSION);
    assert_eq!(prepared.stage, TransactionStage::BaselineCaptured);
    assert!(prepared.confirmation_required);
    assert_eq!(prepared.actions.len(), 1);
    assert_eq!(prepared.actions[0].baseline, RegistrySnapshot::Dword(1));
    assert_eq!(prepared.actions[0].desired_dword, 0);
    assert_eq!(service.pending_session_count(), 1);
}

#[test]
fn apply_requires_explicit_confirmation_and_keeps_session_retryable() {
    let mut service = service();
    let mut host = FakeHost::with(crate::SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    let prepared = service
        .prepare_with_test_host(
            &context(&[TWEAK_PREPARE_PERMISSION_SCOPE]),
            prepare_request("prepare-2"),
            &host,
        )
        .unwrap();
    let error = service
        .apply_with_test_host(
            &context(&[TWEAK_APPLY_PERMISSION_SCOPE]),
            apply_request(&prepared, false),
            &mut host,
        )
        .unwrap_err();
    assert_eq!(error.code(), TweakRpcErrorCode::ConfirmationRequired);
    assert_eq!(service.pending_session_count(), 1);
}

#[test]
fn apply_is_bound_to_exact_fingerprint_and_action_set() {
    let mut service = service();
    let mut host = FakeHost::with(crate::SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    let prepared = service
        .prepare_with_test_host(
            &context(&[TWEAK_PREPARE_PERMISSION_SCOPE]),
            prepare_request("prepare-3"),
            &host,
        )
        .unwrap();
    let mut request = apply_request(&prepared, true);
    request.plan_fingerprint = "0".repeat(64);
    assert_eq!(
        service
            .apply_with_test_host(
                &context(&[TWEAK_APPLY_PERMISSION_SCOPE]),
                request,
                &mut host,
            )
            .unwrap_err()
            .code(),
        TweakRpcErrorCode::PlanMismatch
    );
    assert_eq!(service.pending_session_count(), 1);

    let mut request = apply_request(&prepared, true);
    request.approved_action_ids.clear();
    assert_eq!(
        service
            .apply_with_test_host(
                &context(&[TWEAK_APPLY_PERMISSION_SCOPE]),
                request,
                &mut host,
            )
            .unwrap_err()
            .code(),
        TweakRpcErrorCode::PlanMismatch
    );
    assert_eq!(service.pending_session_count(), 1);
}

#[test]
fn prepared_session_is_bound_to_original_caller() {
    let mut service = service();
    let mut host = FakeHost::with(crate::SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    let prepared = service
        .prepare_with_test_host(
            &context(&[TWEAK_PREPARE_PERMISSION_SCOPE]),
            prepare_request("prepare-4"),
            &host,
        )
        .unwrap();
    let foreign = TweakRpcContext {
        caller: TweakRpcCaller {
            kind: TweakRpcCallerKind::Hunter,
            principal: "hunter.other".to_string(),
        },
        granted_scopes: vec![TWEAK_APPLY_PERMISSION_SCOPE.to_string()],
    };
    service.policy = TweakRpcPolicy::new(vec![caller(), foreign.caller.clone()]).unwrap();
    assert_eq!(
        service
            .apply_with_test_host(&foreign, apply_request(&prepared, true), &mut host)
            .unwrap_err()
            .code(),
        TweakRpcErrorCode::CallerMismatch
    );
    assert_eq!(service.pending_session_count(), 1);
}

#[test]
fn confirmed_scoped_apply_completes_and_is_single_use() {
    let mut service = service();
    let mut host = FakeHost::with(crate::SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    let prepared = service
        .prepare_with_test_host(
            &context(&[TWEAK_PREPARE_PERMISSION_SCOPE]),
            prepare_request("prepare-5"),
            &host,
        )
        .unwrap();
    let request = apply_request(&prepared, true);
    let receipt = service
        .apply_with_test_host(
            &context(&[TWEAK_APPLY_PERMISSION_SCOPE]),
            request.clone(),
            &mut host,
        )
        .unwrap();
    assert_eq!(receipt.stage, TransactionStage::Complete);
    assert_eq!(
        host.values.get(crate::SHOW_FILE_EXTENSIONS),
        Some(&RegistrySnapshot::Dword(0))
    );
    assert_eq!(service.pending_session_count(), 0);
    assert_eq!(
        service
            .apply_with_test_host(
                &context(&[TWEAK_APPLY_PERMISSION_SCOPE]),
                request,
                &mut host,
            )
            .unwrap_err()
            .code(),
        TweakRpcErrorCode::SessionNotFound
    );
}

#[test]
fn failed_execution_consumes_authority_and_requires_reprepare() {
    let mut service = service();
    let mut host = FakeHost::with(crate::SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    let prepared = service
        .prepare_with_test_host(
            &context(&[TWEAK_PREPARE_PERMISSION_SCOPE]),
            prepare_request("prepare-6"),
            &host,
        )
        .unwrap();
    host.fail_write = true;
    let request = apply_request(&prepared, true);
    let error = service
        .apply_with_test_host(
            &context(&[TWEAK_APPLY_PERMISSION_SCOPE]),
            request.clone(),
            &mut host,
        )
        .unwrap_err();
    assert_eq!(error.code(), TweakRpcErrorCode::ExecutionFailed);
    assert_eq!(service.pending_session_count(), 0);
    assert_eq!(
        service
            .apply_with_test_host(
                &context(&[TWEAK_APPLY_PERMISSION_SCOPE]),
                request,
                &mut host,
            )
            .unwrap_err()
            .code(),
        TweakRpcErrorCode::SessionNotFound
    );
}

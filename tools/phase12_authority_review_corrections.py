#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RPC = ROOT / "crates/neo-tweak-executor/src/rpc.rs"
TESTS = ROOT / "crates/neo-tweak-executor/src/rpc_tests.rs"

rpc = RPC.read_text(encoding="utf-8")

replacements = [
    (
        '#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]\n#[serde(rename_all = "snake_case")]\npub enum TweakRpcCallerKind',
        '#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]\n#[serde(rename_all = "snake_case")]\npub enum TweakRpcCallerKind',
    ),
    (
        '#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]\npub struct TweakRpcCaller',
        '#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]\npub struct TweakRpcCaller',
    ),
    (
        '#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct TweakRpcContext',
        '#[derive(Debug, Clone, PartialEq, Eq, Serialize)]\npub struct TweakRpcContext',
    ),
    (
        '    SessionConflict,\n    CallerMismatch,',
        '    SessionConflict,\n    ServiceStateExhausted,\n    CallerMismatch,',
    ),
    (
        '    #[error("prepared RPC tweak session already exists: {0}")]\n    SessionConflict(String),\n    #[error("RPC caller differs from the caller that prepared the session")]',
        '    #[error("prepared RPC tweak session already exists: {0}")]\n    SessionConflict(String),\n    #[error("RPC service session sequence is exhausted")]\n    ServiceStateExhausted,\n    #[error("RPC caller differs from the caller that prepared the session")]',
    ),
    (
        '            Self::SessionConflict(_) => TweakRpcErrorCode::SessionConflict,\n            Self::CallerMismatch =>',
        '            Self::SessionConflict(_) => TweakRpcErrorCode::SessionConflict,\n            Self::ServiceStateExhausted => TweakRpcErrorCode::ServiceStateExhausted,\n            Self::CallerMismatch =>',
    ),
    (
        '''pub struct TweakRpcService {
    catalogue: TweakCatalogue,
    policy: TweakRpcPolicy,
    pending: BTreeMap<String, PendingTweakRpcSession>,
}

impl TweakRpcService {
    pub fn new(catalogue: TweakCatalogue, policy: TweakRpcPolicy) -> Result<Self, TweakRpcError> {
        catalogue.validate().map_err(TweakExecutionError::from)?;
        Ok(Self {
            catalogue,
            policy,
            pending: BTreeMap::new(),
        })
    }''',
        '''pub struct TweakRpcService {
    catalogue: TweakCatalogue,
    policy: TweakRpcPolicy,
    service_instance_id: String,
    next_session_sequence: u64,
    pending: BTreeMap<String, PendingTweakRpcSession>,
}

impl TweakRpcService {
    pub fn new(
        catalogue: TweakCatalogue,
        policy: TweakRpcPolicy,
        service_instance_id: impl Into<String>,
    ) -> Result<Self, TweakRpcError> {
        catalogue.validate().map_err(TweakExecutionError::from)?;
        let service_instance_id = service_instance_id.into();
        require_text("service instance id", &service_instance_id, 160)?;
        Ok(Self {
            catalogue,
            policy,
            service_instance_id,
            next_session_sequence: 0,
            pending: BTreeMap::new(),
        })
    }''',
    ),
    (
        '''        let session_id = format!("phase12:{}:{plan_fingerprint}", request.request_id);
        if self.pending.contains_key(&session_id) {
            return Err(TweakRpcError::SessionConflict(session_id));
        }
        let actions = session''',
        '''        self.pending
            .retain(|_, pending| pending.caller != context.caller);
        let session_sequence = self
            .next_session_sequence
            .checked_add(1)
            .ok_or(TweakRpcError::ServiceStateExhausted)?;
        self.next_session_sequence = session_sequence;
        let session_id = format!(
            "phase12:{}:{}:{plan_fingerprint}",
            self.service_instance_id, session_sequence
        );
        let actions = session''',
    ),
]

for old, new in replacements:
    if rpc.count(old) != 1:
        raise SystemExit(f"RPC correction target did not match exactly once:\n{old[:120]}")
    rpc = rpc.replace(old, new)

RPC.write_text(rpc, encoding="utf-8")

tests = TESTS.read_text(encoding="utf-8")
old_ctor = '    TweakRpcService::new(catalogue, TweakRpcPolicy::new(vec![caller()]).unwrap()).unwrap()'
new_ctor = '''    TweakRpcService::new(
        catalogue,
        TweakRpcPolicy::new(vec![caller()]).unwrap(),
        "phase12-test-instance",
    )
    .unwrap()'''
if tests.count(old_ctor) != 1:
    raise SystemExit("test service constructor target did not match exactly once")
tests = tests.replace(old_ctor, new_ctor)

anchor = '''#[test]
fn failed_execution_consumes_authority_and_requires_reprepare() {'''
new_test = '''#[test]
fn reprepare_invalidates_prior_plan_and_stale_apply_cannot_target_fresh_authority() {
    let mut service = service();
    let mut host = FakeHost::with(crate::SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    let first = service
        .prepare_with_test_host(
            &context(&[TWEAK_PREPARE_PERMISSION_SCOPE]),
            prepare_request("prepare-replay"),
            &host,
        )
        .unwrap();
    let stale_apply = apply_request(&first, true);

    let second = service
        .prepare_with_test_host(
            &context(&[TWEAK_PREPARE_PERMISSION_SCOPE]),
            prepare_request("prepare-replay"),
            &host,
        )
        .unwrap();

    assert_eq!(first.plan_fingerprint, second.plan_fingerprint);
    assert_ne!(first.session_id, second.session_id);
    assert_eq!(service.pending_session_count(), 1);
    assert_eq!(
        service
            .apply_with_test_host(
                &context(&[TWEAK_APPLY_PERMISSION_SCOPE]),
                stale_apply,
                &mut host,
            )
            .unwrap_err()
            .code(),
        TweakRpcErrorCode::SessionNotFound
    );
    assert_eq!(service.pending_session_count(), 1);

    let receipt = service
        .apply_with_test_host(
            &context(&[TWEAK_APPLY_PERMISSION_SCOPE]),
            apply_request(&second, true),
            &mut host,
        )
        .unwrap();
    assert_eq!(receipt.stage, TransactionStage::Complete);
    assert_eq!(service.pending_session_count(), 0);
}

#[test]
fn failed_execution_consumes_authority_and_requires_reprepare() {'''
if tests.count(anchor) != 1:
    raise SystemExit("test insertion anchor did not match exactly once")
tests = tests.replace(anchor, new_test)
TESTS.write_text(tests, encoding="utf-8")

#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RPC = ROOT / "crates/neo-tweak-executor/src/rpc.rs"
TESTS = ROOT / "crates/neo-tweak-executor/src/rpc_tests.rs"

rpc = RPC.read_text(encoding="utf-8")
old_import = "use crate::{prepare_windows_tweaks, RegistrySnapshot, TweakExecutionError};"
new_import = "use crate::{curated_tweak_ids, prepare_windows_tweaks, RegistrySnapshot, TweakExecutionError};"
if rpc.count(old_import) != 1:
    raise SystemExit("RPC crate import target did not match exactly once")
rpc = rpc.replace(old_import, new_import)

old_prepare = '''        if request.selected_ids.is_empty() {
            return Err(TweakRpcError::InvalidRequest(
                "selected tweak ids must not be empty".to_string(),
            ));
        }
        unique_text_set("selected tweak id", &request.selected_ids, 240)?;'''
new_prepare = '''        if request.selected_ids.is_empty() {
            return Err(TweakRpcError::InvalidRequest(
                "selected tweak ids must not be empty".to_string(),
            ));
        }
        if request.selected_ids.len() > curated_tweak_ids().len() {
            return Err(TweakRpcError::InvalidRequest(
                "selected tweak ids exceed the curated Phase 11 action ceiling".to_string(),
            ));
        }
        unique_text_set("selected tweak id", &request.selected_ids, 240)?;'''
if rpc.count(old_prepare) != 1:
    raise SystemExit("prepare cardinality target did not match exactly once")
rpc = rpc.replace(old_prepare, new_prepare)

old_apply = '''        let approved = unique_text_set("approved action id", &request.approved_action_ids, 240)?;
        let expected = pending'''
new_apply = '''        if request.approved_action_ids.len() > curated_tweak_ids().len() {
            return Err(TweakRpcError::PlanMismatch);
        }
        let approved = unique_text_set("approved action id", &request.approved_action_ids, 240)?;
        let expected = pending'''
if rpc.count(old_apply) != 1:
    raise SystemExit("apply cardinality target did not match exactly once")
rpc = rpc.replace(old_apply, new_apply)
RPC.write_text(rpc, encoding="utf-8")

tests = TESTS.read_text(encoding="utf-8")
anchor = '''#[test]
fn prepare_returns_baseline_and_transaction_fingerprint() {'''
insert = '''#[test]
fn oversized_action_lists_fail_closed_before_extra_authority_work() {
    let mut service = service();
    let mut host = FakeHost::with(crate::SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    let oversized_prepare = TweakRpcPrepareRequest {
        request_id: "prepare-oversized".to_string(),
        mission_id: "mission-rpc".to_string(),
        selected_ids: vec![
            crate::SHOW_FILE_EXTENSIONS.to_string(),
            crate::SHOW_HIDDEN_FILES.to_string(),
            crate::TASKBAR_CENTERED_ICONS.to_string(),
            "windows.unapproved.fourth".to_string(),
        ],
    };
    let error = service
        .prepare_with_test_host(
            &context(&[TWEAK_PREPARE_PERMISSION_SCOPE]),
            oversized_prepare,
            &host,
        )
        .unwrap_err();
    assert_eq!(error.code(), TweakRpcErrorCode::InvalidRequest);
    assert_eq!(host.reads.get(), 0);

    let prepared = service
        .prepare_with_test_host(
            &context(&[TWEAK_PREPARE_PERMISSION_SCOPE]),
            prepare_request("prepare-cardinality"),
            &host,
        )
        .unwrap();
    let mut oversized_apply = apply_request(&prepared, true);
    oversized_apply.approved_action_ids = vec![
        crate::SHOW_FILE_EXTENSIONS.to_string(),
        crate::SHOW_HIDDEN_FILES.to_string(),
        crate::TASKBAR_CENTERED_ICONS.to_string(),
        "windows.unapproved.fourth".to_string(),
    ];
    let error = service
        .apply_with_test_host(
            &context(&[TWEAK_APPLY_PERMISSION_SCOPE]),
            oversized_apply,
            &mut host,
        )
        .unwrap_err();
    assert_eq!(error.code(), TweakRpcErrorCode::PlanMismatch);
    assert_eq!(service.pending_session_count(), 1);
}

#[test]
fn prepare_returns_baseline_and_transaction_fingerprint() {'''
if tests.count(anchor) != 1:
    raise SystemExit("cardinality regression anchor did not match exactly once")
tests = tests.replace(anchor, insert)
TESTS.write_text(tests, encoding="utf-8")

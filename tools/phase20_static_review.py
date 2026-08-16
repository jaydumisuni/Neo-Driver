#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates" / "neo-debloat-restore-executor"
RPC = (CRATE / "src" / "rpc.rs").read_text(encoding="utf-8")
RPC_TESTS = (CRATE / "src" / "rpc_tests.rs").read_text(encoding="utf-8")
MODEL = (CRATE / "src" / "model.rs").read_text(encoding="utf-8")
LIB = (CRATE / "src" / "lib.rs").read_text(encoding="utf-8")
MANIFEST = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
HISTORY_STORE_MANIFEST = (
    ROOT / "crates" / "neo-debloat-history-store" / "Cargo.toml"
).read_text(encoding="utf-8")
HISTORY_STORE = (
    ROOT / "crates" / "neo-debloat-history-store" / "src" / "store.rs"
).read_text(encoding="utf-8")
HISTORY_MANIFEST = (ROOT / "crates" / "neo-debloat-history" / "Cargo.toml").read_text(
    encoding="utf-8"
)
CLI = (ROOT / "crates" / "neo-cli" / "src" / "main.rs").read_text(encoding="utf-8")
DECISION = (
    ROOT / "docs" / "decisions" / "0020-PHASE20-DEBLOAT-RESTORE-RPC-AUTHORITY.md"
).read_text(encoding="utf-8")
REVIEW = (ROOT / "docs" / "PHASE20_20_LANE_REVIEW.md").read_text(encoding="utf-8")
CI = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")


def has_all(text: str, values: tuple[str, ...]) -> bool:
    return all(value in text for value in values)


def appears_before(text: str, first: str, second: str) -> bool:
    left = text.find(first)
    right = text.find(second)
    return left >= 0 and right >= 0 and left < right


required_errors = (
    "InvalidRequest",
    "UnauthorizedCaller",
    "PermissionDenied",
    "ConfirmationRequired",
    "SessionNotFound",
    "ServiceStateExhausted",
    "CallerMismatch",
    "PlanMismatch",
    "HistoryUnavailable",
    "RestoreNotReady",
    "UnsupportedPlatform",
    "ExecutionFailed",
)

checks = [
    (
        "authority-continuity",
        has_all(
            RPC,
            (
                "pub struct DebloatRestoreRpcService",
                ".prepare_windows_restore_by_id(&record_id",
                "prepare_debloat_restore_execution(&prepared)?",
                "DebloatRestoreExecutorCapability::for_rpc()",
            ),
        )
        and "trusted Phase 19 history record" in DECISION,
    ),
    (
        "trusted-transport-context",
        "#[derive(Debug, Clone, PartialEq, Eq, Serialize)]\npub struct DebloatRestoreRpcContext"
        in RPC
        and "#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]\npub struct DebloatRestoreRpcCaller"
        in RPC
        and "Deserialize)]\npub struct DebloatRestoreRpcContext" not in RPC
        and "Deserialize)]\npub struct DebloatRestoreRpcCaller" not in RPC,
    ),
    (
        "untrusted-request-shape",
        RPC.count("#[serde(deny_unknown_fields)]") == 2
        and "pub struct DebloatRestoreRpcPrepareRequest" in RPC
        and "pub struct DebloatRestoreRpcApplyRequest" in RPC
        and "caller:" not in RPC.split("pub struct DebloatRestoreRpcPrepareRequest", 1)[1].split("}", 1)[0]
        and "granted_scopes:" not in RPC.split("pub struct DebloatRestoreRpcApplyRequest", 1)[1].split("}", 1)[0]
        and "request_json_cannot_deserialize_trusted_caller_or_scope_context" in RPC_TESTS,
    ),
    (
        "exact-caller-policy",
        has_all(
            RPC,
            (
                "allowed_callers: BTreeSet<DebloatRestoreRpcCaller>",
                "self.allowed_callers.contains(caller)",
                "UnauthorizedCaller",
            ),
        )
        and "policy_and_prepare_scope_fail_before_history_selection" in RPC_TESTS,
    ),
    (
        "scoped-permissions",
        has_all(
            RPC,
            (
                '"neo.debloat.restore.prepare"',
                '"neo.debloat.restore.low-risk.apply"',
                "self.validate_context(context, DEBLOAT_RESTORE_PREPARE_PERMISSION_SCOPE)?",
                "self.validate_context(context, DEBLOAT_RESTORE_APPLY_PERMISSION_SCOPE)?",
            ),
        ),
    ),
    (
        "typed-trusted-history-selection",
        has_all(
            RPC,
            (
                "DebloatHistoryRecordId::new(request.record_id.clone())?",
                "request.record_id != record_id.as_str()",
                ".prepare_windows_restore_by_id(&record_id",
            ),
        )
        and "caller-supplied receipt JSON" in DECISION
        and "filesystem paths" in DECISION,
    ),
    (
        "fresh-phase17-readiness",
        "prepare_windows_restore_by_id" in RPC
        and "prepare_restore_from_inventory_by_id" in RPC
        and "pub fn prepare_windows_restore_by_id" in HISTORY_STORE
        and "let stored = self.load(record_id)?;" in HISTORY_STORE
        and "prepare_windows_restore_from_receipt" in HISTORY_STORE,
    ),
    (
        "phase18-shape-continuity",
        "prepare_debloat_restore_execution(&prepared)?" in RPC
        and "Phase 18 execution-session validation" in DECISION
        and "pub fn prepare_debloat_restore_execution" in LIB,
    ),
    (
        "fingerprint-and-confirmation-binding",
        has_all(
            RPC,
            (
                "session.plan().transaction().fingerprint()?",
                "if !request.confirmed",
                "if request.plan_fingerprint != pending.plan_fingerprint",
                "ConfirmationRequired",
                "PlanMismatch",
            ),
        )
        and "apply_requires_same_caller_confirmation_fingerprint_and_exact_action_set" in RPC_TESTS,
    ),
    (
        "exact-single-action-approval",
        "if request.approved_action_ids.len() != 1" in RPC
        and "BTreeSet::from([pending.session.plan().step().action_id()])" in RPC
        and "approved != expected" in RPC,
    ),
    (
        "caller-continuity",
        "if pending.caller != context.caller" in RPC
        and "CallerMismatch" in RPC
        and "apply_requires_same_caller_confirmation_fingerprint_and_exact_action_set" in RPC_TESTS,
    ),
    (
        "bounded-pending-authority",
        "self.pending\n            .retain(|_, pending| pending.caller != context.caller);" in RPC
        and "newer_prepare_replaces_only_that_callers_older_unconfirmed_session" in RPC_TESTS,
    ),
    (
        "monotonic-session-identity",
        ".checked_add(1)" in RPC
        and '"phase20:{}:{}:{plan_fingerprint}"' in RPC
        and "ServiceStateExhausted" in RPC
        and "sequence_exhaustion_fails_closed_without_destroying_existing_authority" in RPC_TESTS,
    ),
    (
        "single-use-replay-resistance",
        appears_before(
            RPC,
            ".pending\n            .remove(&request.session_id)",
            "DebloatRestoreExecutorCapability::for_rpc()",
        )
        and "successful_apply_is_single_use_and_completes_exact_restore" in RPC_TESTS
        and "execution_failure_consumes_authority_and_requires_fresh_prepare" in RPC_TESTS,
    ),
    (
        "capability-opacity",
        "pub(crate) fn for_rpc() -> Self" in MODEL
        and "pub fn for_rpc()" not in MODEL
        and "pub fn new() -> Self" not in MODEL.split("pub struct DebloatRestoreExecutorCapability", 1)[1]
        and "DebloatRestoreExecutorCapability::for_rpc()" in RPC,
    ),
    (
        "acyclic-owning-layer",
        'neo-debloat-history-store = { path = "../neo-debloat-history-store" }' in MANIFEST
        and "neo-debloat-restore-executor" not in HISTORY_STORE_MANIFEST
        and "neo-debloat-restore-executor" not in HISTORY_MANIFEST,
    ),
    (
        "structured-error-taxonomy",
        all(value in RPC for value in required_errors)
        and "pub fn code(&self) -> DebloatRestoreRpcErrorCode" in RPC
        and "history_and_fresh_readiness_errors_are_structurally_classified" in RPC_TESTS,
    ),
    (
        "no-cli-or-shell-bypass",
        "DebloatRestoreExecutorCapability" not in CLI
        and "neo_debloat_restore_apply" not in CLI
        and "neo.debloat.restore.apply" not in CLI
        and "std::process::Command" not in RPC
        and "GitHub" not in RPC,
    ),
    (
        "no-authority-widening",
        "Store/network/vendor package acquisition" in DECISION
        and "batch restore" in DECISION
        and "all-users restore" in DECISION
        and "plugin dependency" in DECISION
        and "Store/network acquisition" in REVIEW
        and "batch/all-users/provisioned restore" in REVIEW,
    ),
    (
        "proof-and-ci-wiring",
        "Phase 20 twenty-lane static review" in CI
        and "python -W error tools/phase20_static_review.py" in CI
        and "Phase 20 Debloat restore MCP/RPC authority proof" in CI
        and "cargo test --locked -p neo-debloat-restore-executor rpc::tests" in CI
        and all(
            name in RPC_TESTS
            for name in (
                "request_json_cannot_deserialize_trusted_caller_or_scope_context",
                "policy_and_prepare_scope_fail_before_history_selection",
                "prepare_requires_canonical_store_id_and_returns_exact_phase18_plan",
                "newer_prepare_replaces_only_that_callers_older_unconfirmed_session",
                "apply_requires_same_caller_confirmation_fingerprint_and_exact_action_set",
                "successful_apply_is_single_use_and_completes_exact_restore",
                "execution_failure_consumes_authority_and_requires_fresh_prepare",
                "sequence_exhaustion_fails_closed_without_destroying_existing_authority",
                "history_and_fresh_readiness_errors_are_structurally_classified",
            )
        ),
    ),
]

failed: list[str] = []
for index, (name, passed) in enumerate(checks, 1):
    print(f"{index:02d}. {'PASS' if passed else 'FAIL'} - {name}")
    if not passed:
        failed.append(name)

if failed:
    raise SystemExit("Phase 20 static review failed: " + ", ".join(failed))

print("PHASE 20 STATIC REVIEW PASS: 20/20")
sys.exit(0)

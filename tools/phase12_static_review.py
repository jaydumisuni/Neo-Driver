#!/usr/bin/env python3
from __future__ import annotations

import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LIB = (ROOT / "crates/neo-tweak-executor/src/lib.rs").read_text(encoding="utf-8")
MODEL = (ROOT / "crates/neo-tweak-executor/src/model.rs").read_text(encoding="utf-8")
RPC = (ROOT / "crates/neo-tweak-executor/src/rpc.rs").read_text(encoding="utf-8")
RPC_TESTS = (ROOT / "crates/neo-tweak-executor/src/rpc_tests.rs").read_text(encoding="utf-8")
SESSION = (ROOT / "crates/neo-tweak-executor/src/session.rs").read_text(encoding="utf-8")
DECISION = (
    ROOT / "docs/decisions/0012-PHASE12-MCP-RPC-TWEAK-AUTHORITY.md"
).read_text(encoding="utf-8")
REVIEW = (ROOT / "docs/PHASE12_20_LANE_REVIEW.md").read_text(encoding="utf-8")
CLI_MANIFEST = tomllib.loads((ROOT / "crates/neo-cli/Cargo.toml").read_text(encoding="utf-8"))
CLI_MAIN = (ROOT / "crates/neo-cli/src/main.rs").read_text(encoding="utf-8")


def test_functions(text: str) -> set[str]:
    return set(
        re.findall(r"(?m)^\s*#\[test\]\s*\n\s*fn\s+([A-Za-z0-9_]+)\s*\(", text)
    )


def struct_body(text: str, name: str) -> str:
    match = re.search(rf"pub struct {re.escape(name)}\s*\{{(.*?)\n\}}", text, re.S)
    return match.group(1) if match else ""


prepare_request = struct_body(RPC, "TweakRpcPrepareRequest")
apply_request = struct_body(RPC, "TweakRpcApplyRequest")
context = struct_body(RPC, "TweakRpcContext")
regressions = test_functions(RPC_TESTS)
cli_dependencies = CLI_MANIFEST.get("dependencies", {})

required_regressions = {
    "mcp_and_rpc_method_names_are_frozen",
    "unauthorized_prepare_is_rejected_before_live_read",
    "prepare_requires_exact_prepare_scope",
    "prepare_returns_baseline_and_transaction_fingerprint",
    "apply_requires_explicit_confirmation_and_keeps_session_retryable",
    "apply_is_bound_to_exact_fingerprint_and_action_set",
    "prepared_session_is_bound_to_original_caller",
    "confirmed_scoped_apply_completes_and_is_single_use",
    "failed_execution_consumes_authority_and_requires_reprepare",
}

decision_markers = {
    "approved MCP caller",
    "authenticated workstation/local RPC transport",
    "trusted server-side context",
    "must not be allowed to self-assert a principal",
    "single-use authority",
    "no public constructor",
    "GitHub as an interactive execution transport",
    "does **not** claim that CI or ATHENA has performed a live Registry mutation",
}

checks = [
    (
        "protocol-identity",
        all(
            marker in RPC
            for marker in [
                'NEO_RPC_SCHEMA_VERSION: &str = "neo-rpc-v1"',
                'MCP_TWEAK_PREPARE_TOOL: &str = "neo_tweaks_prepare"',
                'MCP_TWEAK_APPLY_TOOL: &str = "neo_tweaks_apply"',
                'RPC_TWEAK_PREPARE_METHOD: &str = "neo.tweaks.prepare"',
                'RPC_TWEAK_APPLY_METHOD: &str = "neo.tweaks.apply"',
            ]
        ),
    ),
    (
        "transport-context-separate-from-request",
        all(marker in context for marker in ["caller: TweakRpcCaller", "granted_scopes: Vec<String>"])
        and all(marker not in prepare_request + apply_request for marker in ["caller", "principal", "granted_scopes"]),
    ),
    (
        "exact-caller-policy",
        "allowed_callers: BTreeSet<TweakRpcCaller>" in RPC
        and "self.allowed_callers.contains(caller)" in RPC,
    ),
    (
        "prepare-permission-before-host-work",
        'TWEAK_PREPARE_PERMISSION_SCOPE: &str = "neo.tweaks.prepare"' in RPC
        and "self.validate_prepare(context, &request)?;" in RPC
        and RPC.index("self.validate_prepare(context, &request)?;")
        < RPC.index("let session = prepare_windows_tweaks("),
    ),
    (
        "apply-permission",
        'TWEAK_APPLY_PERMISSION_SCOPE: &str = "neo.tweaks.low-risk.apply"' in RPC
        and "self.validate_context(context, TWEAK_APPLY_PERMISSION_SCOPE)?;" in RPC,
    ),
    (
        "bounded-duplicate-free-request-validation",
        all(marker in RPC for marker in ["chars().any(char::is_control)", "unique_text_set", "duplicate {label}"]),
    ),
    (
        "curated-phase11-preparation-reuse",
        "prepare_windows_tweaks(" in RPC
        and "prepare_with_host(" in RPC
        and all(marker not in RPC for marker in ["RegSetValueExW", "RegDeleteValueW", "RegistryTweakSpec {"]),
    ),
    (
        "actual-baseline-exposed",
        "baseline: RegistrySnapshot" in RPC and "baseline: step.baseline()" in RPC,
    ),
    (
        "transaction-fingerprint-exposed",
        "plan_fingerprint: String" in RPC
        and ".transaction()" in RPC
        and ".fingerprint()" in RPC,
    ),
    (
        "explicit-confirmation",
        "pub confirmed: bool" in apply_request
        and "if !request.confirmed" in RPC
        and "TweakRpcError::ConfirmationRequired" in RPC,
    ),
    (
        "caller-continuity",
        "if pending.caller != context.caller" in RPC and "TweakRpcError::CallerMismatch" in RPC,
    ),
    (
        "fingerprint-continuity",
        "if request.plan_fingerprint != pending.plan_fingerprint" in RPC
        and "TweakRpcError::PlanMismatch" in RPC,
    ),
    (
        "exact-action-set-continuity",
        "let approved = unique_text_set" in RPC
        and "collect::<BTreeSet<_>>()" in RPC
        and "if approved != expected" in RPC,
    ),
    (
        "phase4-authorization-reuse",
        "TransactionAuthorization" in RPC
        and "manual_override_action_ids: vec![]" in RPC
        and "high_risk_ack_action_ids: vec![]" in RPC
        and "irreversible_acknowledgements: vec![]" in RPC,
    ),
    (
        "opaque-crate-private-rpc-capability",
        "pub struct TweakExecutorCapability" in MODEL
        and "pub(crate) fn for_rpc()" in MODEL
        and "pub fn for_rpc" not in MODEL
        and "pub fn new" not in MODEL.split("pub struct TweakExecutorCapability", 1)[1]
        and "TweakExecutorCapability::for_rpc()" in RPC,
    ),
    (
        "existing-executor-authorize-apply-reuse",
        "pending.session.authorize(&capability, authorization)?;" in RPC
        and "pending.session.apply(&capability)?;" in RPC
        and "ensure_baseline_unchanged" in SESSION
        and "rollback_with_host" in SESSION,
    ),
    (
        "single-use-before-authority",
        ".remove(&request.session_id)" in RPC
        and RPC.index(".remove(&request.session_id)") < RPC.index("TweakExecutorCapability::for_rpc()"),
    ),
    (
        "stable-structured-error-taxonomy",
        all(
            marker in RPC
            for marker in [
                "InvalidRequest",
                "UnauthorizedCaller",
                "PermissionDenied",
                "ConfirmationRequired",
                "SessionNotFound",
                "SessionConflict",
                "CallerMismatch",
                "PlanMismatch",
                "NoChange",
                "UnsupportedPlatform",
                "ExecutionFailed",
                "pub fn payload(&self) -> TweakRpcErrorPayload",
            ]
        ),
    ),
    (
        "no-cli-mutation-bypass",
        "neo-tweak-executor" not in cli_dependencies
        and "neo_tweaks_apply" not in CLI_MAIN
        and "TweakRpcService" not in CLI_MAIN
        and "TweakExecutorCapability" not in CLI_MAIN,
    ),
    (
        "frozen-boundary-and-adversarial-proof",
        required_regressions.issubset(regressions)
        and all(marker in DECISION for marker in decision_markers)
        and "Every lane is blocking" in REVIEW
        and "mod rpc;" in LIB,
    ),
]

if len(checks) != 20 or len({name for name, _ in checks}) != 20:
    raise SystemExit("Phase 12 review definition must contain exactly 20 unique lanes")

failed = []
for index, (name, passed) in enumerate(checks, 1):
    print(f"{index:02d}. {'PASS' if passed else 'FAIL'} - {name}")
    if not passed:
        failed.append(name)

if failed:
    raise SystemExit("Phase 12 static review failed: " + ", ".join(failed))

print("PHASE 12 STATIC REVIEW PASS: 20/20")

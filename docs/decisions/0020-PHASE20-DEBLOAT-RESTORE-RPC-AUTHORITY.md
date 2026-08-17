# Decision 0020 — Phase 20 Debloat Restore MCP/RPC Authority

**Status:** FROZEN FOR IMPLEMENTATION PROOF
**Product domain:** eighth bounded Debloat child
**Authority:** typed MCP/RPC orchestration for one trusted Phase 19 history record into the proven Phase 18 current-user restore executor

## Decision

Phase 20 issues the first external Debloat restore mutation authority without widening the frozen Phase 18 AppX executor or weakening Phase 19 history provenance.

The primary path is:

```text
approved MCP caller
    -> typed MCP tool contract
    -> authenticated workstation/local RPC transport
    -> DebloatRestoreRpcService
    -> Phase 19 trusted by-ID history selection
    -> Phase 17 fresh restore readiness + inverse transaction
    -> Phase 18 execution-session validation
    -> internal DebloatRestoreExecutorCapability issuance
    -> proven Phase 18 executor
```

The MCP-facing tool names are frozen as:

- `neo_debloat_restore_prepare`
- `neo_debloat_restore_apply`

The corresponding workstation/local RPC method names are frozen as:

- `neo.debloat.restore.prepare`
- `neo.debloat.restore.apply`

GitHub, shell commands, and the public Neo CLI are not mutation transports for this phase.

## Transport trust boundary

`DebloatRestoreRpcContext` is trusted server-side context. It is created only after the workstation/RPC host authenticates the connection and resolves the principal and granted permission scopes.

Caller identity, caller kind, service instance identity, and granted scopes are not deserializable request fields. Untrusted request JSON may supply only operation inputs. Exact principal policy remains mandatory; caller kind alone is not authority.

The scopes are:

- `neo.debloat.restore.prepare` — read-only trusted history selection and fresh restore preparation;
- `neo.debloat.restore.low-risk.apply` — confirmed execution of exactly one prepared low-risk Phase 18 restore action.

Prepare permission is checked before history access or fresh AppX inventory capture. Apply permission is checked before capability issuance.

## Prepare protocol

`neo.debloat.restore.prepare` accepts only:

- request ID;
- mission ID;
- Phase 19 history record ID as its canonical lowercase 64-hex string.

Neo parses the record ID into `DebloatHistoryRecordId`; caller-supplied receipt JSON and filesystem paths remain invalid authority.

Preparation then:

1. loads the trusted Phase 19 record by typed ID;
2. performs Phase 17 fresh Windows inventory/readiness checks through `prepare_windows_restore_by_id`;
3. constructs the Phase 18 execution session through `prepare_debloat_restore_execution`;
4. verifies the exact one-action low-risk restore transaction shape already enforced by Phase 18;
5. computes the exact transaction fingerprint;
6. returns the record ID, receipt fingerprint, exact package/action identity, session ID, transaction fingerprint, and `confirmation_required: true`.

Preparation performs no AppX mutation and issues no capability to the caller.

Neo permits one outstanding prepared Debloat restore session per caller. A newer successful prepare for that caller invalidates the older unconfirmed prepared session before the new one is stored.

## Apply protocol

`neo.debloat.restore.apply` requires:

- request ID;
- exact prepared session ID;
- exact transaction fingerprint;
- the complete exact approved action-ID set;
- explicit `confirmed: true`;
- the same authenticated caller principal that prepared the session;
- `neo.debloat.restore.low-risk.apply` permission.

The prepared Phase 18 plan contains exactly one action, so the approved set must equal that one action ID exactly. Partial, extra, duplicate, empty, or oversized approval input fails closed before capability issuance.

A normal Phase 4 `TransactionAuthorization` is derived from the prepared fingerprint and exact action set. Phase 20 does not invent an alternative authorization model.

## Capability issuance

`DebloatRestoreExecutorCapability` remains opaque and has no public constructor.

Phase 20 adds only a crate-private `for_rpc()` constructor inside `neo-debloat-restore-executor`. `DebloatRestoreRpcService`, implemented in the same crate, is the only new production caller of that constructor.

The capability is issued only after:

- exact caller policy;
- exact apply scope;
- caller continuity;
- explicit confirmation;
- exact session identity;
- exact transaction fingerprint;
- exact action-set equality.

The capability does not widen Phase 18. The executor still performs its own fresh baseline/route checks immediately before authorization and immediately before mutation.

## Single-use and replay authority

A successfully validated apply request consumes its prepared service session before authorization or machine mutation begins.

Session IDs are server-derived from:

- trusted service instance ID;
- checked monotonic sequence;
- exact transaction fingerprint.

Client request IDs are correlation metadata only.

Therefore:

- successful apply cannot be replayed;
- execution/verification/rollback failure leaves no reusable mutation token;
- retry after execution failure requires a fresh prepare and therefore fresh Phase 17 state capture;
- a newer prepare invalidates the caller's older unconfirmed session;
- service sequence exhaustion fails closed instead of wrapping.

Validation failures before capability issuance, such as missing confirmation or fingerprint mismatch, do not consume the session so the same authenticated caller can correct the request.

## Existing Phase 17–19 laws remain authoritative

Phase 20 reuses rather than replaces:

- Phase 19 typed record IDs and trusted store-owned selection;
- Phase 19 no arbitrary receipt/path import boundary;
- Phase 17 fresh current/provisioned conflict detection;
- Phase 17 exact locally staged main/dependency route;
- Phase 17 restore-time baseline capture;
- Phase 4 transaction fingerprint and authorization;
- Phase 18 two fresh execution-state checks;
- Phase 18 shared Debloat serialization boundary;
- Phase 18 exact full-name registration and post-state verification;
- Phase 18 restore-time-baseline rollback.

No Store/network acquisition, broader package resolution, or second restore backend is added.

## Error contract

MCP/RPC callers receive stable structured error classes:

- `invalid_request`;
- `unauthorized_caller`;
- `permission_denied`;
- `confirmation_required`;
- `session_not_found`;
- `service_state_exhausted`;
- `caller_mismatch`;
- `plan_mismatch`;
- `history_unavailable`;
- `restore_not_ready`;
- `unsupported_platform`;
- `execution_failed`.

Detailed internal errors remain operator evidence; callers do not infer policy from free-form host strings.

## CLI and GUI boundary

Phase 20 does not add a public mutation CLI command. The CLI remains diagnostic/manual tooling.

A future GUI must call the same typed service/core authority path rather than constructing `DebloatRestoreExecutorCapability` or invoking the executor directly.

Hunter, Oracle, GUI, and other approved TTG callers converge on this same MCP/RPC trust boundary.

## Proof boundary

Deterministic unit/adversarial proof must cover:

- trusted-context separation from request JSON;
- exact caller policy and scopes;
- valid/invalid history record IDs;
- by-ID store selection rather than caller path/JSON;
- one-outstanding-session-per-caller replacement;
- service-instance/monotonic session identity;
- explicit confirmation;
- fingerprint equality;
- exact single action-set approval;
- caller continuity;
- single-use/replay resistance;
- retry-after-failure requiring fresh prepare;
- error classification;
- capability issuance remaining crate-private;
- no CLI mutation surface.

Windows CI must compile the real Phase 18 mutation path. Live destructive AppX restore against a sacrificial machine/profile is still not claimed by this phase unless separately performed and recorded.

## Deliberate limits

Phase 20 does not add:

- public CLI restore mutation;
- a second GUI mutation backend;
- Store/network/vendor package acquisition;
- package download/staging;
- provisioned-image restore;
- all-users restore;
- batch restore;
- history editing/deletion/import;
- plugin dependency;
- caller-supplied identity/scopes/service instance identity;
- generic capability factories;
- live destructive AppX restore proof.

Those remain separately gated.

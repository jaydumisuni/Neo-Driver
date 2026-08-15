# Decision 0012 — Phase 12 MCP/RPC Tweak Authority

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Product domain:** first external authority boundary over the proven Phase 11 tweak executor  
**Authority:** typed MCP/RPC orchestration for exactly the three Phase 11 low-risk reversible tweaks

## Decision

Phase 12 exposes Neo's first machine-changing orchestration contract through MCP/RPC without widening Phase 11 Registry authority.

The primary path is:

```text
approved MCP caller
    -> typed MCP tool contract
    -> authenticated workstation/local RPC transport
    -> TweakRpcService
    -> internal TweakExecutorCapability issuance
    -> proven Phase 11 transaction/executor
```

The MCP-facing tool names are frozen as:

- `neo_tweaks_prepare`
- `neo_tweaks_apply`

The corresponding workstation/local RPC method names are frozen as:

- `neo.tweaks.prepare`
- `neo.tweaks.apply`

This follows the existing TTG control-plane convention: MCP is the caller-facing tool surface and RPC is the direct machine/service invocation boundary. Neo does not use GitHub or a public shell command as its interactive mutation transport.

## Transport trust boundary

`TweakRpcContext` is trusted server-side context. The MCP/RPC transport or embedding host must construct it only after authenticating the connection and resolving the caller principal and granted permission scopes.

Caller identity and permission scopes are deliberately **not** fields of `TweakRpcPrepareRequest` or `TweakRpcApplyRequest`. Untrusted clients must not be allowed to self-assert a principal or grant themselves scopes by placing those values in request JSON. The caller-kind, caller, and trusted context types therefore do not implement Serde `Deserialize`; only the untrusted request contracts are deserializable.

Phase 12 policy authorizes an exact set of `TweakRpcCaller` identities. Caller kind alone is not authority; the exact principal must also be present in `TweakRpcPolicy`.

`TweakRpcService` is also constructed with a trusted **service instance ID** supplied by the embedding workstation/RPC host. It is not a client request field. The host must use a fresh instance identity whenever it creates a new authority-service instance.

The current scopes are:

- `neo.tweaks.prepare` — permits read-only preparation and live baseline capture;
- `neo.tweaks.low-risk.apply` — permits confirmed execution of the exact prepared low-risk Phase 11 plan.

A transport may grant both scopes to an approved owner/admin caller, but Neo validates the required scope independently for each operation. The apply scope is mechanically limited to low-risk actions because the three Phase 11 Registry specifications are fixed at `RiskLevel::Low` and transaction action risk is derived from those private fixed specifications, not from caller-supplied request data.

## Two-step authority protocol

### Prepare

`neo.tweaks.prepare` receives only:

- request ID;
- mission ID;
- selected curated tweak IDs.

The selected-action array must contain at least one item and cannot exceed the three Phase 11 curated tweak IDs. This cardinality gate runs before live Registry reads, so an authenticated caller cannot turn an oversized request into unbounded host work.

Neo validates caller policy and the prepare scope **before** live Registry state is read.

Preparation then reuses Phase 11 to:

- validate the catalogue and exact curated semantic bindings;
- read actual current state;
- reject unsupported Registry state;
- assess selected tweaks;
- capture exact rollback baselines;
- construct the Phase 4 transaction;
- compute its exact fingerprint.

The response exposes the prepared action IDs, desired DWORDs, captured baselines, transaction fingerprint, session ID, and the fact that confirmation is still required. Preparation never issues mutation authority to the caller.

Neo permits **one outstanding prepared tweak plan per caller**. A newer successful prepare for the same caller invalidates that caller's older unconfirmed prepared session before the new session is stored. This bounds pending authority and makes the newest reviewed plan the only plan that caller can subsequently confirm.

### Apply

`neo.tweaks.apply` requires:

- the exact prepared session ID;
- the exact transaction fingerprint returned by prepare;
- the complete exact set of approved action IDs;
- explicit `confirmed: true`;
- the same authenticated caller principal that prepared the session;
- the `neo.tweaks.low-risk.apply` permission scope.

The approved-action array is also capped at the same three-action Phase 11 ceiling before Neo constructs its approval set. Any caller change, missing confirmation, fingerprint mismatch, oversized/partial/extra action set, absent session, or missing permission fails closed before capability issuance.

The service then derives a normal Phase 4 `TransactionAuthorization`. It does not create a second authorization model inside the executor.

## Capability issuance

`TweakExecutorCapability` remains opaque and has no public constructor.

Phase 12 adds only a crate-private `for_rpc()` issuance path. That constructor is reachable by `TweakRpcService` inside `neo-tweak-executor`; it is not exported as a public capability factory and is not available to the CLI or arbitrary external Rust callers.

The service issues the capability only after its policy, scope, caller-continuity, confirmation, fingerprint, and exact-action checks succeed.

The capability does not broaden what Phase 11 can do. The raw Registry host, curated Registry bindings, and write/rollback implementation remain private and unchanged.

## Single-use and replay authority

A successfully validated apply request consumes the prepared service session before authorization or machine mutation begins.

Every prepared session ID is server-derived from:

- the trusted service instance ID;
- a checked **monotonic session sequence** owned by that service instance;
- the exact Phase 4 transaction fingerprint.

The client request ID is correlation metadata only and is not the uniqueness source for mutation authority. Sequence exhaustion fails closed with `service_state_exhausted` rather than wrapping.

Therefore:

- an apply request cannot be replayed after success;
- a backend/verification/rollback failure cannot leave a reusable mutation token;
- an identical re-prepare cannot recreate the previous session ID inside the same service instance;
- a newer prepare from the same caller invalidates that caller's older unconfirmed prepared plan;
- retry after execution failure requires a fresh prepare, which recaptures actual current state and receives a fresh service session identity even when the deterministic transaction fingerprint is unchanged.

Validation failures that happen before capability issuance, such as missing confirmation or fingerprint mismatch, do not consume the prepared session so the same caller can correct the request without silently recapturing state. Phase 11 baseline-drift checks still run again at authorization and apply.

## Existing Phase 11 laws remain authoritative

Phase 12 reuses rather than replaces:

- the exact three curated HKCU DWORD bindings;
- certified evidence requirements;
- actual pre-state capture;
- Phase 4 transaction fingerprints and `TransactionAuthorization`;
- same-session mutex `Local\\THETECHGUY.NeoDriver.TweakExecutor.v1`;
- second baseline-drift check immediately before apply;
- direct Windows Registry APIs;
- fresh post-write verification;
- complete rollback-attempt recording and exact baseline restoration.

No new Registry key, value, operation, shell path, or mutation backend is added by Phase 12.

## Error contract

MCP/RPC callers receive stable structured error classes rather than parsing arbitrary host strings:

- `invalid_request`;
- `unauthorized_caller`;
- `permission_denied`;
- `confirmation_required`;
- `session_not_found`;
- `session_conflict`;
- `service_state_exhausted`;
- `caller_mismatch`;
- `plan_mismatch`;
- `no_change`;
- `unsupported_platform`;
- `execution_failed`.

Detailed execution errors may still be retained for operator evidence, but the RPC classification remains deterministic.

## CLI and GUI boundary

Phase 12 does not add a public mutation CLI command. The CLI remains diagnostic/manual tooling and existing read-only/validation surfaces remain intact.

A future Neo GUI should invoke the same typed service/core authority path rather than implementing a second mutation backend. Hunter, Oracle, GUI, and other approved TTG callers must converge on the MCP/RPC service boundary.

## Proof boundary

Phase 12 unit/adversarial proof uses a deterministic fake host for mutation behavior and proves policy, scope, caller continuity, trusted-context separation, request cardinality, confirmation, exact fingerprint/action binding, service-instance sequencing, stale-request rejection, single-use authority, and error classification.

Windows CI compiles the real Phase 12 path through the existing Phase 11 Windows executor. Phase 12 does **not** claim that CI or ATHENA has performed a live Registry mutation. Live attached-machine mutation remains a separate proof obligation before broader deployment claims.

Likewise, this phase defines Neo's typed service contract; wiring a specific remote MCP deployment to a workstation transport is an integration layer outside the Registry executor itself and must preserve this exact trust boundary.

## Deliberate limits

Phase 12 does not add:

- new tweak IDs or opposite/inverse operations;
- arbitrary Registry editing;
- services, AppX/debloat, Windows Features, scheduled tasks, Explorer restart, BCD, Test Signing, or security mutation;
- public CLI mutation;
- an independent GUI mutation backend;
- caller-supplied identity, permission scopes, or service instance identity;
- reusable bearer mutation tokens;
- GitHub as an interactive execution transport;
- live Registry mutation proof;
- runtime-executor or driverstore MCP/RPC authority issuance.

Those remain separately gated.

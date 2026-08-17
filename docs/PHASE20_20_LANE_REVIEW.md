# Phase 20 — Twenty-Lane Review

**Scope:** typed MCP/RPC authority from one trusted Phase 19 Debloat history record into the existing Phase 18 current-user AppX restore executor.

**Freeze rule:** all 20 lanes must reconcile to PASS before implementation freeze. A named gap is not PASS.

1. **Authority continuity** — only the typed RPC service may bridge Phase 19 trusted selection into Phase 18 capability issuance.
2. **Trusted transport context** — caller identity and scopes are server-side context and cannot be supplied by request JSON.
3. **Exact caller policy** — caller kind alone grants nothing; exact approved principal membership is required.
4. **Prepare permission** — `neo.debloat.restore.prepare` is checked before history access or live inventory capture.
5. **Apply permission** — `neo.debloat.restore.low-risk.apply` is checked before capability issuance.
6. **Trusted record identity** — prepare accepts only canonical Phase 19 record-ID text and never caller paths/receipt JSON.
7. **Fresh readiness** — every prepare reloads the store record and reruns Phase 17 fresh Windows readiness/inventory authority.
8. **Phase 18 shape continuity** — prepared state is revalidated through `prepare_debloat_restore_execution`; no second executor-plan format exists.
9. **Plan fingerprint binding** — response/apply authority is bound to the exact Phase 4 transaction fingerprint.
10. **Explicit confirmation** — apply requires `confirmed: true` after prepare.
11. **Exact action approval** — the approval set must equal the exact one Phase 18 restore action with no extras/partials/duplicates.
12. **Caller continuity** — only the authenticated principal that prepared the session may apply it.
13. **Bounded outstanding authority** — at most one unconfirmed restore session exists per caller; newer successful prepare invalidates older authority.
14. **Monotonic service sessions** — session identity derives from trusted service-instance ID + checked monotonic sequence + fingerprint; exhaustion fails closed.
15. **Single-use/replay resistance** — a validated apply consumes the pending session before authorization/mutation; failure does not leave a reusable token.
16. **Capability opacity** — `DebloatRestoreExecutorCapability` has no public constructor; only crate-private RPC issuance is added.
17. **Executor non-widening** — Phase 18 fresh baseline/route checks, mutex, exact full-name registration, verification, and rollback remain unchanged.
18. **Structured errors** — stable RPC error classes distinguish request/policy/history/readiness/platform/execution failures without granting authority through strings.
19. **No bypass surfaces** — no public mutation CLI, GUI backend, shell transport, plugin dependency, Store/network acquisition, batch/all-users/provisioned restore.
20. **Adversarial + integration proof** — fake-host unit proof covers policy/session/replay/fingerprint/action/caller failure cases; Ubuntu/Windows CI compile/test the full workspace and Windows compiles the real mutation path; Builder proves the final EXE. Live destructive restore remains separately unclaimed unless explicitly performed.

## Expected owning-layer changes

- `neo-debloat-restore-executor` gains the Phase 20 RPC service and crate-private `for_rpc()` capability constructor.
- `neo-debloat-restore-executor` may depend on `neo-debloat-history-store`; Phase 19 does not depend on the restore executor, so the dependency graph remains acyclic.
- CI gains a Phase 20 static-review lane and focused RPC test lane.
- No CLI mutation command is added.

## Acceptance

PASS requires all 20 obligations above, no unresolved review thread, exact-head CI on Ubuntu + Windows, physical Builder Windows artifact/runtime-smoke proof, merge with an expected-head guard, merged-main CI, then a separate one-file `docs/IMPLEMENTATION_STATUS.md` closeout.

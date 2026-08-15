# Phase 12 — MCP/RPC Tweak Authority 20-Lane Review

Phase 12 is the first external orchestration authority over Neo's proven Phase 11 three-tweak executor. It does not widen Registry scope. Every lane is blocking.

Canonical architecture alignment: `docs/NEO_DRIVER_MASTER_PLAN.md` section 25 is reconciled to the MCP/RPC-first control plane. GUI, CLI, Hunter, Oracle, and remote-control surfaces must converge on the same Neo core/service authority rather than creating independent mutation logic.

1. **Protocol identity** — MCP tool names and RPC method names are explicit, versioned, and frozen.
2. **Trusted transport-context separation** — caller kind, principal and granted scopes are trusted server context, are not request fields, and are not client-deserializable contracts.
3. **Exact caller policy** — authority is tied to an exact caller kind + principal allow-list entry.
4. **Prepare permission** — live preparation requires `neo.tweaks.prepare` before any host read.
5. **Mechanically low-risk apply permission** — mutation requires `neo.tweaks.low-risk.apply`, and every reachable Phase 11 specification is privately fixed at `RiskLevel::Low` with transaction risk derived from that fixed specification.
6. **Bounded request/service validation** — IDs/scopes/service-instance identity are bounded and control-character-free; repeated list values fail closed; untrusted prepare/apply action arrays cannot exceed the three curated Phase 11 actions.
7. **Curated-only preparation** — Phase 12 delegates tweak semantics to the exact Phase 11 catalogue/binding checks.
8. **Actual baseline evidence** — prepare returns the actual captured baseline for each changed action.
9. **Transaction fingerprint exposure** — prepare returns the exact Phase 4 plan fingerprint used for later authority.
10. **Explicit confirmation** — apply fails closed unless the caller explicitly confirms the prepared plan.
11. **Caller continuity** — only the exact principal that prepared the session can apply it.
12. **Fingerprint continuity** — apply fingerprint must equal the prepared transaction fingerprint.
13. **Action-set continuity** — approved IDs must equal the full exact prepared action set; partial or extra authority is rejected.
14. **Phase 4 authorization reuse** — RPC builds the existing `TransactionAuthorization`; no parallel transaction law is invented.
15. **Opaque capability issuance** — `TweakExecutorCapability` keeps no public constructor; RPC issuance is crate-private only.
16. **Executor reuse** — production apply calls the existing Phase 11 authorize/apply methods and therefore retains mutex, drift, verification and rollback behavior.
17. **Single-use + stale-replay boundary** — validated apply consumes authority before executor issuance; server instance identity + checked monotonic sequence make session IDs non-repeatable inside an instance; a newer prepare invalidates that caller's older outstanding plan.
18. **Stable error taxonomy** — caller, permission, confirmation, plan/session, service-state, platform, no-change and execution failures map to typed RPC error codes.
19. **No CLI mutation bypass** — Phase 12 adds no CLI dependency or public tweak-apply command.
20. **Frozen boundary + adversarial proof** — Decision 0012 and regressions prove unauthorized, unscoped, oversized, unconfirmed, mismatched, stale/replayed and failed-execution cases without live Registry writes.

## Required proof

Before merge, the exact implementation head must pass on Ubuntu and Windows:

- Phase 1–12 static review gates;
- Cargo lockfile integrity;
- `cargo fmt --all -- --check`;
- locked workspace type/build proof;
- Clippy with warnings denied;
- complete workspace unit/adversarial tests;
- inherited Windows read-only System X-Ray/state proof;
- all applicable inherited CLI fixtures.

A green test run proves the frozen implementation; it does not create permission to broaden Phase 12 scope.

## Not claimed

Phase 12 does not claim live Registry mutation on a real workstation, broad tweak authority, runtime/driver MCP mutation, or public CLI/GUI mutation. Those require separate authority and proof.

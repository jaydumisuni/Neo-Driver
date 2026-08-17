# Phase 21 — Repair & Windows Features 20-Lane Review

**Formation:** SUBSTANTIAL / 20 distinct obligations

The Phase 21 candidate may freeze only when every lane is PASS and no named gap remains. Source proof precedes packaging proof.

1. **Master-plan continuity** — Repair/Windows Features implementation matches the frozen Phase 21 decision without silently absorbing Windows Update, networking, Winget, AppX, PnP, arbitrary DISM package/capability, or GUI mutation scope.
2. **Platform and trusted executable boundary** — real servicing execution/RPC/session internals are Windows-only while deterministic fake-host tests remain portable; Windows directory comes from the trusted API and production launches exact System32 DISM/SFC binaries, never PATH/shell authority.
3. **Fixed-command surface** — only frozen DISM/SFC operations and exact feature identities are constructible; no arbitrary argument/script/executable adapter exists; every mutating DISM invocation carries `/NoRestart` so Neo retains reboot/resume authority.
4. **Elevation truth** — error 740 and NUL-separated SFC administration failures become explicit elevation-required/unavailable evidence, never false healthy/disabled/absent state.
5. **Bounded command evidence** — stdout/stderr are bounded at UTF-8 boundaries; start error, exit status and truncation remain explicit.
6. **Component-store parsing** — healthy/repairable/unrepairable/unavailable outcomes are deterministic and unknown output fails closed.
7. **System-file parsing** — SFC healthy/integrity-violations/unavailable outcomes are deterministic; `/verifyonly` and `/scannow` semantics stay distinct.
8. **Feature catalogue identity** — exactly six fixed feature IDs exist; feature state is enabled/disabled/pending/removed/unavailable and unsupported/absent evidence never becomes disabled by assumption.
9. **Read-only probe separation** — combined inspection may compose all evidence, but `repair inspect` probes only component-store/SFC health and `repair features` probes only the six fixed optional features; neither executes mutation.
10. **Repair transaction truth** — RestoreHealth/SFC repair use irreversible Repair actions with fresh pre/post diagnosis and no fake rollback claim.
11. **Feature transaction truth** — one selected supported feature uses exact desired state, stable Enabled/Disabled captured baseline, reversible inverse, and no `/Remove`.
12. **Freshness/drift proof** — executor performs fresh pre-mutation observation, rejects baseline/route drift, and requires fresh postcondition observation rather than trusting process exit alone.
13. **Servicing reboot semantics** — exit `3010` is successful-with-reboot; pending feature state or 3010 both preserve an explicit reboot/resume obligation even when the immediate feature state already equals target; DISM is never allowed to seize restart control from Neo.
14. **Write-ahead mutation state** — `Applying` is durably persisted before Windows mutation can begin so restart recovery can reconcile without blindly repeating an irreversible operation.
15. **Durable session ownership** — resume state is append-only beneath Neo-owned `NeoData/sessions`, no-follow constrained, caller-bound, strictly decoded with unknown fields rejected, an `AlreadyExists` racing opener never claims creation/cleanup ownership, and version publication is exclusive/no-replace.
16. **MCP/RPC trust and confirmation continuity** — trusted caller/scopes are transport-owned; authorization happens before machine evidence; prepare/apply binds caller, exact action ID, plan fingerprint, confirmation and irreversible acknowledgement where required.
17. **Replay/resume continuity** — resume is bound to exact caller, persisted version, plan fingerprint and action identity; Windows apply/resume is serialized across processes through the bounded Phase 21 servicing mutex before version read or mutation; stale, replayed, failed-execution and exhausted-state requests fail closed.
18. **CLI/core separation** — CLI exposes diagnosis only and contains no direct capability issuer, executor, raw DISM/SFC mutation, apply/resume RPC method, or trust-context bypass.
19. **Regression/CI continuity** — Phase 1–20 gates remain active; Phase 21 adds its own static review, deterministic `neo-repair` test proof and Windows read-only source-run evidence.
20. **Adversarial + source-first acceptance** — malformed IDs/output, unsupported features, injected persisted/request fields, elevation failure, 3010 reboot, command failure, baseline drift, session-directory races, replay and crash recovery are challenged before freeze; no Builder/package loop is used to discover source defects.

## Freeze boundary

No named failure, unresolved external-review finding, skipped applicable proof, stale-base mismatch, or unreviewed authority widening may be converted into PASS. Live destructive DISM/SFC or feature mutation is not claimed without a separately approved sacrificial/elevated lane.

## Proof order

1. format/type/Clippy/deterministic source tests;
2. direct read-only source execution on Windows;
3. independent review and exact-SHA freeze;
4. one final Builder/package proof only after source confidence is earned;
5. merge and merged-main proof.

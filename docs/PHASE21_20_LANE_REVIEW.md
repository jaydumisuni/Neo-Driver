# Phase 21 — Repair & Windows Features 20-Lane Review

**Formation:** SUBSTANTIAL / 20 distinct obligations

The Phase 21 candidate may freeze only when every lane is PASS and no named gap remains.

1. **Master-plan continuity** — Repair/Windows Features implementation matches Sections 14, 15, 20, 21, 25, 29, 32–35 without silently broadening scope.
2. **Trusted executable identity** — Windows directory comes from trusted API; production launches exact System32 DISM/SFC binaries, never PATH/shell authority.
3. **Fixed-command surface** — only frozen DISM/SFC operations and fixed feature identities are constructible; no arbitrary argument/script adapter exists.
4. **Elevation truth** — error 740/unelevated evidence becomes explicit elevation-required/unavailable state, never false healthy/disabled/absent state.
5. **Bounded command evidence** — stdout/stderr are bounded and exact command evidence is retained without unbounded durable payloads.
6. **Component-store parsing** — healthy/repairable/unrepairable/unavailable outcomes are deterministic and unknown output fails closed.
7. **System-file parsing** — SFC healthy/integrity-violations/unavailable outcomes are deterministic; `/verifyonly` and `/scannow` semantics stay distinct.
8. **Feature catalogue identity** — exact fixed feature IDs, labels, risk/admin/reboot metadata, direct-Serde validation, duplicate rejection.
9. **Feature state parsing** — enabled/disabled/pending/removed/unavailable are exact; absent/unsupported editions never become disabled by assumption.
10. **Read-only inspection proof** — inspection surfaces contain no repair/feature mutation command and synthetic + Windows live lanes prove the read path.
11. **Repair transaction binding** — RestoreHealth/SFC repair use irreversible Repair actions with fresh pre/post diagnosis and no fake rollback claim.
12. **Feature transaction binding** — one selected supported feature, exact desired state, captured Enabled/Disabled baseline, reversible inverse, no `/Remove`.
13. **Reboot semantics** — pending feature states create explicit reboot/resume obligations; pending is never reported as final enabled/disabled success.
14. **Freshness/drift** — executor performs fresh pre-mutation observation and rejects baseline/route drift before changing the machine.
15. **MCP/RPC trust boundary** — trusted caller context/scopes, authorization-before-evidence, strict request decoding, no client caller/scope injection.
16. **Confirmation continuity** — prepare/apply binds session, caller, exact action IDs, plan fingerprint and explicit acknowledgement; mismatch fails closed.
17. **Replay/session safety** — caller-bound single-use sessions; stale, replayed, failed-execution and sequence-exhaustion cases are deterministic.
18. **CLI/core separation** — CLI exposes diagnosis/planning only and contains no direct capability issuer, executor, raw DISM/SFC mutation, or RPC trust bypass.
19. **Regression/CI continuity** — Phase 1–20 static/proof gates remain active; Phase 21 static, focused unit and Windows read-only proof are additive.
20. **Adversarial + three-person acceptance** — malformed IDs/output, unsupported features, injection attempts, elevation failure, command failure, drift, replay and recovery boundaries are challenged; beginner/standard/expert explanations remain derivable from one core truth.

## Freeze boundary

No named failure, unresolved external-review finding, skipped applicable proof, or stale-base mismatch may be converted into PASS. Live destructive DISM/SFC or feature mutation is not claimed without a separately approved sacrificial/elevated lane.

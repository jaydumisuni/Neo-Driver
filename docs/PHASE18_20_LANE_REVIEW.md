# Phase 18 — 20-Lane Engineering Review

**Scope:** internal transaction-bound post-success current-user AppX restore executor  
**Mutation authority:** opaque internal capability only  
**Input authority:** exactly one constructor-produced Phase 17 prepared inverse transaction

1. Phase 18 is a separate restore-executor crate so `neo-debloat-history -> neo-debloat-executor` never becomes a dependency cycle.
2. Execution accepts only revision-1 `:phase17-debloat-restore-current-user` state with exactly one `BaselineCaptured` action and exact checkpoint/plan fingerprint continuity.
3. The accepted action remains exactly `Debloat + LOW + Repair + Certified + explicit confirmation + non-admin + no reboot + reversible + non-default`.
4. Phase 17 receipt fingerprint, exact main full name, and direct dependency count must remain bound into the prepared action evidence.
5. The prepared restore route must equal the exact captured main/dependency identities; main package-kind/dependency shape remains frozen by Phase 17.
6. Mutation requires an opaque `DebloatRestoreExecutorCapability` with no public constructor and no CLI/GUI/plugin/MCP/RPC issuer in Phase 18.
7. Phase 18 re-probes the exact current-user restore-time baseline before authorization.
8. Phase 18 independently re-probes the same restore-time baseline immediately before mutation.
9. Main/dependency side-by-side current-user conflicts fail closed independently of inventory ordering.
10. The exact staged/provisioned main and every exact staged/provisioned dependency are re-proven on both pre-write checks.
11. Windows restore uses only `RegisterPackageByFullNameAsync` with the exact Phase 17 main/dependency full names.
12. Native restore requires terminal `AsyncStatus::Completed` plus a non-error `DeploymentResult`; API success alone is not mission success.
13. Forward verification observes and proves the exact restored main **and every direct dependency**, not only the main package.
14. Restore `machine_changed` evidence is derived from observed target-vs-restore-time-baseline state and is conservative when observation is unavailable.
15. Native restore failure after machine change routes through Phase 4 rollback and preserves both restore + rollback failure causes when recovery also fails.
16. Rollback removes the restored main, preserves dependencies already present at the Phase 17 baseline, and removes only dependencies introduced from an `Absent` baseline.
17. Dependency rollback runs in reverse recorded order and Phase 4 `MatchesBaseline` verification must prove every target before `RolledBack`.
18. Phase 16 removal and Phase 18 restore share the same named Debloat mutex so Neo cannot run the two AppX mutators concurrently.
19. Deterministic regressions cover success, both baseline-drift windows, staged-route drift, order-independent side-by-side conflict, partial native failure, failed postcondition, post-write observation loss, no-change semantics, rollback failure composition, dependency preservation/removal, and opaque capability.
20. No Store/network acquisition, package staging, provisioned/all-users/batch mutation, public restore write surface, plugin dependency, persistent history-store authority, or MCP/RPC restore capability issuance exists in Phase 18.

A Phase 18 PASS requires all twenty lanes simultaneously. Public history/restore authority and trusted persistent receipt selection remain separately gated.

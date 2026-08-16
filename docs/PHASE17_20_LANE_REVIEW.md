# Phase 17 — 20-Lane Engineering Review

**Scope:** completed-removal history + post-success restore readiness  
**Mutation authority:** none  
**Input authority:** completed Phase 16 execution + fresh exact AppX inventory

1. Receipt creation accepts only a Phase 16 session whose checkpoint is exactly `Complete`.
2. Execution-plan and checkpoint transaction fingerprints must agree before history is emitted.
3. Receipt main/dependency identities are recovered from the captured Phase 16 baseline, not reconstructed from package names.
4. Receipt restore route must exactly match the captured main full/family identity and ordered dependency full-name list.
5. Receipt schema is explicitly versioned and durable JSON deserialization revalidates the complete source checkpoint.
6. Receipt id is deterministic and bound to the source transaction id.
7. Receipt SHA-256 fingerprint covers all authority-bearing receipt fields except the fingerprint itself and is rechecked on deserialization.
8. Receipt fingerprint is documented as tamper detection only, not a signature or caller-authentication mechanism.
9. Restore preparation always performs a fresh exact AppX inventory validation; a historical receipt is never assumed current.
10. Exact original main package already present returns `AlreadyRestored` and produces no inverse transaction.
11. A different current-user main package name/family identity blocks deterministic old-version restore.
12. Exact staged/provisioned main full-name + family identity must still exist and retain the receipt dependency shape.
13. Every original direct dependency must still have an exact staged/provisioned full-name + family identity.
14. Different current-user dependency version/name/family conflicts block preparation.
15. Fresh restore-time baseline captures main as absent and every dependency as exactly present or absent at preparation time.
16. The new inverse transaction requires exact original main + direct-dependency identities as forward postconditions.
17. The new inverse transaction uses `MatchesBaseline` rollback verification over every restore-time snapshot target.
18. Restore action is explicit-confirmation, never preselected, inherits original risk, and starts at `BaselineCaptured` with `machine_changes = false`.
19. Deterministic regressions cover receipt round-trip, tamper rejection, non-complete rejection, exact restore readiness, dependency baseline preservation, already-restored, main-version conflict, missing staged main/dependency, dependency-version conflict, and byte-for-byte non-mutation.
20. No AppX registration/removal executor, public restore write CLI/GUI, plugin dependency, or MCP/RPC restore authority exists in Phase 17.

A Phase 17 PASS requires all twenty lanes simultaneously. Post-success restore execution requires a separately frozen phase.

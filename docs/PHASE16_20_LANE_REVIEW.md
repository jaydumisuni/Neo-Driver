# Phase 16 — 20-Lane Engineering Review

**Scope:** bounded internal current-user AppX removal + deterministic local rollback executor  
**Mutation authority:** opaque internal capability only; no public issuer  
**Input authority:** exactly one frozen Phase 15 `DebloatPreparedTransaction`

1. Phase 15 remains the only preparation authority; Phase 16 cannot construct arbitrary AppX targets.
2. Execution accepts exactly one prepared action at `BaselineCaptured` with fingerprint continuity.
3. Main package full/family identity and direct dependency full names are copied from the constructor-owned Phase 15 restore route and cannot be replaced externally.
4. Mutation methods require `DebloatExecutorCapability`; the capability has no public constructor.
5. No CLI, GUI, plugin, MCP, or RPC surface can issue the Phase 16 capability or invoke debloat mutation.
6. Actual current-user AppX state is re-read and compared with every captured baseline target before authorization.
7. Authorization is delegated to the Phase 4 `TransactionAuthorization` contract, including exact plan fingerprint and action coverage.
8. Apply is serialized by the fixed same-session `Local\\THETECHGUY.NeoDriver.DebloatExecutor.v1` mutex.
9. Under that mutex, every captured main/dependency baseline is re-read again immediately before mutation; drift blocks the write.
10. Forward mutation uses native `PackageManager::RemovePackageAsync` with only the exact prepared package full name.
11. Phase 16 does not deprovision/provision all users, batch packages, force shutdown, preserve app data, or invoke shell/PowerShell mutation.
12. Native deployment completion checks `DeploymentResult.ExtendedErrorCode` and surfaces deployment error text rather than treating API dispatch as success.
13. After the removal call, Neo freshly observes the main package and all captured direct-dependency targets.
14. If post-write observation is unavailable, `machine_changed` is conservatively true; uncertainty cannot erase rollback obligation.
15. The single Debloat action records API outcome and observed `machine_changed` separately in the Phase 4 checkpoint.
16. Forward completion is determined by Phase 4 postcondition verification; API success alone cannot complete the transaction.
17. Rollback uses native `RegisterPackageByFullNameAsync` with the exact main full name, exact captured dependency full names, and `DeploymentOptions::None`.
18. Rollback records its outcome and then freshly verifies every captured AppX target with Phase 4 `MatchesBaseline` predicates.
19. Deterministic fake-host proof covers clean removal, both baseline-drift windows, partial failure after mutation, postcondition failure after dependency mutation, no-change success, rollback failure, and dependency-preserving removal.
20. CI compiles the real Windows backend but performs no live package mutation; Phase 16 has no public write CLI/GUI/MCP/RPC/plugin authority.

A Phase 16 PASS requires all twenty lanes to remain true simultaneously. Any future expansion of scope—provisioning, all-users mutation, Store/network restoration, batch removal, public capability issuance, or live-machine mutation proof—requires a separately frozen phase.

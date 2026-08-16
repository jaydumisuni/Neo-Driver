# Decision 0018 — Transaction-Bound Post-Success AppX Restore Executor

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Product domain:** sixth bounded Debloat child  
**Authority:** internal capability-gated execution of exactly one Phase 17 prepared current-user restore transaction

## Decision

Phase 18 executes the inverse transaction that Phase 17 deliberately stopped before mutating. It does not reopen the completed Phase 16 removal transaction and does not infer a restore plan from a package name. Its only accepted input is the constructor-produced Phase 17 `DebloatRestorePreparedTransaction` whose fresh restore-time baseline and exact local staged identities have already been proven.

Because `neo-debloat-history` already depends on the Phase 16 removal executor in order to create completed-removal receipts, Phase 18 is a separate `neo-debloat-restore-executor` crate rather than adding a dependency from `neo-debloat-executor` back to history and creating an architectural cycle.

## Accepted authority

Phase 18 accepts only the frozen Phase 17 inverse transaction shape:

- transaction revision `1`;
- transaction id ending `:phase17-debloat-restore-current-user`;
- exactly one action;
- checkpoint exactly `BaselineCaptured`;
- checkpoint/plan fingerprint continuity;
- `Debloat` action id `restore:{debloat_id}`;
- `LOW` risk;
- `Repair` recommendation;
- `Certified` verdict;
- not selected by default;
- explicit confirmation required;
- no administrator requirement;
- no reboot requirement;
- rollback available;
- receipt fingerprint evidence exactly matching the prepared state;
- exact restore main full name and direct dependency count bound into action evidence.

The prepared main/dependency identities and restore route must also agree exactly, including main package-kind flags and the direct dependency identity list already frozen by Phase 17.

## Capability boundary

The real mutation methods require `DebloatRestoreExecutorCapability`, an opaque type with no public constructor. Phase 18 does not issue that capability through CLI, GUI, plugin, MCP, or RPC surfaces.

The capability requirement is intentionally separate from the Phase 4 `TransactionAuthorization`: the authorization proves explicit user approval of the exact plan fingerprint, while the opaque capability proves that the internal trusted service path is permitted to invoke the executor at all.

## Two fresh pre-write checks

The Phase 17 prepared transaction is not treated as permanently current. Phase 18 re-probes immediately before recording authority and again immediately before native mutation.

Both checks require:

1. every current-user AppX target still exactly matches the Phase 17 restore-time baseline;
2. no side-by-side main package with the same name/family has appeared;
3. no side-by-side dependency version/name/family conflict has appeared, even if the expected exact dependency is also present;
4. the exact staged/provisioned main identity still exists and retains the exact main package shape;
5. every exact staged/provisioned direct dependency identity still exists.

Any drift fails before mutation.

## Native restore path

On Windows, Phase 18 uses `PackageManager::RegisterPackageByFullNameAsync` with:

- the exact original staged main package full name from Phase 17; and
- the exact ordered direct dependency full-name list from Phase 17.

It validates terminal `AsyncStatus::Completed` and the `DeploymentResult` extended error code. It performs no Store, network, vendor-download, package-staging, deprovisioning, provisioning, all-users, or batch action.

Phase 18 shares the same named local Debloat execution mutex as Phase 16 so removal and restore cannot mutate AppX state concurrently through Neo's two internal Debloat executors.

## Forward verification

A successful native API result is not completion. Phase 18 re-reads the exact native AppX inventory and gives the Phase 4 checkpoint observations for **every** target in the inverse transaction:

- the restored main must equal the exact original `ExactPackageIdentity`;
- every direct dependency must equal its exact original `ExactPackageDependency` identity.

If post-write observation is unavailable, or any required exact identity is not proven, the transaction cannot complete.

## Failure recovery

Rollback is against the **Phase 17 restore-time baseline**, never the historical pre-removal baseline.

For the one restore action, Phase 18 recovery:

1. removes the restored main package if it is present;
2. preserves every direct dependency that was already exact-and-present in the Phase 17 restore-time baseline;
3. removes only a dependency whose Phase 17 restore-time baseline was `Absent` and which the restore attempt introduced;
4. applies dependency cleanup in reverse recorded order;
5. records one Phase 4 rollback result for the one restore action;
6. re-probes every target;
7. requires every rollback predicate to satisfy `MatchesBaseline` before reaching `RolledBack`.

If the native restore fails after changing state, rollback is attempted. If both restore and rollback fail, both causes remain in the returned diagnostic. If the API reports success but no target changed, Phase 4 does not invent rollback work; failed forward verification reaches `Failed` because there is no proven changed action to recover.

## Concurrency and observation loss

The same Debloat mutex serializes Phase 16 removal and Phase 18 restore mutations. A post-write inventory failure is conservative: Phase 18 treats the state as changed when it cannot prove otherwise, routes through Phase 4 recovery, and reports the observation failure only after the restore-time baseline has been proven restored.

## Deliberate limits

Phase 18 does not implement or prove:

- a public restore/undo button;
- GUI or CLI restore mutation;
- persistent on-disk history-store authority, ACLs, or trusted receipt selection;
- Store/network/vendor restore acquisition;
- staging packages that are no longer locally provisioned;
- provisioned-image reprovision/deprovision;
- all-users restore;
- batch restore;
- plugin dependency;
- MCP/RPC Debloat restore capability issuance;
- live destructive restore against a sacrificial Windows user profile.

Those remain separately gated. Phase 18 proves the internal exact local staged execution/recovery mechanism only.

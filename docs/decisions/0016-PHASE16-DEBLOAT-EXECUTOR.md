# Decision 0016 — Phase 16 Transaction-Bound Current-User AppX Executor

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Product domain:** fourth bounded child of Debloat  
**Authority:** internal capability-gated current-user AppX removal with deterministic local rollback

## Decision

Phase 16 introduces the first Debloat mutator. It consumes only the exact read/identity/restore/transaction state already prepared by Phase 15 and advances that same Phase 4 checkpoint through authorization, apply, verification, and when required rollback.

It does not create a generic package manager, arbitrary AppX command surface, batch remover, or public mutation API.

## Input and identity authority

Phase 16 accepts exactly one `DebloatPreparedTransaction` whose checkpoint is still `BaselineCaptured`. The prepared transaction must contain exactly one action and exactly one prepared step, and its checkpoint fingerprint must still equal the frozen Phase 15 transaction fingerprint.

The executor copies the constructor-owned Phase 15 main package identity and restore route into an externally read-only execution plan. The forward target is the exact current-user package `FullName`. The rollback route is the exact staged main `FullName` plus the exact direct dependency `FullName` list captured by Phase 15.

No Phase 16 public field accepts a package name, family name, dependency name, command, script, deployment option, user SID, or scope supplied independently of the Phase 15 prepared object.

## Baseline law

Authorization is not accepted merely because Phase 15 captured a baseline earlier. Phase 16 re-reads native current-user AppX inventory and compares every main/dependency target against the captured Phase 15 baseline before calling the shared Phase 4 authorization method.

Apply repeats that entire comparison under the executor mutex immediately before any removal call. Drift in either window fails closed without a Phase 16 write.

The main package baseline is compared using the exact serialized `ExactPackageIdentity`. Dependency baselines are compared using the exact serialized `ExactPackageDependency` triple used by Phase 15. This keeps fresh observation representation identical to the frozen rollback baseline rather than accidentally comparing different schemas.

## Forward mutation

The Windows backend uses `Windows.Management.Deployment.PackageManager` directly.

Forward mutation is exactly:

- `RemovePackageAsync(exact_main_package_full_name)`;
- current user only;
- no all-users operation;
- no provisioning mutation;
- no arbitrary `RemovalOptions`;
- no PowerShell, `cmd.exe`, shell, script, or plugin path.

`RemovePackageAsync` may also remove direct dependency registrations that Windows determines are no longer required. Phase 16 therefore observes every captured main/dependency target after the call; it does not assume only the main package changed.

Native async completion is awaited. The returned `DeploymentResult.ExtendedErrorCode` is checked and deployment `ErrorText` is surfaced when the extended result is a failure. Starting or awaiting the API call is not itself considered successful machine mutation.

## Transaction law

Phase 16 uses the Phase 4 transaction already created by Phase 15. It does not generate a new action or fingerprint.

After authorization, the executor opens `Applying`, asserts the single prepared Debloat action is pending, invokes removal, and records one `ApplyRecord` whose two important facts remain separate:

- API outcome (`Success` / `Failure`);
- observed `machine_changed`.

`machine_changed` is true when any freshly observed captured target differs from baseline. If post-write observation itself is unavailable, it is conservatively true because Neo cannot prove the mutation did not occur.

After a successful API result, Phase 4 postcondition verification determines completion. The required forward postcondition remains the Phase 15 main-package `Absent` predicate. If the API reports success but the postcondition is not proven, the checkpoint—not the API result—decides whether rollback is required.

## Rollback

When the shared checkpoint enters `RollingBack`, Phase 16 restores captured local registration using exactly:

- `RegisterPackageByFullNameAsync`;
- the exact staged main package `FullName`;
- the exact captured direct dependency `FullName` list;
- `DeploymentOptions::None`.

There is no Store, network, vendor download, manifest path, broad family-name registration, or arbitrary deployment option fallback in Phase 16.

After registration, Phase 16 records the rollback result and freshly observes every main/dependency rollback target. Phase 4 `verify_rollback` must prove all `MatchesBaseline` predicates before the session reaches `RolledBack`. Registration API success alone is not rollback proof.

If registration fails or restoration cannot be proven, the transaction remains failed/unresolved rather than claiming recovery.

## Serialization

Phase 16 serializes current-session Debloat apply operations with the fixed named mutex:

`Local\\THETECHGUY.NeoDriver.DebloatExecutor.v1`

The mutex is acquired before the second baseline-drift check and held through removal, post-write verification, and any rollback attempt. This prevents two Neo processes in the same Windows session from acting concurrently on independently prepared stale AppX baselines.

The mutex does not claim control over unrelated external installers/package managers or other Windows sessions. Fresh state observation and verification remain mandatory.

## Capability boundary

Public mutation methods require the opaque `DebloatExecutorCapability`. Its field is private and Phase 16 provides no public constructor or issuer.

The real Windows host and raw mutation trait remain crate-private. The capability is created only in crate tests for deterministic proof.

Phase 16 does **not** expose capability issuance through CLI, GUI, plugin, MCP, RPC, Hunter, or Oracle. A future authority/service phase may issue this capability only after its own permission, confirmation, replay, and caller-context contract is frozen and proven.

## Proof boundary

CI must compile and Clippy-check the real Windows `PackageManager` removal/registration implementation, including the exact `windows` 0.62 API signatures.

CI must **not** remove or register a real GitHub runner AppX package. Mutation semantics are proven with deterministic fake-host tests that exercise:

- successful exact removal;
- drift before authorization;
- drift after authorization but before mutation;
- API failure after actual mutation;
- dependency-only unexpected mutation causing forward verification failure and rollback;
- API success with no machine change;
- rollback registration failure;
- successful main-only removal while dependency remains.

This is an engineering proof of the executor and real backend compilation, not a claim that Neo has performed a live destructive AppX mutation on a test machine.

## Deliberate limits

Phase 16 does not implement or prove:

- provisioned-image deprovisioning or reprovisioning;
- all-users removal/registration;
- batch removal;
- overlapping multi-package dependency transactions;
- Store/network/vendor restoration;
- package download or staging;
- arbitrary deployment/removal options;
- package family-name fallback;
- live GitHub-runner AppX mutation;
- public CLI or GUI Debloat write actions;
- plugin dependency;
- MCP/RPC Debloat capability issuance or execution.

Those remain separately gated.

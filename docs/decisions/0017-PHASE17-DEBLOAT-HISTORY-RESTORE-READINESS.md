# Decision 0017 — Completed-Removal History and Post-Success Restore Readiness

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Product domain:** fifth bounded Debloat child  
**Authority:** read-only history receipt + fresh inverse transaction preparation; zero post-success restore mutation authority

## Decision

Phase 17 closes the history gap left deliberately open by Phase 16. Phase 16 can automatically roll back a failed or unverified removal, but a successful removal ends at `Complete` and the shared Phase 4 state machine has no `Complete -> RollingBack` transition. The master plan nevertheless requires Debloat history to offer a restore path when technically possible.

Phase 17 therefore does **not** reopen a completed Phase 16 transaction. It creates a durable completed-removal receipt and, later, may prepare a new inverse Phase 4 transaction after re-proving that the exact local staged restore route is still valid.

No AppX registration/removal occurs in Phase 17.

## Completed-removal receipt

A receipt may be created only from a Phase 16 `DebloatExecutionSession` whose checkpoint is exactly `Complete` and whose execution-plan/checkpoint transaction fingerprints still agree. Durable validation also requires the source transaction to remain revision `1` with the frozen `:phase15-debloat-current-user` identity, and exactly one `Debloat` action whose authority still matches the Phase 13→15 candidate law: `LOW` risk, `Recommended` or `OptionalComponent`, `Certified`, explicit-confirmation-required, non-admin, no-reboot, reversible, and not selected by default.

The receipt records:

- schema version `1`;
- deterministic receipt id bound to the source transaction id;
- source transaction id, mission id, and fingerprint;
- Debloat action id and package id;
- exact original current-user `ExactPackageIdentity` from the captured Phase 16 baseline;
- exact original direct `ExactPackageDependency` identities from that same baseline;
- exact local staged full-name restore route;
- the validated completed source `TransactionCheckpoint`;
- a SHA-256 receipt fingerprint over every authority-bearing receipt field except the fingerprint itself.

The receipt is intentionally serializable **and deserializable** because it is a durable history contract. Deserialization revalidates the embedded completed checkpoint, source transaction/fingerprint continuity, action identity, captured baseline identities, restore-route continuity, schema version, deterministic receipt id, and receipt fingerprint.

A receipt is history evidence. It is not by itself mutation authority.

## Restore-time re-probe

A historical receipt does not imply that restore remains safe later. Phase 17 must re-read the current native AppX inventory before preparing any inverse transaction.

Restore readiness requires all of the following:

1. the exact original main package is currently absent for the current user;
2. no different current-user package with the same package name or family has replaced it;
3. the exact original staged/provisioned main full-name + family identity still exists;
4. the staged main identity still has the same package-kind flags and direct dependency identity shape recorded by the receipt;
5. every exact original direct dependency still has a matching staged/provisioned full-name + family identity;
6. no different current-user dependency version/family identity conflicts with any receipt dependency.

If the exact main package is already registered, Phase 17 reports `AlreadyRestored` instead of preparing duplicate work.

Missing staged identity, changed identity shape, version/family conflicts, or ambiguous evidence fail closed.

## Fresh inverse transaction

When readiness is proven, Phase 17 creates a **new** Phase 4 transaction. It does not mutate or resume the completed removal transaction.

The restore transaction:

- contains exactly one Debloat restore action;
- is never selected by default;
- requires explicit confirmation;
- carries the original Debloat risk level;
- uses `Repair` recommendation + `Certified` evidence verdict only after exact local re-proof;
- snapshots the current restore-time state of the main package and every direct dependency;
- requires the original main exact identity as a forward postcondition;
- requires every original direct dependency identity as a forward postcondition;
- defines rollback verification as `MatchesBaseline` for every restore-time snapshot target;
- starts at `BaselineCaptured`;
- reports `machine_changes = false` because Phase 17 prepares state only.

The restore-time baseline is intentionally distinct from the original pre-removal baseline. A future restore executor must return to the **restore-time** baseline if restore fails, because dependencies may have changed between the original removal and a later restore request.

## Dependency semantics

The original main and direct dependency identities are retained from the completed Phase 16 baseline, not reconstructed from package names later.

At restore preparation time, a direct dependency may be:

- already present as the exact original full/family identity; or
- absent but still available as the exact staged/provisioned identity.

Both states are captured explicitly in the fresh restore transaction baseline. A different current dependency version/name/family conflicts with deterministic restoration and blocks preparation.

Phase 17 does not claim that dependency reconciliation is already executable. That belongs to the future restore executor.

## Persistence and tamper detection

The receipt schema is versioned because history is intentionally durable across process lifetimes.

The receipt fingerprint detects accidental or untrusted modification of receipt fields. It is **not** a cryptographic signature or caller-authentication mechanism. A future persistent history store/restore authority must separately define trusted storage provenance and caller authorization before a receipt can lead to mutation.

## Windows path

`prepare_windows_restore_from_receipt` uses the already-proven Phase 15 native `PackageManager` exact AppX inventory scanner, then runs the same platform-neutral readiness/transaction preparation logic.

It performs no registration/removal operation.

## Deliberate limits

Phase 17 does not implement or prove:

- post-success AppX registration execution;
- rollback execution for a failed restore attempt;
- removal of dependencies added by a future restore attempt;
- Store/network/vendor package acquisition;
- package staging/download;
- provisioned-image deprovision/reprovision;
- all-users restore;
- batch restore;
- persistent on-disk history-store authority or ACL design;
- public GUI/CLI restore mutation;
- plugin dependency;
- MCP/RPC Debloat restore capability issuance or execution.

Those remain separately gated.

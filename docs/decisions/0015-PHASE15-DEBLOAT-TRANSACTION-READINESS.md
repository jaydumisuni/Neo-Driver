# Decision 0015 — Phase 15 Debloat Transaction Readiness

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Product domain:** third bounded child of the frozen Debloat domain  
**Authority:** read-only exact AppX identity, rollback-readiness, and transaction preparation only

## Decision

Phase 15 does not remove an AppX package. It closes the evidence gap between Phase 14 logical presence and a future controlled executor by proving the exact package identity, exact direct dependency set, deterministic local restore readiness, and Phase 4 transaction/checkpoint binding before any mutation capability can exist.

The flow is:

```text
Phase 13 certified removal candidate
        +
Phase 14 current-user/provisioned presence evidence
        +
Windows PackageManager exact identity inventory
        ↓
exact package full/family identity + direct dependencies
        ↓
matching provisioned staged identity for main + every dependency
        ↓
Phase 4 Debloat transaction + captured baseline checkpoint
        ↓
NO APPLY CAPABILITY IN PHASE 15
```

## Scope

Phase 15 intentionally prepares exactly one `CurrentUser` removal candidate at a time. `Provisioned` and `CurrentUserAndProvisioned` mutation planning remain blocked because deprovisioning is a separate administrator operation with a different restore and verification contract. Batch removal also remains blocked because selected apps can share framework/dependency rollback targets and Phase 4 correctly rejects overlapping state targets.

Only a Phase 13 `RemovalCandidate` may enter Phase 15. Protected, profile-preserved, policy-blocked, review-only, absent, uncertified, higher-risk, or unavailable-evidence items cannot be converted into a transaction plan here.

## Exact Windows identity boundary

Phase 15 adds a native read-only `Windows.Management.Deployment.PackageManager` inventory in Rust. It records for current-user and provisioned packages:

- package `Name`;
- package `FullName`;
- package `FamilyName`;
- framework/resource/bundle/optional classification flags;
- exact direct dependency names/full names/family names for current-user packages.

Catalogue IDs do not become WinRT method names, commands, scripts, paths, or executable text. Matching remains case-insensitive in Rust. Duplicate exact full names, missing identities, ambiguous package-name matches, resource/framework main candidates, or disagreement with Phase 14 presence fail closed.

## Restore readiness law

Phase 13 restore metadata is descriptive until Phase 15 proves an executable local rollback route. Phase 15 therefore accepts only `RestoreMethod::ProvisionedImage` for prepared mutation authority and additionally requires:

- the selected current-user package to have exactly one exact identity;
- an exact matching provisioned package with the same FullName and FamilyName;
- every direct dependency to have an exact matching provisioned FullName and FamilyName.

Store IDs and vendor-source metadata remain useful recovery information but are not treated as deterministic local rollback authority. Phase 15 does not perform Store/network acquisition.

This readiness contract is designed for the future native PackageManager executor: current-user removal is keyed by package FullName, while current-user re-registration can be keyed by the same staged package FullName plus dependency package FullNames. Phase 15 only proves and records those identities; it does not invoke either operation.

## Transaction law

A prepared item becomes one Phase 4 `ActionKind::Debloat` transaction action with:

- explicit confirmation required;
- the exact current-user package FullName as the main `AppxPackage` state target;
- every exact direct dependency FullName as an additional rollback state target;
- exact serialized baseline identities captured into the Phase 4 checkpoint;
- required postcondition that the main current-user package becomes absent;
- reversible rollback metadata requiring every captured target to match its baseline after restoration;
- transaction fingerprint binding the exact transaction targets, postconditions, and rollback obligations.

The captured baseline values are checkpoint state, not inputs to the transaction-plan fingerprint. Phase 15 therefore claims both exact baseline capture and plan-fingerprint continuity, not a fingerprint over the baseline payload itself.

`DebloatPreparedTransaction` is constructor-owned. Its assessment, prepared steps, transaction plan, checkpoint, and machine-change marker are crate-visible only; external callers receive immutable getters. This prevents callers from replacing restore-route, transaction, or checkpoint state while reusing the same prepared object and plan fingerprint.

The checkpoint stops at `BaselineCaptured`. No authorization, apply record, verification result, rollback execution, or capability issuance occurs in Phase 15.

## Proof boundary

Phase 15 proves:

- native PackageManager current-user exact identity enumeration on real Windows CI;
- native PackageManager provisioned exact identity enumeration on real Windows CI;
- exact package/dependency validation and case-insensitive matching;
- Phase 14-vs-native drift rejection;
- single-item/current-user-only authority boundary;
- resource/framework main-candidate rejection;
- Store/vendor metadata not misrepresented as executable rollback;
- exact provisioned twin required for main and every direct dependency;
- Phase 4 `ActionKind::Debloat` transaction creation;
- constructor-owned externally read-only prepared authority state;
- exact baseline capture and transaction-plan fingerprint continuity;
- deterministic fixture proof on Ubuntu and Windows;
- byte-for-byte fixture-tree equality around the Windows live inventory proof;
- continued absence of AppX mutation capability.

Phase 15 does **not** prove or implement:

- `PackageManager.RemovePackageAsync` execution;
- `RegisterPackageByFullNameAsync` execution;
- deprovision/provision execution;
- Store/network restore;
- batch debloat transactions;
- all-users package mutation;
- live package mutation on CI or a donor machine;
- public CLI/GUI write actions;
- plugin dependency;
- MCP/RPC debloat capability issuance or execution.

Those remain separately gated.

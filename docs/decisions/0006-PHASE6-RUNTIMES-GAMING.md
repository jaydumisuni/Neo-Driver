# Decision 0006 — Phase 6 Runtimes & Gaming Assessment

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Scope:** deterministic runtime/gaming evidence, package binding, recommendation and CLI assessment  
**Mutation authority:** none in this phase

## Decision

Phase 6 introduces a shared `neo-runtime` engine that consumes normalized runtime evidence, the existing validated Neo package catalogue, and an explicit runtime component-to-package binding policy.

The engine produces reviewable runtime/gaming recommendations and `neo-core::PlannedAction` objects but does not execute them.

## Manual authority

Every surfaced runtime item remains individually user-selectable.

A profile baseline may be selected by default only when:

- normalized evidence proves it is missing, broken or partial;
- exactly one validated compatible runtime package is bound;
- architecture and Windows-build applicability pass;
- the action remains confirmation-gated.

Preselection is not force installation. The user may deselect the item before any later execution phase.

Optional components are never selected by default.

Unknown evidence, an unbound package, a non-runtime binding, or multiple compatible packages does not become install authority.

## Baseline policy

The Phase 6 modern baseline is:

- Visual C++ 2015+ x86;
- Visual C++ 2015+ x64;
- DirectX End-User Runtimes (June 2010) for Fresh Windows and Gaming profiles.

DirectX June 2010 remains deselectable. It is not treated as a replacement for modern Windows DirectX.

## Optional runtime/gaming policy

The runtime model includes explicit evidence states for:

- .NET Framework 3.5;
- .NET Framework 4.x;
- modern .NET Runtime;
- .NET Desktop Runtime;
- Python;
- WebView2;
- XNA Framework 4.0 Refresh;
- OpenAL;
- PhysX;
- PhysX Legacy;
- DirectPlay.

Gaming-only legacy components remain optional unless a later verified game/application dependency proves they are required.

Python remains optional in Technician and Developer profiles. A partial/broken Python state is represented distinctly so later work can prefer repair over blindly installing another interpreter.

## Catalogue binding

Package bytes, source, SHA-256, redistribution state, Windows applicability, dependency/conflict metadata and package kind remain owned by `neo-catalogue`.

`RuntimePolicy` adds only the typed relationship between a `RuntimeComponent` and a Neo package ID. A binding must resolve to an existing `PackageKind::Runtime` package.

The planner does not infer runtime meaning from package names or IDs.

## Fail-closed candidate selection

A runtime package is eligible only when its Windows architecture and build range include the normalized host evidence.

If no compatible package is bound, Neo reports investigation/no action authority.

If more than one compatible package is bound, Neo reports ambiguity/no action authority. Phase 6 does not invent a version-ranking rule for generic runtime installers.

## Readiness

A profile is not baseline-ready while any baseline component is missing, broken, partial or unknown.

Optional missing components do not make the profile baseline-unready.

## CLI boundary

Phase 6 adds read-only:

- `neo runtimes ...`
- `neo gaming ...`

These commands may emit normalized recommendations and planned actions. They do not download, install, repair, enable Windows features, reboot, or advance a transaction.

## Deliberately blocked

This phase does not add:

- runtime downloads;
- runtime installer execution;
- MSI/EXE/Winget execution;
- .NET 3.5 or DirectPlay Windows-feature mutation;
- security/BCD mutation;
- transaction advancement from CLI;
- rollback claims for runtime installation;
- GUI write actions.

Runtime execution requires a later bounded executor with explicit capture, apply, verification, reboot/resume, and rollback/recovery semantics appropriate to installer packages and Windows features.

The existing Phase 5 driver mutation backend remains internal pending live attached-device proof.

## Proof

Phase 6 must pass:

- Phase 1–6 deterministic static reviews;
- lockfile integrity;
- rustfmt;
- locked workspace build/type proof;
- Clippy with warnings denied;
- all workspace tests on Ubuntu and Windows;
- catalogue/matcher/transaction regression fixtures;
- dedicated runtime CLI fixture;
- dedicated gaming CLI fixture;
- external review with all correctness/security findings reconciled before merge.

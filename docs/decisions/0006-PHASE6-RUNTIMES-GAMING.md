# Decision 0006 — Phase 6 Runtimes & Gaming System X-Ray

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Scope:** deterministic runtime/gaming evidence collection, package binding, recommendation, readiness and CLI assessment  
**Mutation authority:** none in this phase

## Decision

Phase 6 introduces two model-free layers:

- `neo-runtime` — pure normalized runtime evidence, package binding, profile readiness and reviewable action planning;
- `neo-runtime-probe` — a read-only Windows System X-Ray adapter that reuses `neo-probe::CommandRunner` and preserves raw `CommandEvidence`.

The assessment engine consumes normalized runtime evidence, the existing validated Neo package catalogue, and an explicit runtime component-to-package binding policy. It produces reviewable runtime/gaming recommendations and `neo-core::PlannedAction` objects but does not execute them.

The System X-Ray adapter may collect evidence. It may not install, repair, download, enable, disable, reboot, or otherwise mutate Windows state.

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

DirectX June 2010 remains deselectable. It is not treated as a replacement for modern Windows DirectX capability.

The x64 baseline law is not silently projected onto 32-bit Windows. Until an architecture-specific requirement policy is frozen, x64 runtime applicability on an x86 host remains unknown rather than invented.

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

## System X-Ray evidence predicates

Phase 6 currently freezes these read-only Windows evidence paths:

- Visual C++ v14 x86/x64: Microsoft Visual Studio 14.0 VC Runtime registry `Version` evidence;
- .NET Framework 4.x: `HKLM\SOFTWARE\Microsoft\NET Framework Setup\NDP\v4\Full` `Release` evidence;
- modern .NET: `dotnet --list-runtimes`, distinguishing `Microsoft.NETCore.App` and `Microsoft.WindowsDesktop.App`;
- .NET Framework 3.5: read-only DISM `/Get-FeatureInfo /FeatureName:NetFx3 /English`;
- DirectPlay: read-only DISM `/Get-FeatureInfo /FeatureName:DirectPlay /English`;
- WebView2: Microsoft EdgeUpdate product GUID `{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}`, architecture-aware HKLM path plus HKCU path, with a non-empty/non-zero `pv` value;
- Python: `py -0p` compatibility listing plus `where.exe` discovery for `python.exe`, `py.exe` and `pip.exe`.

The Python probe deliberately does not invoke a bare `python`/`py` runtime. A PATH gap is `Partial`, not an excuse to install another interpreter. Lack of global command evidence remains `Unknown`, not `Missing`.

## Unproven predicates

Phase 6 deliberately reports `Unknown` for these components until their predicates are independently recovered and frozen:

- DirectX June 2010 side-by-side legacy completeness;
- XNA Framework 4.0 Refresh;
- OpenAL;
- PhysX;
- PhysX Legacy.

Modern DirectX capability and DirectX June 2010 legacy components remain separate evidence problems. Neo does not infer one from the other.

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

- `neo runtime-scan [--json]` — live Windows runtime System X-Ray with normalized state plus raw command evidence;
- `neo runtimes ...` — profile assessment from normalized evidence;
- `neo gaming ...` — Gaming profile assessment from normalized evidence.

These commands do not download, install, repair, enable Windows features, reboot, or advance a transaction.

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
- a detector-aware Phase 6 20-lane review that rejects runtime/feature mutation paths;
- lockfile integrity;
- rustfmt;
- locked workspace build/type proof;
- Clippy with warnings denied;
- all workspace tests on Ubuntu and Windows;
- Windows-only live `neo runtime-scan --json` execution;
- catalogue/matcher/transaction regression fixtures;
- dedicated runtime CLI fixture;
- dedicated gaming CLI fixture;
- external review with all correctness/security findings reconciled before merge.

A green assessment foundation is not by itself Phase 6 completion. Remaining master-plan runtime/gaming predicates and Gaming readiness lanes must be implemented or explicitly deferred with evidence before the phase can be marked complete.

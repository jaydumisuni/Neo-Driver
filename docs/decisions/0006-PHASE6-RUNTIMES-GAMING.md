# Decision 0006 — Phase 6 Runtimes & Gaming System X-Ray

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Scope:** deterministic runtime/gaming evidence collection, package binding, recommendation, readiness and CLI assessment  
**Mutation authority:** none in this phase

## Decision

Phase 6 introduces three model-free layers:

- `neo-runtime` — pure normalized runtime evidence, package binding, profile readiness and reviewable action planning;
- `neo-directx-legacy` — read-only completeness evidence for the documented DirectX End-User Runtimes (June 2010) side-by-side framework component set;
- `neo-runtime-probe` — a read-only Windows System X-Ray adapter that reuses `neo-probe::CommandRunner`, preserves raw `CommandEvidence`, and consumes the DirectX detector.

The assessment engine consumes normalized runtime evidence, the existing validated Neo package catalogue, and an explicit runtime component-to-package binding policy. It produces reviewable runtime/gaming recommendations and `neo-core::PlannedAction` objects but does not execute them.

The System X-Ray layers may collect evidence. They may not install, repair, download, enable, disable, reboot, or otherwise mutate Windows state.

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
- .NET Framework 4.x: `HKLM\SOFTWARE\Microsoft\NET Framework Setup\NDP\v4\Full` `Release` evidence, mapped through Microsoft's documented Release-key thresholds instead of presenting the numeric DWORD as a product version;
- modern .NET: `dotnet --list-runtimes`, distinguishing `Microsoft.NETCore.App` and `Microsoft.WindowsDesktop.App`;
- .NET Framework 3.5: read-only DISM `/Get-FeatureInfo /FeatureName:NetFx3 /English`;
- DirectPlay: read-only DISM `/Get-FeatureInfo /FeatureName:DirectPlay /English`;
- WebView2: Microsoft EdgeUpdate product GUID `{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}`, architecture-aware HKLM path plus HKCU path, with a non-empty/non-zero `pv` value;
- Python: `py -0p` compatibility listing plus `where.exe` discovery for `python.exe`, `py.exe` and `pip.exe`; displayed Python version is parsed from the launcher version token, not from an executable path;
- DirectX June 2010 legacy framework components: a deterministic file-set predicate generated from Microsoft's documented D3DCompiler/D3DCSX/D3DX/X3DAudio/XACT/XAPOFX/XAudio/XInput ranges.

The Python probe deliberately does not invoke a bare `python`/`py` runtime. A PATH gap is `Partial`, not an excuse to install another interpreter. Lack of global command evidence remains `Unknown`, not `Missing`.

A failed registry query is also `Unknown`; an exit code alone is not enough evidence to certify that a runtime is absent. Neo only reports `Missing` from a positive absence predicate, such as a successful Windows-feature query returning `Disabled`/payload removed or a deterministic accessible DirectX layout containing none of the expected legacy components.

DISM feature states `Enable Pending` and `Disable Pending` are classified `Partial` until reboot completion. Neo does not promote a pending state to installed/missing before Windows has finished the transition.

### DirectX legacy completeness semantics

`neo-directx-legacy` uses `GetWindowsDirectoryW` as the trusted Windows-directory authority rather than `%SystemRoot%` process environment state.

For x64 Windows, a 64-bit Neo process checks the documented framework-component filename set in both native `System32` and x86 `SysWOW64`. For x86 Windows it checks `System32`. ARM64 remains `Unknown` until an architecture-specific June 2010 layout is proven.

The detector classifies:

- `Installed` — every expected documented framework filename is present in every required architecture directory;
- `Partial` — at least one expected component exists but the set is incomplete;
- `Missing` — required directories are accessible but none of the expected components are present;
- `Unknown` — the layout cannot be safely interpreted or accessed.

This is a **presence/completeness predicate**, not a cryptographic or binary-health certification. It does not claim that every DLL is uncorrupted, correctly registered, or functionally healthy. Modern DirectX capability remains a separate evidence problem.

## Unproven predicates

Phase 6 deliberately reports `Unknown` for these components until their installation predicates are independently recovered and frozen:

- XNA Framework 4.0 Refresh;
- OpenAL;
- PhysX;
- PhysX Legacy.

Neo does not substitute uninstall display-name guesses or community folklore for a verified vendor predicate.

## Catalogue binding

Package bytes, source, SHA-256, redistribution state, Windows applicability, dependency/conflict metadata and package kind remain owned by `neo-catalogue`.

`RuntimePolicy` adds only the typed relationship between a `RuntimeComponent` and a Neo package ID. A binding must resolve to an existing `PackageKind::Runtime` package.

The planner does not infer runtime meaning from package names or IDs.

## Fail-closed candidate selection

A runtime package is eligible only when its Windows architecture and build range include the normalized host evidence.

If no compatible package is bound, Neo reports investigation/no action authority.

If more than one compatible package is bound, Neo reports ambiguity/no action authority. Phase 6 does not invent a version-ranking rule for generic runtime installers.

If the single compatible package has dependency or conflict edges, Phase 6 also reports investigation/no standalone action authority. Dependency/conflict closure must be planned and proven before a dependent package can become a Certified executable action.

## Independent review corrections

The pre-merge independent source challenge found and closed five fail-closed gaps:

1. Registry command exit code `1` could be interpreted as package absence even when the query itself failed. Failed registry evidence now remains `Unknown`.
2. DISM `Enable Pending` / `Disable Pending` could be collapsed into final enabled/disabled states. Both now remain `Partial` until reboot completion.
3. The .NET Framework `Release` DWORD could be displayed as though it were a product version. Neo now maps the Release value through documented .NET Framework version thresholds while preserving the raw Release evidence.
4. Python `detected_version` could contain the executable path emitted by `py -0p`. Neo now parses the launcher version token and keeps paths as evidence details.
5. A runtime package with dependency/conflict edges could become a standalone Certified action even though Phase 6 has no dependency-closure executor. Such packages now remain `Investigate` with no action authority until dependency closure is implemented.

These corrections passed Phase 1–6 static review, lock integrity, Windows compiler, Clippy with warnings denied, the complete unit suite, live Windows runtime scan, runtime fixture, and gaming fixture in pre-commit run `31684439927` before the temporary proof helpers self-cleaned and the correction was committed as `cb911bd72887e02d7d648294d793b9364c085d67`.

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
- external review disposition with all correctness/security findings reconciled before merge.

If the external reviewer is unavailable or rate-limited, Neo does not claim an external-review PASS; the disposition must record the limitation and zero unresolved review threads, while deterministic CI and the independent source challenge remain the active proof authorities.

A green System X-Ray/assessment foundation is not by itself the full Runtimes & Gaming execution phase. Runtime mutation and broader Gaming hardware/API readiness remain separate bounded work and must not be implied by this decision.
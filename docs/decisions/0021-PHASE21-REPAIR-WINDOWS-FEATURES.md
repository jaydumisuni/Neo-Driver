# Decision 0021 — Phase 21 Repair & Windows Features

**Status:** FROZEN FOR IMPLEMENTATION

## Why this phase exists

The master plan places **Repair & Windows Features** immediately after Debloat. Phases 13–20 completed the Debloat domain through trusted MCP/RPC restore authority. Phase 21 now opens the repair domain without bypassing Neo's existing evidence, transaction, confirmation, verification, and MCP/RPC laws.

Microsoft's supported Windows servicing surfaces distinguish diagnosis from repair: DISM `/Online /Cleanup-Image /CheckHealth` checks component-store health, `/RestoreHealth` performs repair, SFC `/verifyonly` verifies protected system files without repair, and `/scannow` performs repair. DISM also exposes online feature inspection and enable/disable operations. Phase 21 binds only those fixed Windows-owned operations; it does not create an arbitrary shell/script runner.

## Phase 21 bounded product surface

Phase 21 supports exactly these first-class repair/feature capabilities:

1. **Component Store**
   - inspect with fixed `dism.exe /Online /Cleanup-Image /CheckHealth /English`;
   - repair with fixed `dism.exe /Online /Cleanup-Image /RestoreHealth /English`;
   - no caller-supplied DISM source path, `/LimitAccess`, cleanup/reset-base, offline image, mount, package, or arbitrary arguments in this phase.
2. **Protected System Files**
   - inspect with fixed `sfc.exe /verifyonly`;
   - repair with fixed `sfc.exe /scannow`;
   - no caller-supplied file/offline directory arguments in this phase.
3. **Windows Optional Features**
   - inspect a fixed Neo catalogue by exact feature identity;
   - enable/disable one explicitly selected feature at a time;
   - never use DISM `/Remove`;
   - no caller-supplied raw feature name reaches the executor.

The initial fixed feature catalogue is:

- `NetFx3` — .NET Framework 3.5;
- `DirectPlay` — DirectPlay legacy component;
- `Microsoft-Hyper-V-All` — Hyper-V feature group;
- `Microsoft-Windows-Subsystem-Linux` — Windows Subsystem for Linux;
- `VirtualMachinePlatform` — Virtual Machine Platform;
- `Containers-DisposableClientVM` — Windows Sandbox.

A feature absent from the current Windows edition/build is reported as unavailable/unsupported. Neo does not manufacture support because the catalogue knows the feature name.

## Elevation law

ATHENA evidence proved that even DISM `/CheckHealth` and `/Get-FeatureInfo` return Windows error **740** in the unelevated Oracle terminal. Therefore:

- deep component-store and optional-feature inspection is an **elevated read authority**;
- mutation is elevated authority as well;
- an unelevated failure must become explicit `elevation_required`/Unavailable evidence, never Healthy/Disabled/Absent;
- Neo must not weaken UAC, create a hidden scheduled-task bypass, or silently self-elevate outside the approved service/elevation boundary.

SFC evidence follows the same conservative authority rule for this phase even if a particular host invocation appears to return output unelevated.

## Trusted executable path law

Production Windows execution must derive `%Windows%` through the trusted Windows API and invoke the exact System32 binaries:

- `<Windows>\System32\dism.exe`
- `<Windows>\System32\sfc.exe`

PATH lookup, current directory lookup, environment-controlled `%SystemRoot%`, `cmd.exe`, PowerShell script interpolation, and shell execution are not authority.

## Evidence model

Every diagnosis retains exact command evidence:

- trusted executable identity;
- exact fixed argument vector;
- exit code/start error;
- bounded stdout/stderr;
- parsed typed state;
- explicit uncertainty reason when parsing/authority fails.

Command output is bounded before durable/reporting use. Unknown output does not become a successful diagnosis.

## Repair state model

Component Store states:

- `Healthy`
- `Repairable`
- `Unrepairable`
- `Unavailable`

System File states:

- `Healthy`
- `IntegrityViolations`
- `Unavailable`

Feature states:

- `Enabled`
- `Disabled`
- `EnablePending`
- `DisablePending`
- `Removed`
- `Unavailable`

Unknown or contradictory output is `Unavailable`, never inferred.

## Transaction law

### DISM RestoreHealth / SFC scannow

These are Windows repair operations, not deterministic inverse edits. Phase 21 represents them as explicit **irreversible `ActionKind::Repair` transactions**:

- fresh diagnosis captured before apply;
- exact fixed operation bound into the plan;
- explicit user confirmation and admin requirement;
- no fake rollback claim;
- fresh post-operation diagnosis is mandatory;
- process exit code alone is never completion proof.

### Windows feature enable/disable

Feature changes use `ActionKind::WindowsFeature` and are reversible when the captured baseline is `Enabled` or `Disabled`:

- capture exact current feature state immediately before mutation;
- execute only the fixed catalogue identity and requested desired state;
- do not use `/Remove`;
- re-read the feature after execution;
- pending states become reboot obligations, not false completion;
- rollback restores the captured Enabled/Disabled baseline and verifies it.

A `Removed` or otherwise unavailable baseline cannot be promoted into normal reversible authority in this phase.

## MCP/RPC-first mutation authority

Machine-changing Phase 21 operations are not exposed by direct CLI executor calls.

Typed service flow:

`trusted caller context -> prepare -> explicit confirmation -> apply -> verify`

The service must preserve the Phase 12/20 laws:

- caller identity/scopes come only from trusted server/transport context;
- authorization occurs before privileged evidence lookup or mutation preparation;
- raw requests cannot self-assert caller/scopes;
- prepare is non-mutating;
- plan fingerprint + exact action continuity bind prepare to apply;
- sessions are caller-bound and single-use;
- capability issuance is crate-private;
- replay, stale/mismatched confirmation, and action drift fail closed;
- client-visible errors use stable codes/messages; internal OS output/details remain operator evidence.

Required scopes:

- `repair.inspect`
- `repair.apply`
- `windows_features.inspect`
- `windows_features.apply`

## CLI boundary

CLI may expose read-only diagnosis/plan surfaces such as `neo repair inspect` and `neo features inspect`. It must not construct executor capability or provide a raw DISM/SFC/feature mutation bypass.

## Explicitly deferred from Phase 21

These master-plan repair areas remain future typed children rather than arbitrary script recipes in this phase:

- Windows Update service/cache reset;
- networking reset/repair;
- Winget repair;
- AppX repair beyond the already-proven Debloat restore path;
- Driver Store/PnP repair beyond existing driver executor authority;
- system restore/recovery creation/application;
- offline DISM image servicing;
- caller-supplied repair source paths;
- batch feature mutation;
- arbitrary Windows feature names;
- live destructive repair/feature mutation on ATHENA unless a sacrificial/elevated proof lane is explicitly approved.

Deferral is not a hidden implementation gap: those operations require separate baseline, rollback/recovery, and privilege contracts and may not be smuggled through a generic command adapter.

## Source-first proof law

Phase 21 follows the owner-required development order:

1. recover contracts and official Windows command semantics;
2. source review/static proof;
3. run Rust tests and direct source-level Windows read-only probes;
4. where a UI appears later, run it directly and inspect/interact visually before packaging;
5. only after source confidence is earned may Builder perform one final packaging proof.

Builder is not the bug-discovery loop.

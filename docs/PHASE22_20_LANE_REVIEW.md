# Phase 22 — 20-Lane Engineering Review

Phase 22 is a read-only Driver Store / PnP repair assessment foundation. PASS requires every lane below to remain true on the exact candidate head.

| Lane | Obligation |
| --- | --- |
| 01 | Frozen master-plan continuity: Driver Store/PnP repair remains the governing Repair child. |
| 02 | Exact post-Phase-21 authority binding is recorded in Decision 0022. |
| 03 | `neo-driver-repair` is a separate crate; Phase 5 `neo-driverstore` authority is not rewritten. |
| 04 | Live collection uses only `DriverHost::inventory()` plus OEM-only `resolve_published_package()`; inbox/system INF bindings remain read evidence and never enter Phase 5 reversible package authority. |
| 05 | No Phase 22 production path calls driver staging, install, rollback, package deletion, re-enumeration, or enable/disable mutation. |
| 06 | Device instance identity is exact and case-insensitive duplicates fail closed. |
| 07 | Package evidence without an active published INF fails closed. |
| 08 | Exact package authority is OEM-only and has one crate-owned identity predicate shared by live capture and imported-evidence validation: both the active binding and imported/resolved `current_package` must be `oem<digits>.inf`, package published identity must equal the active published INF, imported exact-package evidence must carry a fully qualified Windows-root `System32\DriverStore\FileRepository\<package>\<original>.inf` path with exactly one root component before `System32`, and the reused Phase 5 live package resolver must decode `SetupGetInfDriverStoreLocationW` plus `SetupGetInfPublishedNameW` fail-closed rather than using lossy UTF-16 replacement. |
| 09 | Live Windows PnP status follows the actual `CM_Get_DevNode_Status` contract: a query failure aborts inventory; `NoProblem` requires `DN_HAS_PROBLEM` clear and a canonical zero initialized problem value; `Problem { code }` requires `DN_HAS_PROBLEM` set plus a nonzero `CM_PROB_*` code; flag/code contradictions fail closed. Only after that Windows normalization may Phase 22 inherit `None` as `NoProblem` and nonzero `Some(code)` as `Problem`; imported `Some(0)` or mismatched explicit PnP status remains invalid. |
| 10 | Healthy requires explicit normalized `NoProblem` plus exact active published INF and exact Driver Store package. |
| 11 | A non-disabled nonzero problem with exact current package is only a future reinstall candidate, not execution authority. |
| 12 | Driver selection is suggested only when Windows reports an actual PnP problem and no active binding exists; a no-problem/no-binding device does not manufacture a repair need. |
| 13 | Device Manager Code 22 is `Disabled` even when the generic disabled field is unavailable; contradictory explicit disabled evidence fails closed; no enable/re-enumeration authority exists. |
| 14 | The public Windows `DriverHost` inventory boundary uses the unified `SetupDiGetDevicePropertyW`/`DEVPKEY_Device_*` model for IDs, active INF, metadata, and upper/lower filters; `ERROR_NOT_FOUND` is the only property-absence result; buffer/type/byte-count failures fail closed; UTF-16 strings and string lists require exact canonical NUL termination with no trailing data; ClassGuid comes from enumerated `SP_DEVINFO_DATA`; hardware/compatible IDs are stable-deduplicated case-insensitively while preserving first-occurrence order; exact filter lists are retained as evidence; and filter presence alone is never blamed as a fault. The anti-drift source proof structurally requires exactly two `SetupDiGetDevicePropertyW` calls in the property helper, requires the sizing result to be bound and matched, and forbids a discarded sizing result. |
| 15 | Assessment order and evidence digest are deterministic across inventory ordering. |
| 16 | Report explicitly records `machine_changes = false`. |
| 17 | CLI surface is read-only: `neo repair drivers` supports live Windows evidence and validated fixture evidence with explicit normalized PnP status only; evidence-file read failures identify the exact failing input path. |
| 18 | Adversarial host proof panics all write-capable Phase 5 methods and Phase 22 still passes. |
| 19 | Normal Ubuntu/Windows CI runs Phase 22 static, focused, fixture, and Windows live read-only gates; focused/static proof binds the single shared OEM identity law, exact imported-package path law, strict live package-resolution UTF-16 law, documented `DN_HAS_PROBLEM` PnP status law, public SetupAPI absence/type/UTF-16 boundary, case-insensitive stable ID normalization, live-filter correction, and structural discarded-result detector against drift. |
| 20 | Windows Update/networking/Winget/AppX/restore-recovery and all Driver/PnP mutations remain explicitly deferred. |

No lane may be waived by a passing test elsewhere. A material failure reopens Phase 22 review.

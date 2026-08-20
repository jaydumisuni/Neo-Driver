# Phase 22 — 20-Lane Engineering Review

Phase 22 is a read-only Driver Store / PnP repair assessment foundation. PASS requires every lane below to remain true on the exact candidate head.

| Lane | Obligation |
| --- | --- |
| 01 | Frozen master-plan continuity: Driver Store/PnP repair remains the governing Repair child. |
| 02 | Exact post-Phase-21 authority binding is recorded in Decision 0022. |
| 03 | `neo-driver-repair` is a separate crate; Phase 5 `neo-driverstore` authority is not rewritten. |
| 04 | Live collection uses only `DriverHost::inventory()` and `resolve_published_package()`. |
| 05 | No Phase 22 production path calls driver staging, install, rollback, package deletion, re-enumeration, or enable/disable mutation. |
| 06 | Device instance identity is exact and case-insensitive duplicates fail closed. |
| 07 | Package evidence without an active published INF fails closed. |
| 08 | Package published identity must equal the active published INF. |
| 09 | Unknown PnP problem-code evidence never becomes Healthy. |
| 10 | Healthy requires problem code 0 plus exact active published INF and exact Driver Store package. |
| 11 | A nonzero problem with exact current package is only a future reinstall candidate, not execution authority. |
| 12 | A missing active binding routes back to existing matcher/catalogue selection authority. |
| 13 | Disabled evidence is preserved without adding enable or re-enumeration authority. |
| 14 | Upper/lower filters are retained as evidence and never automatically blamed. |
| 15 | Assessment order and evidence digest are deterministic across inventory ordering. |
| 16 | Report explicitly records `machine_changes = false`. |
| 17 | CLI surface is read-only: `neo repair drivers` supports live Windows evidence and validated fixture evidence only. |
| 18 | Adversarial host proof panics all write-capable Phase 5 methods and Phase 22 still passes. |
| 19 | Normal Ubuntu/Windows CI runs Phase 22 static, focused, fixture, and Windows live read-only gates. |
| 20 | Windows Update/networking/Winget/AppX/restore-recovery and all Driver/PnP mutations remain explicitly deferred. |

No lane may be waived by a passing test elsewhere. A material failure reopens Phase 22 review.

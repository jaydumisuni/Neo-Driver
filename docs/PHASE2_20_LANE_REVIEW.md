# Neo Driver — Phase 2 20-Lane Review

**Scope:** normalized device evidence + package catalogue contracts.  
**Mutation boundary:** read-only; no downloads, installs, driver binding changes, debloat/tweak execution, or security mutation.

## Findings corrected before first branch commit

1. Empty opaque device IDs could bypass a constructor through Serde; custom deserialization now enforces the invariant.
2. Technician components were too close to driver-family semantics; INF bundles and broader technician components are distinct.
3. A draft fixture escaped a hardware ID incorrectly; corrected before publication.
4. Applicability lists could contain duplicate IDs; ordered-ID validation now rejects duplicates while preserving order.
5. Security-state changes could omit a required reboot; validation now rejects that state.
6. Package provenance initially omitted signature/signer/catalogue evidence; per-INF signature evidence is explicit.
7. Driver bundles were initially flattened; each INF now has its own applicability and signature metadata.
8. Security requirements were booleans; they are now explicit `unchanged/enabled/disabled` target states.
9. A stale local CI copy would have regressed the proven Rust setup; Phase 2 is based on recovered `main`.
10. The workspace lacked a committed `Cargo.lock`; CI now fails and prints the generated lock until the exact lock is committed and re-proven.

No finding closes by documentation alone. Windows + Ubuntu static review, lockfile, formatting, build/type, Clippy, unit, and CLI fixture gates must all pass before merge.

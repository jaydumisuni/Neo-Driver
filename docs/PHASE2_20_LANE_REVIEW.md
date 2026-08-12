# Neo Driver — Phase 2 20-Lane Review

**Scope:** normalized device evidence + package catalogue contracts.  
**Mutation boundary:** read-only; no downloads, installs, driver binding changes, debloat/tweak execution, or security mutation.

## Findings corrected

1. Empty opaque device IDs could bypass a constructor through Serde; custom deserialization now enforces the invariant.
2. Technician components were too close to driver-family semantics; INF bundles and broader technician components are distinct.
3. A draft fixture escaped a hardware ID incorrectly; corrected before publication.
4. Applicability lists could contain duplicate IDs; ordered-ID validation now rejects duplicates while preserving order.
5. Security-state changes could omit a required reboot; validation now rejects that state.
6. Package provenance initially omitted signature/signer/catalogue evidence; per-INF signature evidence is explicit.
7. Driver bundles were initially flattened; each INF now has its own applicability and signature metadata.
8. Security requirements were booleans; they are now explicit `unchanged/enabled/disabled` target states.
9. A stale local CI copy would have regressed the proven Rust setup; Phase 2 is based on recovered `main`.
10. The workspace lacked a committed `Cargo.lock`; CI generated the exact lock, which was committed unchanged and then verified with `--locked`.
11. Rustfmt found catalogue layout drift; the exact formatter output was applied without behavior changes.
12. Clippy found `OpaqueDeviceId` imported in production scope although only tests used it; the import was moved into the test module.
13. Clippy found a manually implemented `Default` for `RequiredState`; it was replaced by the derived default with `Unchanged` explicitly marked as the default variant.
14. External review found dependency/conflict references could name packages absent from the catalogue; catalogue validation now resolves every relation against the complete package-ID set and has negative tests for both relation types.
15. External review found `DeviceRecord` / `DeviceInventory` deserialization could bypass explicit validation; deserialization now passes through validated wire types and regression tests reject duplicate filters and duplicate instance IDs at parse time.
16. External review found the lock guard could accept an ignored/untracked `Cargo.lock`; it now requires `git ls-files --error-unmatch Cargo.lock` before checking modification state.
17. External review found the Phase 2 anti-drift scan covered only selected sources; it now scans every workspace-member `Cargo.toml` and every Rust source before read-only/model-free boundary checks.

No warning or finding is suppressed with an allow-list escape hatch.

## Proof history

Implementation-code proof commit `a2c6453ea4fe5e20ea5ab4da7d7894530612c777`, GitHub Actions run `31613855813`, passed all configured Windows and Ubuntu gates. That proof was subsequently **reopened** when external review identified findings 14-17.

The corrected findings 14-17 require a new full Windows + Ubuntu proof before Phase 2 may close or merge.

Phase 2 remains read-only. Live attached-device behavior and machine mutation are not claimed.

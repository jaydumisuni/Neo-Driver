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

No warning or finding was suppressed with an allow-list escape hatch.

## Proof

Implementation-code proof commit: `a2c6453ea4fe5e20ea5ab4da7d7894530612c777`  
GitHub Actions run: `31613855813`

Both **Ubuntu** and **Windows** passed:

- Phase 1 20-lane static review;
- Phase 2 20-lane static review;
- committed Cargo.lock/current dependency graph guard;
- Rust formatting;
- locked workspace type/build check;
- Clippy with warnings denied;
- Rust unit tests;
- read-only `neo catalogue validate` synthetic fixture proof.

Phase 2 remains read-only. Live attached-device behavior and machine mutation are not claimed by this proof.

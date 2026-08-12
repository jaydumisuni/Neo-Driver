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
18. Re-proof found one rustfmt layout change in the unresolved-dependency regression test; the exact formatter output was applied without behavior changes.

No warning or finding was suppressed with an allow-list escape hatch.

## Proof

Corrected implementation head: `6d08ca5b4c048a833f1122b48fc042d9453bc556`  
GitHub Actions run: `31614831572`

Both **Ubuntu** and **Windows** passed:

- Phase 1 20-lane static review;
- strengthened Phase 2 20-lane static review;
- committed + Git-tracked Cargo.lock/current dependency graph guard;
- Rust formatting;
- locked workspace type/build check;
- Clippy with warnings denied;
- Rust unit tests, including the new deserialization and unresolved-reference regressions;
- read-only `neo catalogue validate` synthetic fixture proof.

The four external-review code threads were automatically marked resolved after the corrective commit. A later full incremental CodeRabbit review was rate-limited, so no additional full external-review PASS is claimed.

CodeRabbit also reports a generic **docstring coverage** pre-merge warning. This is a documentation-quality metric rather than a correctness/security finding and is not used as a Phase 2 functional proof gate; it remains visible rather than being represented as resolved by the code changes above.

Phase 2 remains read-only. Live attached-device behavior and machine mutation are not claimed by this proof.

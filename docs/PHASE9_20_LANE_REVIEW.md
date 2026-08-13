# Neo Driver — Phase 9 State Assessment 20-Lane Review

**Scope:** deterministic read-only state assessment.

1. Workspace membership is explicit.
2. Dependency surface stays platform-neutral.
3. The crate remains assessment-only.
4. Typed values have deterministic serialization.
5. Opaque state keys validate before use.
6. State keys are ASCII-only and compare case-insensitively through ASCII canonicalization.
7. Catalogue deserialization re-runs validation.
8. Evidence deserialization re-runs validation.
9. Duplicate catalogue IDs are rejected.
10. Duplicate catalogue keys are rejected.
11. Duplicate evidence keys are rejected.
12. High-risk defaults are rejected.
13. Non-Certified defaults are rejected.
14. Unsafe recommendation defaults are rejected.
15. Explicit selection is required.
16. Duplicate selections are rejected.
17. Unknown selections are rejected.
18. Rejected selections are rejected.
19. Missing or unavailable observations block assessment.
20. Reports distinguish matching from differing state without claiming a machine change.

## Review findings closed before freeze

- Independent PR-surface review and CodeRabbit identified that ASCII lowercase canonicalization did not define behavior for non-ASCII state keys. Phase 9 now rejects non-ASCII keys before canonicalization, and an explicit regression proves that boundary.
- CodeRabbit identified that the frozen review contract previously counted numbered lines without proving their exact sequence. The contract test now requires the parsed lane sequence to equal exactly `1..=20`.
- CodeRabbit identified that the read-only proof surface did not behaviorally demonstrate filesystem non-mutation. Phase 9 now has an isolated integration regression that launches both `neo-state-assess` subcommands against fixture JSON, snapshots the isolated fixture tree before and after, and requires identical paths, file bytes, directory/file shape, and modification timestamps. The Windows proof log explicitly records `state_assess_subcommands_leave_isolated_fixture_tree_unchanged ... ok`.
- The CI-covered Phase 9 Rust contract additionally freezes the production I/O boundary: the `neo-state-plan` production surface may contain exactly its two existing JSON `read_to_string` filesystem calls, no process surface, and the proof CLI handler may expose neither filesystem nor process authority. This is deliberately enforced in normal workspace unit proof rather than inferred from user-facing text.
- Two formatter-only child findings were closed after adding the behavioral and contract regressions. No behavior changed in those rustfmt corrections.
- CodeRabbit's docstring-coverage warning is a documentation-style metric, not a correctness or authority defect; no runtime or safety claim depends on that metric.

## Implementation-code proof

- Frozen implementation head: `6af5579a7e9f0db9f1fd44ce32dbc18d1541cc9b`.
- Authoritative proof run: `31715322010`.
- Ubuntu: **PASS** — inherited Phase 1–8 static reviews, Cargo lock integrity, rustfmt, locked workspace build, Clippy with warnings denied, full workspace tests including the Phase 9 read-only behavioral regression and contract proof, and all applicable CLI fixtures.
- Windows: **PASS** — the same gates plus the live read-only Runtime System X-Ray and the Windows-native runtime-executor fixture.
- External review: **3/3 CodeRabbit threads resolved**. The Major non-ASCII identity finding, exact-lane-sequence finding, and Major behavioral read-only proof finding are all closed.

Phase 9 remains assessment-only. It does not add OS-specific state probing, transaction binding, registry/service/AppX/feature mutation, or a public tweak write surface.

Phase 9 proof preserves every inherited Phase 1–8 gate and adds compiler, Clippy, regression, exact frozen-contract, and real proof-binary behavior checks for this boundary.

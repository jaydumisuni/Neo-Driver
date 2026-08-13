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
- CodeRabbit identified that the read-only proof surface did not behaviorally demonstrate filesystem non-mutation. A temporary-filesystem behavior test was not accepted by the available safety tooling, so no such behavioral proof is claimed. Instead, the frozen production-I/O contract structurally requires exactly two direct filesystem calls in `neo-state-plan`, both `std::fs::read_to_string`, and requires the proof CLI handler to contain no direct `std::fs` API. This closes the production write-surface concern structurally without overstating the evidence.
- CodeRabbit's docstring-coverage warning is a documentation-style metric, not a correctness or authority defect; no runtime or safety claim depends on that metric.

Phase 9 proof preserves every inherited Phase 1–8 gate and adds compiler, Clippy, regression, frozen-contract, and proof-binary checks for this boundary.

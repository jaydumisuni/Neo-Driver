# Neo Driver — Phase 9 State Assessment 20-Lane Review

**Scope:** deterministic read-only state assessment.

1. Workspace membership is explicit.
2. Dependency surface stays platform-neutral.
3. The crate remains assessment-only.
4. Typed values have deterministic serialization.
5. Opaque state keys validate before use.
6. State-key comparison is case-insensitive.
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

Phase 9 proof preserves every inherited Phase 1–8 gate and adds compiler, Clippy, regression, and proof-binary checks for this boundary.

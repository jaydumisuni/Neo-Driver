# Neo Driver — Phase 10 Windows State Resolution 20-Lane Review

**Scope:** deterministic read-only Windows state resolution layered on the proven Phase 9 assessment engine.

1. Phase 9 remains the canonical state-planning crate; Phase 10 does not create a competing resolver crate.
2. Cargo.lock remains free of a separate `neo-state-resolver` package boundary.
3. Reader identities are opaque, ASCII-only, validated identities.
4. Direct ReaderId deserialization re-runs validation.
5. State bindings use canonical Phase 9 target identity.
6. Root StateBindings deserialization re-runs validation and rejects duplicate targets.
7. Root CapturedStates deserialization re-runs validation and rejects duplicate reader observations.
8. Captured state preserves an explicit provenance source.
9. Explicit selection remains mandatory.
10. Duplicate and unknown tweak selections remain fail-closed.
11. Missing target bindings remain fail-closed.
12. Missing reader capture becomes unavailable evidence, never guessed state.
13. Phase 10 reuses the existing Phase 9 `assess_tweaks()` decision engine.
14. Windows evidence reuses the proven `neo-probe::scan_current_machine()` System X-Ray boundary.
15. The Windows adapter has a fixed reviewed reader catalogue rather than configurable commands.
16. Unknown reader IDs produce unavailable evidence rather than executing fallback logic.
17. The Phase 10 CLI uses validated domain JSON readers and contains no direct filesystem authority.
18. The original Phase 9 proof CLI remains preserved and no mutation authority is introduced.
19. Windows CI behaviorally proves live state capture while preserving an isolated fixture tree byte-for-byte.
20. Normal CI runs Phase 9 and Phase 10 static reviews, workspace proof, and the Windows live-state test.

## Review findings closed before freeze

- An initial design considered a separate resolver workspace crate. Recovery/review corrected that before merge: Phase 10 extends the existing `neo-state-plan` domain so no competing authority or unnecessary dependency boundary is introduced.
- An early low-level Windows-read draft was abandoned before entering the canonical workspace. The final implementation reuses the already-proven `neo-probe` System X-Ray profile rather than duplicating registry/service/AppX/feature command logic.
- The original Phase 9 proof module is restored and preserved byte-for-byte at its frozen boundary; Phase 10 uses a separate CLI module so predecessor contract tests remain authoritative.
- The first Windows live-proof fixture used a nonexistent `RecommendationState::Manual` enum variant. The fixture was corrected to the existing `Recommended` variant; production logic was unaffected.
- `ReaderId` originally derived Serde deserialization directly. Freeze review hardened the public persistence boundary so direct deserialization now calls `ReaderId::new()` and rejects uppercase, whitespace, non-ASCII, and other invalid identifiers.
- The new live CLI initially read the bindings file through `std::fs` directly. Freeze review switched it to `StateBindings::read_json()` and strengthened the Phase 10 static gate to reject direct filesystem access in the new CLI layer.
- Formatter-only findings were closed during integration. No behavior changed in those rustfmt corrections.

## Pre-PR implementation proof

- Hardened implementation head lineage culminated in the Phase 10 freeze branch after direct ReaderId validation and validated live binding loading.
- Authoritative pre-PR proof run: `31886646107`.
- Ubuntu: **PASS** — Phase 1–10 static reviews, Cargo lock integrity, rustfmt, locked workspace build, Clippy with warnings denied, complete workspace tests, and all applicable deterministic fixtures.
- Windows: **PASS** — the same gates plus the real Phase 10 live Windows state proof, live Runtime System X-Ray, Windows-native runtime executor fixture, and all remaining applicable fixtures.
- The Windows live-state proof launches the real `neo-state-assess live` binary and proves `Machine changes: none` while an isolated fixture tree remains unchanged.

Phase 10 remains strictly read-only. It does not bind tweak intent to a transaction or add any Windows mutation surface.

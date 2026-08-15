# Neo Driver — Phase 11 Transaction-Bound Tweak Executor 20-Lane Review

**Scope:** first bounded Tweaks mutation child. Internal capability-gated execution of three curated, reversible HKCU DWORD preferences only.

1. `neo-tweak-executor` is a first-class workspace crate.
2. The Windows dependency is target-only; non-Windows builds do not gain Registry linkage.
3. The curated mutation catalogue is exactly three approved tweak IDs.
4. The curated Windows binding table is exactly Explorer `HideFileExt`, `Hidden`, and `TaskbarAl` under the fixed current-user Explorer Advanced key.
5. Only certified DWORD `Set` operations with desired values `0` or `1` can become Phase 11 authority.
6. Unsupported tweak IDs, targets, operations, evidence, and Registry types fail closed.
7. Registry paths/value names are crate-private and cannot be supplied through persisted/public tweak data.
8. The raw host and Windows adapter are crate-private.
9. Public mutation methods require an opaque `TweakExecutorCapability` with no public constructor.
10. Phase 11 reuses the Phase 4 `TransactionPlan`/`TransactionCheckpoint` engine instead of introducing a second transaction state machine.
11. Actual HKCU pre-state is captured before authorization; donor/default values are never rollback truth.
12. Baseline state is re-read before authorization and before apply so drift blocks before a write.
13. API outcome and observed machine change remain separate evidence.
14. Successful writes require a fresh Registry observation through the transaction verification path.
15. Verification failure or changed write failure routes exact captured-state rollback.
16. A captured absent value rolls back by deleting that exact value; a captured DWORD rolls back to that exact DWORD.
17. Unsupported Registry type/size blocks before authority because Phase 11 cannot restore it exactly.
18. The real Windows backend uses typed Registry APIs directly; no shell, PowerShell, `reg.exe`, or arbitrary process command exists.
19. The public `neo` CLI has no Phase 11 mutation dependency/command.
20. Fake-host adversarial regressions cover unsupported authority, satisfied no-op, exact present/absent baselines, pre-authority/pre-apply drift, post-write verification, rollback, partial changed failure, multi-tweak completion, and the closed capability boundary.

## Frozen donor evidence

The first three bindings were recovered from the repository donor `jaydumisuni/winutil`:

- `Customize-Preferences/ShowExt.mdx` — `HideFileExt`, value `0` to show extensions.
- `Customize-Preferences/HiddenFiles.mdx` — `Hidden`, value `1` to show hidden files.
- `Customize-Preferences/TaskbarAlignment.mdx` — `TaskbarAl`, value `1` centered / `0` left.

WinUtil `OriginalValue` fields are **not** Neo rollback evidence. Neo restores only the actual value/presence captured immediately before authority.

## Pre-compiler findings closed

- Cargo generated the Phase 11 workspace lock with only the new local `neo-tweak-executor` package entry; no external dependency version changed.
- The first Phase 11 static run found a proof-harness wording mismatch in lane 20. Review then exposed a Python bug where `set.issubset()` was incorrectly applied to the decision string character-by-character. The lane now uses explicit phrase membership and the implementation lanes remain unchanged.
- The first formatter attempt exposed a Rust parse error in the curated-target comparison before type proof. The expected `TweakTarget` is now constructed as a named local value before canonical comparison.
- The fake-host rollback regression was corrected before compiler proof so a synthetic corrupted write produces a genuinely different observed DWORD and therefore exercises changed-state verification failure and exact rollback.
- The baseline-drift authorization regression now computes authorization before the mutable session borrow, avoiding a test-only borrow conflict.
- Canonical `cargo fmt --all` completed successfully after the parse correction, and its temporary write-enabled workflow self-deleted in the same commit.

## First compiler findings closed

The first cross-platform type proof reached the real Phase 11 crate on both Ubuntu and Windows and exposed one type-boundary defect plus one architecture-quality issue:

- `engine.rs` attempted to construct `TweakExecutionStep` through private sibling-module fields. The fix preserves private fields and adds a crate-private constructor instead of widening visibility.
- Mutation internals were compiled on non-Windows production builds even though only Windows execution and crate tests can use them, producing dead-code warnings that would later fail Clippy. `engine`/`session` and internal curated Registry model helpers are now compiled only for Windows or tests, while the public inspection surface remains cross-platform.
- The unused `validate_baseline_snapshot` helper was removed rather than suppressed.
- The corrected workspace was formatted canonically and the temporary formatting helper self-deleted before authoritative re-proof.

## Deliberate proof boundary

CI compiles the real Windows Registry backend, but all Phase 11 write/rollback behavior is exercised through the deterministic fake host. Phase 11 does not modify a GitHub runner Registry value and does not claim live ATHENA tweak mutation proof.

There is no public tweak apply CLI/GUI surface in this phase. Explorer restart, services, AppX/debloat, Windows Features, BCD/security controls, and broader tweak mutation remain separate future authority domains.

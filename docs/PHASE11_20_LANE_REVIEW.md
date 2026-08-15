# Neo Driver — Phase 11 Transaction-Bound Tweak Executor 20-Lane Review

**Scope:** first bounded Tweaks mutation child. Internal capability-gated execution of three curated, reversible HKCU DWORD preferences only.

1. `neo-tweak-executor` is a first-class workspace crate.
2. The Windows dependency is target-only; non-Windows builds do not gain Registry linkage.
3. The curated mutation catalogue is exactly three approved tweak IDs.
4. The curated Windows binding table is exactly Explorer `HideFileExt`, `Hidden`, and `TaskbarAl` under the fixed current-user Explorer Advanced key.
5. Each curated ID is bound to one exact approved forward DWORD: `show_file_extensions=0`, `show_hidden_files=1`, `centered_icons=1`; opposite/noncanonical values cannot become Phase 11 authority.
6. Unsupported tweak IDs, targets, operations, evidence, and Registry types fail closed.
7. Registry paths/value names are crate-private and cannot be supplied through persisted/public tweak data.
8. The raw host and Windows adapter are crate-private.
9. Public mutation methods require an opaque `TweakExecutorCapability` with no public constructor.
10. Phase 11 reuses the Phase 4 `TransactionPlan`/`TransactionCheckpoint` engine instead of introducing a second transaction state machine.
11. Actual HKCU pre-state is captured before authorization; donor/default values are never rollback truth.
12. Baseline state is re-read before authorization and before apply; a bounded same-session named mutex spans the pre-apply recheck through write/verify/rollback.
13. API outcome and observed machine change remain separate evidence.
14. Successful writes require a fresh Registry observation through the transaction verification path.
15. Every changed tweak gets a rollback attempt; complete rollback outcomes are recorded through the shared transaction batch contract before terminal failure is decided.
16. A captured absent value rolls back by deleting that exact value; a captured DWORD rolls back to that exact DWORD.
17. Unsupported Registry type/size, including `ERROR_MORE_DATA`, blocks before authority because Phase 11 cannot restore it exactly.
18. The real Windows backend uses typed Registry APIs directly; no shell, PowerShell, `reg.exe`, or arbitrary process command exists.
19. The public `neo` CLI has no Phase 11 mutation dependency/command; future capability issuance remains behind a separately reviewed typed MCP/RPC control plane.
20. Fake-host/shared-transaction adversarial regressions cover unsupported authority, contradictory curated semantics, satisfied no-op, exact present/absent baselines, pre-authority/pre-apply drift, post-write verification, complete multi-tweak rollback attempts, rollback batch completeness, partial failure, multi-tweak completion, and the closed capability boundary.

## Frozen donor evidence

The first three bindings were recovered from the repository donor `jaydumisuni/winutil`:

- `Customize-Preferences/ShowExt.mdx` — `HideFileExt`, DWORD `0` to show extensions.
- `Customize-Preferences/HiddenFiles.mdx` — `Hidden`, DWORD `1` to show hidden files; DWORD `0` is the opposite hidden state.
- `Customize-Preferences/TaskbarAlignment.mdx` — `TaskbarAl`, DWORD `1` centered / DWORD `0` left.

WinUtil `OriginalValue` fields are **not** Neo rollback evidence. Neo restores only the actual value/presence captured immediately before authority.

## Engineering findings closed before freeze

- Cargo generated the Phase 11 workspace lock without external dependency-version drift.
- Early static-review wording and Python `issubset()` proof-harness defects were corrected before product proof.
- Rust parsing/formatting issues in curated-target comparison were corrected before type proof.
- Fake-host rollback and baseline-drift regressions were corrected so they exercise real changed-state and borrow-safe authority paths.
- The first cross-platform type proof found private sibling-field construction and non-Windows dead-code issues; a crate-private constructor plus Windows/test-only internals closed those without widening authority or suppressing lints.
- Clippy found a test-only field-reassignment pattern; the fixture now initializes the field structurally with no lint suppression.
- Independent pre-review audit found same-session cross-process stale-baseline/rollback risk. A bounded `Local\THETECHGUY.NeoDriver.TweakExecutor.v1` named mutex now covers the second baseline check through writes, verification, and rollback. Windows units acquire/release the real mutex without modifying Registry values.

## External-review findings closed

CodeRabbit full review identified three current correctness findings. All are fixed, regression-bound, replied to with current-head evidence, and explicitly resolved on PR #19:

1. **Curated semantic binding:** a binary value with the wrong semantic direction could previously be requested. `RegistryTweakSpec` now binds each ID to its one approved forward DWORD, persisted-plan validation preserves that binding, and `contradictory_curated_semantics_fail_closed` proves rejection.
2. **Complete rollback attempts:** rollback previously could stop after the first restore failure. Phase 11 now attempts every changed independent tweak and submits the complete result set through additive `TransactionCheckpoint::record_rollback_results_batch`; transaction-level tests require complete coverage and preserve every outcome before terminal failure, while the Phase 11 regression proves a later tweak is restored after an earlier restore failure.
3. **Oversized Registry values:** fixed four-byte DWORD reads now classify `ERROR_MORE_DATA` as `UnsupportedRegistryState`, preserving the fail-closed exact-state contract.

CodeRabbit confirmed each correction in-thread. PR #19 has zero unresolved review threads after reconciliation.

## Implementation-code proof

Current implementation head `53427008f41e383b46f8c7fea73a1e048d484844` passed run `31894350194` on Ubuntu and Windows:

- Phase 1–11 deterministic static reviews;
- Cargo lock integrity;
- rustfmt;
- locked full-workspace type/build;
- Clippy with warnings denied;
- complete workspace unit/adversarial suite, including rollback-batch and Phase 11 review regressions;
- Windows Phase 10 live read-only state proof;
- Windows Runtime System X-Ray;
- all applicable catalogue, matcher, runtime, gaming, vault, runtime-executor, and transaction CLI fixtures.

This is implementation-code proof. The documentation-state head created by this freeze must repeat the complete Ubuntu/Windows pipeline before merge.

## Deliberate proof boundary

CI compiles the real Windows Registry backend and acquires/releases the real named mutex, but all Phase 11 Registry write/rollback behavior is exercised through deterministic fake hosts. Phase 11 does **not** modify a GitHub runner Registry value and does **not** claim live ATHENA tweak mutation proof.

There is no public tweak apply CLI/GUI or MCP/RPC capability issuance in this phase. Explorer restart, services, AppX/debloat, Windows Features, BCD/security controls, cross-session/global serialization, and broader tweak mutation remain separate future authority domains.

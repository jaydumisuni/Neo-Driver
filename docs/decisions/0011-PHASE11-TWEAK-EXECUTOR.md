# Decision 0011 — Phase 11 Transaction-Bound Tweak Executor

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Product domain:** first bounded mutation child of Tweaks  
**Authority:** internal, capability-gated, reversible HKCU DWORD mutation only

## Decision

Phase 11 introduces Neo's first tweak mutator, but it does not create a generic registry editor and it does not expose a public tweak-apply CLI or GUI surface.

The first mutation catalogue is intentionally limited to three low-blast-radius, current-user Explorer preferences recovered from the WinUtil donor:

1. `windows.explorer.show_file_extensions`
   - `HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced`
   - value `HideFileExt`
   - approved forward DWORD `0` shows file extensions; DWORD `1` hides them.
2. `windows.explorer.show_hidden_files`
   - same key
   - value `Hidden`
   - approved forward DWORD `1` shows hidden files; DWORD `2` hides them.
3. `windows.taskbar.centered_icons`
   - same key
   - value `TaskbarAl`
   - approved forward DWORD `1` centers taskbar icons; DWORD `0` aligns them left.

The donor's `OriginalValue` fields are semantic reference only. Neo rollback authority comes only from the actual value captured immediately before authorization.

## Registry authority

Persisted tweak/catalogue data may select only the three curated Neo tweak IDs and each ID's exact approved forward DWORD. Phase 11 does not accept the opposite value merely because it has the same Registry type. `show_file_extensions` is bound to `0`, `show_hidden_files` is bound to `1`, and `centered_icons` is bound to `1`. No persisted or public field can provide a registry hive, subkey, value name, arbitrary command, script, or executable.

The Windows backend uses the locked `windows` crate Registry API directly. It does not invoke PowerShell, `reg.exe`, `cmd.exe`, or a shell.

Phase 11 supports only current-user DWORD values. Baseline states are:

- absent; or
- present as exact DWORD value.

If the selected value exists with any other Registry type, pre-authority capture fails closed because Phase 11 cannot restore that state exactly. Baseline capture may preserve any actual DWORD value—even a noncanonical one—because rollback truth is observed machine state, while forward authority remains fixed to the curated value for that tweak ID.

## Transaction law

Every changed tweak becomes one Phase 4 `TransactionAction` with:

- `ActionKind::Tweak`;
- explicit user confirmation;
- a `RegistryValue` snapshot target;
- an exact desired postcondition;
- reversible rollback to the same captured target;
- rollback verification using `MatchesBaseline`.

The transaction fingerprint binds authority to the exact selected actions and desired state.

The executor captures actual Registry state before authorization. Any drift between captured state and the read-only state used to prepare the action blocks execution before a write.

After a write, Neo re-reads the value and feeds the fresh observation into the Phase 4 verification engine. An API success code is not completion proof.

Rollback restores captured reality:

- a captured DWORD is written back exactly;
- a captured absent value is deleted;
- rollback is not attempted for an unsupported/unavailable baseline.

## Cross-process serialization

Phase 11 serializes Neo tweak apply operations within the current Windows session using a fixed `Local\\THETECHGUY.NeoDriver.TweakExecutor.v1` named mutex with a bounded wait.

The mutex is acquired before the second baseline-drift check and remains held through transaction apply, all Registry writes, post-write verification, and any rollback. This prevents two Neo processes in the same Windows session from both acting on overlapping stale baselines and later restoring over one another.

The `Local\\` namespace is intentionally same-session authority only; Phase 11 does not claim cross-session serialization. Unrelated external Registry writers are not controlled by the mutex and remain subject to Neo's fresh pre-apply and post-write observation/verification checks.

## Capability boundary

The raw Registry host and mutation invocation types are crate-private. Public/session mutation methods require an opaque `TweakExecutorCapability` with no public constructor.

Phase 11 therefore proves the executor without issuing mutation authority through the public CLI or GUI.

CI compiles the real Windows Registry backend but exercises mutation behavior only through a deterministic fake host. No GitHub runner Registry value is modified by Phase 11 tests.

## Deliberate limits

Phase 11 does not implement:

- arbitrary registry editing;
- inverse/opposite operations for the three curated one-way tweak IDs;
- services;
- AppX/debloat;
- Windows optional features;
- BCD/Test Signing/security mutation;
- Explorer restart/shell restart;
- public tweak apply;
- GUI write actions;
- cross-session/global tweak serialization;
- live attached-machine tweak mutation proof.

Those remain separately gated.

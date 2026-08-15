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
   - approved forward DWORD `1` shows hidden files; canonical opposite DWORD `2` hides them.
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

If the selected value exists with any other Registry type, pre-authority capture fails closed because Phase 11 cannot restore that state exactly. `ERROR_MORE_DATA` from the fixed DWORD read path is also classified as unsupported Registry state rather than as a generic operational error, because an oversized value cannot satisfy the exact four-byte DWORD contract. Baseline capture may preserve any actual DWORD value—even a noncanonical one—because rollback truth is observed machine state, while forward authority remains fixed to the curated value for that tweak ID.

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

When more than one changed tweak requires rollback, Neo attempts every independent restoration even if an earlier restoration fails. The complete changed-action rollback outcome set is recorded atomically through the shared transaction batch API before the checkpoint becomes terminal. A failed restoration therefore cannot prevent later changed tweaks from receiving their own restoration attempt or evidence record.

## Cross-process serialization

Phase 11 serializes Neo tweak apply operations within the current Windows session using a fixed `Local\\THETECHGUY.NeoDriver.TweakExecutor.v1` named mutex with a bounded wait.

The mutex is acquired before the second baseline-drift check and remains held through transaction apply, all Registry writes, post-write verification, and any rollback. This prevents two Neo processes in the same Windows session from both acting on overlapping stale baselines and later restoring over one another.

The `Local\\` namespace is intentionally same-session authority only; Phase 11 does not claim cross-session serialization. Unrelated external Registry writers are not controlled by the mutex and remain subject to Neo's fresh pre-apply and post-write observation/verification checks.

## Capability and MCP/RPC boundary

The raw Registry host and mutation invocation types are crate-private. Public/session mutation methods require an opaque `TweakExecutorCapability` with no public constructor.

Neo's higher-level orchestration is MCP/RPC-first. Future Hunter, Oracle, GUI, and other approved TTG callers must invoke typed MCP/RPC service contracts rather than bypassing the core through ad-hoc shell or public CLI mutation. Capability issuance belongs behind that service boundary after permission, plan, transaction, confirmation, and evidence checks.

Phase 11 proves the internal executor only. It does **not** issue `TweakExecutorCapability` through MCP/RPC, CLI, or GUI yet. The CLI remains diagnostic/manual tooling rather than the primary mutation control plane.

CI compiles the real Windows Registry backend but exercises mutation behavior only through a deterministic fake host. No GitHub runner Registry value is modified by Phase 11 tests.

## Error taxonomy

Caller/request validation is kept separate from host/Registry failures. Invalid preparation input such as an empty mission ID returns `InvalidRequest`; `Registry` is reserved for actual Windows Registry/host operation failures. This distinction is required so the future MCP/RPC service can map caller errors and execution failures into truthful structured responses.

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
- MCP/RPC mutation capability issuance;
- cross-session/global tweak serialization;
- live attached-machine tweak mutation proof.

Those remain separately gated.

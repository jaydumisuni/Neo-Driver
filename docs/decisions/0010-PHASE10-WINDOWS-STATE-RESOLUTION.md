# Decision 0010 — Phase 10 Windows State Resolution

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Product domain:** second bounded child of the frozen Tweaks domain  
**Authority:** read-only Windows state capture and assessment only

## Decision

Repository Phase 10 extends the proven Phase 9 `neo-state-plan` domain rather than creating a second state-planning crate. Phase 10 binds opaque Phase 9 tweak targets to opaque, validated reader identities and normalizes already-proven Windows evidence into the same `TweakEvidence` assessment contract Phase 9 already trusts.

Phase 10 adds no tweak mutation authority, transaction binding, registry/service/AppX/feature write operation, public tweak write command, or arbitrary command configuration.

## Resolver model

`ReaderId` is an ASCII-only opaque identity. Valid reader IDs contain only lowercase ASCII letters, digits, `.`, `_`, and `-`. Direct Serde deserialization re-runs `ReaderId::new()` validation.

`StateBinding` maps one canonical Phase 9 `TweakTarget` to one `ReaderId`. `StateBindings` validates the complete root and rejects duplicate target bindings after canonical target normalization.

`CapturedState` records one reader identity, one typed `ObservedState`, and one non-empty provenance source. `CapturedStates` validates the complete root and rejects duplicate reader observations.

`resolve_selected_evidence()` requires an explicit, duplicate-free tweak selection. Unknown tweak IDs and missing bindings fail closed. Missing reader capture is normalized to `ObservedState::Unavailable`, which Phase 9 already treats as a hard assessment gate rather than guessed state.

## Windows evidence boundary

The Phase 10 Windows adapter does not introduce a second low-level Windows command surface. It reuses `neo-probe::scan_current_machine()`, the already-proven read-only System X-Ray boundary, and maps only fixed reviewed reader IDs into normalized machine-profile evidence.

The initial fixed catalogue includes Windows product/display/build/architecture identity plus the already-proven read-only security/reboot facts exposed by System X-Ray: Test Signing, no-integrity-checks, Secure Boot, Memory Integrity, and pending reboot.

Unknown reader IDs do not execute or construct a command. They produce unavailable evidence.

Persisted JSON can therefore select only a validated opaque reader ID; it cannot provide an executable, command line, registry path, service name, PowerShell fragment, DISM argument, or other platform instruction.

## CLI proof boundary

`neo-state-assess live` is a read-only proof surface in the existing CLI package. It loads the curated tweak catalogue and validated state bindings through domain constructors, captures approved Windows evidence through the System X-Ray adapter, resolves it into Phase 9 evidence, and runs the existing `assess_tweaks()` engine.

The command reports `Machine changes: none`. No product `neo` write command is added.

The original Phase 9 `state_assess_cli.rs` remains preserved so predecessor contract tests continue proving the Phase 9 boundary independently from the new Phase 10 proof path.

## Behavioral proof

Windows CI launches the real `neo-state-assess live` binary against the fixed `windows.os.current_build` reader, snapshots an isolated fixture tree before and after execution, and requires identical contents plus successful read-only assessment output.

The live proof therefore exercises real Windows evidence capture without claiming or performing a machine change.

## Deliberate limits

Phase 10 does **not** implement tweak execution, registry writes, service changes, AppX removal, Windows feature mutation, BCD/security mutation, transaction binding, rollback, public GUI write actions, or user-facing tweak-apply authority.

Those require a later bounded phase after the read-only current-state evidence contract is frozen and proven.

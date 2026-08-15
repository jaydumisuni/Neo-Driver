from pathlib import Path

path = Path("docs/IMPLEMENTATION_STATUS.md")
text = path.read_text(encoding="utf-8")

phase9_state = "- **Phase 9:** merged and engineering-proven — read-only, platform-neutral tweak/state assessment foundation with typed desired state, validated evidence/catalogue, explicit selection, deterministic current-vs-desired comparison, and behavioral non-mutation proof. OS-specific probing, transaction binding, and tweak execution remain blocked.\n"
phase10_state = "- **Phase 10:** merged and engineering-proven — read-only Windows live-state resolution layered on Phase 9, with validated target→reader bindings, fixed reviewed System-X-Ray reader identities, captured-state provenance, real Windows live-state behavioral proof, and zero tweak mutation authority. Transaction binding and tweak execution remain blocked.\n"
assert phase9_state in text
assert phase10_state not in text
text = text.replace(phase9_state, phase9_state + phase10_state, 1)

phase9_baseline = "Phase 9 implementation-code run `31715322010` and final documentation-state run `31715738064` passed the complete Ubuntu and Windows pipeline with all three CodeRabbit review threads resolved, and Phase 9 merged through PR #14 as `ad75a557f4787b9e1b902971b017cb71ce3ac511`.\n"
phase10_baseline = "\nPhase 10 corrected implementation run `31887310279` and final documentation-state run `31887513599` passed the complete Ubuntu and Windows Phase 1–10 pipeline with both Major CodeRabbit review threads resolved, including the real Windows live-state proof, and Phase 10 merged through PR #17 as `15b62fcbab8d400fd5497b422243b85d7f3d5595`.\n"
assert phase9_baseline in text
assert "Phase 10 corrected implementation run `31887310279`" not in text
text = text.replace(phase9_baseline, phase9_baseline + phase10_baseline, 1)

phase10_section = """## Phase 10 frozen implementation

Phase 10 adds the first Windows live-state resolution layer for Tweaks while preserving the Phase 9 assessment boundary. It does **not** add a tweak mutator or transaction-bound write surface.

It adds:

- validated opaque `ReaderId` identities with direct-Serde revalidation;
- canonical Phase 9 target → reader bindings with duplicate-target rejection;
- validated captured-state roots with explicit provenance, duplicate-reader rejection, and revalidation before indexing even for directly constructed public values;
- deterministic resolution of approved captured reader evidence into the existing Phase 9 `TweakEvidence` contract;
- missing reader capture normalized to `ObservedState::Unavailable`, preserving Phase 9 fail-closed assessment;
- reuse of the proven `neo-probe::scan_current_machine()` System X-Ray boundary instead of introducing a second low-level Windows command surface;
- an exact fixed nine-reader Windows catalogue covering OS identity, Test Signing, no-integrity-checks, Secure Boot, Memory Integrity, and pending reboot evidence;
- unknown reader IDs returning unavailable evidence rather than executing fallback logic;
- read-only `neo-state-assess live` proof flow using validated domain JSON readers and the existing Phase 9 assessment engine;
- a real Windows behavioral proof that captures `windows.os.current_build`, reports `Machine changes: none`, and preserves an isolated fixture tree unchanged;
- a strengthened Phase 10 20-lane gate that structurally inspects Rust blocks, exact reader match arms, the frozen Phase 9 CLI blob, named regression tests, and active CI step definitions/commands before the executable unit/live proof chain runs;
- two Major CodeRabbit findings closed and re-proven: directly constructed captured-state validation and structural/executable 20-lane enforcement.

Phase 10 remains read-only. Registry/service/AppX/feature mutation, transaction binding, rollback, public tweak apply, and GUI write actions remain outside this boundary.

The detailed Phase 10 authority, review corrections, and proof boundary are frozen in `docs/decisions/0010-PHASE10-WINDOWS-STATE-RESOLUTION.md` and `docs/PHASE10_20_LANE_REVIEW.md`.

"""
marker = "## Still deliberately blocked\n"
assert marker in text
assert "## Phase 10 frozen implementation" not in text
text = text.replace(marker, phase10_section + marker, 1)

phase9_blocked = "Phase 9 does **not** yet resolve abstract state targets to Windows registry/service/AppX/feature targets, probe those targets live, bind those targets into the transaction engine, or execute a tweak.\n"
phase10_blocked = "Phase 9 remains the platform-neutral desired/current-state assessment authority. Phase 10 now resolves only its fixed reviewed Windows reader catalogue through the proven System X-Ray, but it does **not** bind tweak intent into the transaction engine, execute registry/service/AppX/feature changes, or expose a public tweak-apply surface.\n"
assert phase9_blocked in text
text = text.replace(phase9_blocked, phase10_blocked, 1)

phase9_decision = "- `docs/decisions/0009-PHASE9-STATE-ASSESSMENT-FOUNDATION.md`.\n"
phase10_decision = "- `docs/decisions/0009-PHASE9-STATE-ASSESSMENT-FOUNDATION.md`;\n- `docs/decisions/0010-PHASE10-WINDOWS-STATE-RESOLUTION.md`.\n"
assert phase9_decision in text
text = text.replace(phase9_decision, phase10_decision, 1)

phase9_proof = "- Phase 9: **PROVEN and merged** at the read-only state-assessment foundation boundary.\n"
phase10_proof = "- Phase 10: **PROVEN and merged** at the read-only Windows live-state resolution + Phase 9 assessment boundary.\n"
assert phase9_proof in text
assert phase10_proof not in text
text = text.replace(phase9_proof, phase9_proof + phase10_proof, 1)

phase9_tail = "- Phase 9 machine mutation proof: **not claimed**; Phase 9 is assessment-only and does not probe or change live Windows tweak state.\n"
phase10_tail = "\n- Phase 10 corrected implementation run `31887310279`: **PASS on Ubuntu and Windows** across Phase 1–10 structural/static gates, lock integrity, rustfmt, locked build, Clippy with warnings denied, complete workspace units including direct-construction captured-state regressions, Windows live-state proof, Runtime System X-Ray, and every applicable fixture.\n- Final Phase 10 documentation-state run `31887513599`: **PASS on Ubuntu and Windows** across the complete configured Phase 1–10 pipeline.\n- Phase 10 PR #17 external review: **2/2 Major CodeRabbit threads resolved** — direct captured-state validation and structural/executable Phase 10 proof enforcement.\n- Phase 10 merged through PR #17 as `15b62fcbab8d400fd5497b422243b85d7f3d5595`.\n- Phase 10 machine mutation proof: **not claimed**; Phase 10 captures and resolves live Windows state read-only and does not execute a tweak.\n"
assert phase9_tail in text
assert "Final Phase 10 documentation-state run `31887513599`" not in text
text = text.replace(phase9_tail, phase9_tail + phase10_tail, 1)

closing = "Phases 1–9 are closed at their recorded repository boundaries. Phase 5 public driver mutation still requires live attached-device proof. Phase 8 public runtime mutation still requires a separately reviewed capability-issuance/live-installer proof path. Phase 9 remains read-only until OS-specific target resolution, live probing, transaction binding, and tweak execution receive their own frozen authority and proof. Phase 7 network acquisition, archive execution, Windows-feature mutation, Winget execution, and public vault write surfaces remain independently blocked until their own authority, verification, cleanup, and recovery contracts are frozen and proven."
replacement = "Phases 1–10 are closed at their recorded repository boundaries. Phase 5 public driver mutation still requires live attached-device proof. Phase 8 public runtime mutation still requires a separately reviewed capability-issuance/live-installer proof path. Phase 10 now provides fixed reviewed Windows live-state resolution but remains read-only until transaction binding and tweak execution receive their own frozen authority and proof. Phase 7 network acquisition, archive execution, Windows-feature mutation, Winget execution, and public vault write surfaces remain independently blocked until their own authority, verification, cleanup, and recovery contracts are frozen and proven."
assert closing in text
text = text.replace(closing, replacement, 1)

required = [
    phase10_state.strip(),
    "Phase 10 corrected implementation run `31887310279`",
    "## Phase 10 frozen implementation",
    "Phase 10: **PROVEN and merged**",
    "Phase 10 merged through PR #17 as `15b62fcbab8d400fd5497b422243b85d7f3d5595`",
    "Phases 1–10 are closed at their recorded repository boundaries.",
]
for value in required:
    assert value in text, value

path.write_text(text, encoding="utf-8")

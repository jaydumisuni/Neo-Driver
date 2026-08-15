from pathlib import Path

path = Path("docs/IMPLEMENTATION_STATUS.md")
text = path.read_text(encoding="utf-8")


def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one status anchor, found {count}: {old[:100]!r}")
    text = text.replace(old, new, 1)


phase10_bullet = "- **Phase 10:** merged and engineering-proven — read-only Windows live-state resolution layered on Phase 9, with validated target→reader bindings, fixed reviewed System-X-Ray reader identities, captured-state provenance, real Windows live-state behavioral proof, and zero tweak mutation authority. Transaction binding and tweak execution remain blocked."
phase11_bullet = "- **Phase 11:** merged and engineering-proven — internal capability-gated transaction-bound execution for exactly three curated reversible HKCU DWORD tweaks, with exact semantic value binding, actual pre-state capture, same-session serialization, complete rollback-attempt evidence, direct Windows Registry APIs, and zero public tweak-apply authority. Live Registry mutation proof is not claimed."
replace_once(phase10_bullet, phase10_bullet + "\n" + phase11_bullet)

phase10_baseline = "Phase 10 corrected implementation run `31887310279` and final documentation-state run `31887513599` passed the complete Ubuntu and Windows Phase 1–10 pipeline with both Major CodeRabbit review threads resolved, including the real Windows live-state proof, and Phase 10 merged through PR #17 as `15b62fcbab8d400fd5497b422243b85d7f3d5595`."
phase11_baseline = "Phase 11 corrected implementation run `31894350194` and final documentation-state run `31894626669` passed the complete Ubuntu and Windows Phase 1–11 pipeline with all three CodeRabbit correctness threads resolved, and Phase 11 merged through PR #19 as `66cca16be15fe617590445c6bb8993c5a242caf0`."
replace_once(phase10_baseline, phase10_baseline + "\n\n" + phase11_baseline)

phase10_detail = "The detailed Phase 10 authority, review corrections, and proof boundary are frozen in `docs/decisions/0010-PHASE10-WINDOWS-STATE-RESOLUTION.md` and `docs/PHASE10_20_LANE_REVIEW.md`."
phase11_section = '''## Phase 11 frozen implementation

Phase 11 adds the first bounded Tweaks mutator over the Phase 9/10 state model while keeping mutation authority internal.

It adds:

- `neo-tweak-executor` as a first-class workspace crate;
- exactly three curated one-way HKCU DWORD mutations: show file extensions (`HideFileExt=0`), show hidden files (`Hidden=1`), and centered taskbar icons (`TaskbarAl=1`);
- fixed crate-private Registry paths/value names with no caller-supplied hive/subkey/value-name authority;
- exact approved forward DWORD binding per curated ID, including direct and persisted-plan revalidation;
- actual Registry baseline capture before authorization and a second drift check before apply;
- Phase 4 reversible transactions with exact postconditions and `MatchesBaseline` rollback verification;
- complete rollback attempts for every changed tweak through the additive shared `record_rollback_results_batch` contract before terminal rollback failure is decided;
- absent-state rollback by exact value deletion and present-state rollback to the exact captured DWORD;
- unsupported Registry type/size fail-closed behavior, including `ERROR_MORE_DATA` classification as unsupported state;
- direct Windows Registry APIs only, with no PowerShell, `reg.exe`, `cmd.exe`, shell, or arbitrary process path;
- a bounded same-session `Local\\THETECHGUY.NeoDriver.TweakExecutor.v1` mutex covering the pre-apply recheck through write, verification, and rollback;
- a real Windows mutex acquire/release regression with no Registry mutation;
- an opaque `TweakExecutorCapability` with no public constructor;
- no `neo` CLI/GUI mutation command and no Phase 11 MCP/RPC capability issuance; future higher-level invocation remains behind a separately reviewed typed control-plane boundary;
- three CodeRabbit correctness findings closed and regression-bound: curated semantic value binding, complete multi-tweak rollback attempts, and oversized Registry-value classification.

CI compiles the real Windows Registry backend and exercises the real named mutex, but write/rollback behavior remains fake-host driven. Phase 11 does **not** claim live ATHENA Registry mutation proof.

The detailed Phase 11 authority, external-review corrections, and proof boundary are frozen in `docs/decisions/0011-PHASE11-TWEAK-EXECUTOR.md` and `docs/PHASE11_20_LANE_REVIEW.md`.'''
replace_once(phase10_detail, phase10_detail + "\n\n" + phase11_section)

old_tweak_boundary = "Phase 9 remains the platform-neutral desired/current-state assessment authority. Phase 10 now resolves only its fixed reviewed Windows reader catalogue through the proven System X-Ray, but it does **not** bind tweak intent into the transaction engine, execute registry/service/AppX/feature changes, or expose a public tweak-apply surface."
new_tweak_boundary = "Phase 9 remains the platform-neutral desired/current-state assessment authority and Phase 10 remains the fixed reviewed read-only Windows state resolver. Phase 11 now binds exactly three curated HKCU DWORD tweaks into the proven transaction engine and implements an internal capability-gated Registry executor, but it does **not** expose public CLI/GUI/MCP-RPC apply authority, broader Registry/service/AppX/feature mutation, or claim live ATHENA Registry mutation proof."
replace_once(old_tweak_boundary, new_tweak_boundary)

replace_once(
    "- debloat/tweak execution;",
    "- public/general debloat or tweak execution beyond the internal Phase 11 three-tweak capability;",
)

decision10 = "- `docs/decisions/0010-PHASE10-WINDOWS-STATE-RESOLUTION.md`."
decision11 = "- `docs/decisions/0011-PHASE11-TWEAK-EXECUTOR.md`."
replace_once(decision10, decision10[:-1] + ";\n" + decision11)

proof10 = "- Phase 10: **PROVEN and merged** at the read-only Windows live-state resolution + Phase 9 assessment boundary."
proof11 = "- Phase 11: **PROVEN and merged** at the internal capability-gated three-tweak HKCU DWORD transaction/executor boundary; no public apply capability or live Registry mutation proof is claimed."
replace_once(proof10, proof10 + "\n" + proof11)

phase10_proof_tail = "- Phase 10 machine mutation proof: **not claimed**; Phase 10 captures and resolves live Windows state read-only and does not execute a tweak."
phase11_proof_tail = '''- Phase 11 corrected implementation run `31894350194`: **PASS on Ubuntu and Windows** across Phase 1–11 static gates, lock integrity, rustfmt, locked build, Clippy with warnings denied, full workspace units/adversarial regressions, Windows live read-only state proof, Runtime System X-Ray, and every applicable fixture.
- Final Phase 11 documentation-state run `31894626669`: **PASS on Ubuntu and Windows** across the complete configured Phase 1–11 pipeline.
- Phase 11 PR #19 external review: **3/3 CodeRabbit correctness threads resolved** — exact curated DWORD semantic binding, complete rollback attempts/evidence for all changed tweaks, and `ERROR_MORE_DATA` fail-closed Registry-state classification.
- Phase 11 Windows synchronization proof: **real named mutex acquisition/release executed in CI without Registry mutation**.
- Phase 11 merged through PR #19 as `66cca16be15fe617590445c6bb8993c5a242caf0`.
- Phase 11 live Registry mutation proof: **not claimed**; real Registry write/rollback behavior remains behind the opaque internal capability and fake-host proof boundary.
- Phase 11 public tweak mutation proof: **not claimed**; CLI/GUI/MCP-RPC capability issuance remains blocked pending its own reviewed authority contract.'''
replace_once(phase10_proof_tail, phase10_proof_tail + "\n\n" + phase11_proof_tail)

old_close = "Phases 1–10 are closed at their recorded repository boundaries. Phase 5 public driver mutation still requires live attached-device proof. Phase 8 public runtime mutation still requires a separately reviewed capability-issuance/live-installer proof path. Phase 10 now provides fixed reviewed Windows live-state resolution but remains read-only until transaction binding and tweak execution receive their own frozen authority and proof. Phase 7 network acquisition, archive execution, Windows-feature mutation, Winget execution, and public vault write surfaces remain independently blocked until their own authority, verification, cleanup, and recovery contracts are frozen and proven."
new_close = "Phases 1–11 are closed at their recorded repository boundaries. Phase 5 public driver mutation still requires live attached-device proof. Phase 8 public runtime mutation still requires a separately reviewed capability-issuance/live-installer proof path. Phase 11 now provides internal transaction-bound Registry execution for exactly three curated HKCU DWORD tweaks, while public tweak authority, broader tweak/debloat domains, and live Registry mutation proof remain separately blocked. Phase 7 network acquisition, archive execution, Windows-feature mutation, Winget execution, and public vault write surfaces remain independently blocked until their own authority, verification, cleanup, and recovery contracts are frozen and proven."
replace_once(old_close, new_close)

required = [
    "Phase 11: **PROVEN and merged**",
    "31894350194",
    "31894626669",
    "66cca16be15fe617590445c6bb8993c5a242caf0",
    "3/3 CodeRabbit correctness threads resolved",
    "live Registry mutation proof: **not claimed**",
]
for marker in required:
    if marker not in text:
        raise SystemExit(f"missing required Phase 11 status marker: {marker}")

path.write_text(text, encoding="utf-8")

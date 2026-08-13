#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PATH = ROOT / "docs/IMPLEMENTATION_STATUS.md"
text = PATH.read_text(encoding="utf-8")


def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one status anchor, found {count}: {old[:120]!r}")
    text = text.replace(old, new, 1)


replace_once(
    "- **Phase 7:** merged and engineering-proven — Builder/portable-rooted managed package vault (`NeoData`) with verified local/offline pack intake, pinned TTG source provenance, no-follow filesystem authority, concurrent-promotion protection, marker-owned cleanup, and read-only public vault inspection. Network acquisition and public vault mutation remain blocked.\n",
    "- **Phase 7:** merged and engineering-proven — Builder/portable-rooted managed package vault (`NeoData`) with verified local/offline pack intake, pinned TTG source provenance, no-follow filesystem authority, concurrent-promotion protection, marker-owned cleanup, and read-only public vault inspection. Network acquisition and public vault mutation remain blocked.\n- **Phase 8:** merged and engineering-proven — bounded internal runtime executor for exact local/offline single-file runtime payloads, bound to Phase 6 Certified assessment, Phase 7 vault authority, and Phase 4 irreversible transaction/reboot verification. Public runtime mutation remains blocked because the opaque execution capability is not issued to external callers or the CLI.\n",
)

replace_once(
    "Phase 7 final exact-head documentation-state run `31687570246` passed the complete Ubuntu and Windows Phase 1–7 pipeline, and Phase 7 merged through PR #10 as `bca02a8a294a976debcc26b480cea0c3ba4da2e2`.\n",
    "Phase 7 final exact-head documentation-state run `31687570246` passed the complete Ubuntu and Windows Phase 1–7 pipeline, and Phase 7 merged through PR #10 as `bca02a8a294a976debcc26b480cea0c3ba4da2e2`.\n\nPhase 8 final documentation-state run `31698767919` passed the complete Ubuntu and Windows Phase 1–8 pipeline with zero unresolved review threads, and Phase 8 merged through PR #12 as `7a26d8d9dc86ac5f5db09eaf82b58424b1babd26`.\n",
)

anchor = "The detailed Phase 7 contract and recovered engineering findings are frozen in `docs/decisions/0007-PHASE7-MANAGED-PACKAGE-VAULT.md` and `docs/PHASE7_20_LANE_REVIEW.md`.\n\n## Still deliberately blocked\n"
phase8 = """The detailed Phase 7 contract and recovered engineering findings are frozen in `docs/decisions/0007-PHASE7-MANAGED-PACKAGE-VAULT.md` and `docs/PHASE7_20_LANE_REVIEW.md`.

## Phase 8 frozen implementation

Phase 8 adds a bounded internal runtime-execution boundary without replacing or weakening the read-only Phase 6 assessment/System-X-Ray layer or Phase 7 vault.

It adds:

- `neo-runtime-executor` as a separate first-class workspace crate;
- runtime-only execution metadata in catalogue manifests for exact EXE/MSI payload contracts;
- execution-plan preparation that re-derives authority from a Phase 6 `Certified` runtime recommendation;
- exact package ID/version/SHA evidence and Builder/portable-rooted Phase 7 vault path derivation;
- absolute application-root validation and direct-Serde revalidation of persisted execution plans;
- dependency/conflict and boot/security-mutation hard blocks before runtime authority exists;
- marker-owned no-follow staging through Phase 7 vault capabilities;
- direct EXE execution and trusted System32 `msiexec.exe` MSI execution with no shell path;
- MSI argument validation, including rejection of bare empty `PROPERTY=` assignments while preserving explicit `PROPERTY=\"\"` semantics;
- Windows 32-bit exit-status bit-pattern preservation, including high-bit HRESULT/Win32 representations;
- bounded same-session cross-process serialization through a fixed `Local\\` named mutex with timeout handling;
- locked staged-file SHA-256 re-verification immediately before process launch;
- conservative `machine_changed` evidence whenever an installer process starts;
- irreversible Phase 4 transaction authority with exact captured runtime baseline and mandatory acknowledgement;
- reboot/resume and post-install re-probe through the proven transaction checkpoint engine;
- retryable verification after transient runtime observation failure;
- an opaque `RuntimeExecutorCapability` with no public constructor, so safe external callers cannot invoke mutation even though validated plans/sessions remain inspectable;
- crate-private raw host/invocation/process/Windows-host adapters, closing the initial library-authority bypass found during PR review;
- read-only public CLI plan validation only; no public runtime install/apply command;
- Phase 8 20-lane review integrated beside inherited Phase 1–7 gates.

The detailed Phase 8 authority, review corrections, and proof boundary are frozen in `docs/decisions/0008-PHASE8-RUNTIME-EXECUTOR.md` and `docs/PHASE8_20_LANE_REVIEW.md`.

## Still deliberately blocked
"""
replace_once(anchor, phase8)

replace_once(
    "Phase 6 does **not** expose runtime installation or repair execution yet.\n\nPhase 7 does **not** expose online package acquisition, archive execution, public pack import/cleanup writes, or any new driver/security mutation authority.\n",
    "Phase 6 remains the read-only runtime/gaming assessment and System-X-Ray authority layer. Phase 8 implements the bounded internal EXE/MSI runtime executor, but does **not** issue its opaque execution capability to external callers or expose a public runtime installation/repair CLI.\n\nPhase 7 does **not** expose online package acquisition, archive execution, public pack import/cleanup writes, or any new driver/security mutation authority.\n",
)

replace_once(
    "- runtime downloads and automatic vault/network package acquisition;\n- EXE/MSI/Winget runtime execution;\n- public vault import/cleanup mutation commands;\n",
    "- runtime downloads and automatic vault/network package acquisition;\n- public EXE/MSI runtime execution and any Winget runtime execution;\n- public issuance of Phase 8 `RuntimeExecutorCapability`;\n- public vault import/cleanup mutation commands;\n",
)

replace_once(
    "- runtime rollback claims before an executor-specific capture/verification/recovery contract exists;\n",
    "- generic runtime rollback claims for third-party installers without a proven package-specific restoration path;\n",
)

replace_once(
    "- `docs/decisions/0007-PHASE7-MANAGED-PACKAGE-VAULT.md`.\n",
    "- `docs/decisions/0007-PHASE7-MANAGED-PACKAGE-VAULT.md`;\n- `docs/decisions/0008-PHASE8-RUNTIME-EXECUTOR.md`.\n",
)

replace_once(
    "- Phase 7: **PROVEN and merged** at the managed local/offline package-vault + read-only public inspection boundary.\n",
    "- Phase 7: **PROVEN and merged** at the managed local/offline package-vault + read-only public inspection boundary.\n- Phase 8: **PROVEN and merged** at the bounded internal EXE/MSI runtime-executor + read-only public plan-validation boundary.\n",
)

closing_marker = "\nPhases 1–7 are closed at their recorded repository boundaries."
if text.count(closing_marker) != 1:
    raise SystemExit(f"expected one final Phase 1–7 closing marker, found {text.count(closing_marker)}")
closing_index = text.index(closing_marker)
proof_lines = """
- Phase 8 Windows review-correction pre-proof run `31697713764`: **PASS** across Phase 8 20/20, locked workspace build, Clippy with warnings denied, runtime-executor tests, catalogue tests, and diff validation before the temporary helper self-cleaned.
- Phase 8 Linux cfg-hygiene pre-proof run `31698343953`: **PASS** across Phase 8 20/20, locked workspace build, Clippy with warnings denied and no warning suppression, runtime-executor tests, and diff validation before the temporary helper self-cleaned.
- Phase 8 corrected implementation run `31698473273`: **PASS on Ubuntu and Windows** across Phase 1–8 static gates, lock integrity, rustfmt, locked workspace build, Clippy with warnings denied, complete workspace tests, Windows live read-only Runtime System X-Ray, and every applicable CLI fixture.
- Final Phase 8 documentation-state run `31698767919`: **PASS on Ubuntu and Windows** across the complete configured Phase 1–8 pipeline.
- Phase 8 PR #12 external review: **all review threads resolved or explicitly dispositioned as outdated; zero unresolved review threads at merge**.
- Phase 8 merged through PR #12 as `7a26d8d9dc86ac5f5db09eaf82b58424b1babd26`.
- Phase 8 live runtime-installer mutation proof: **not claimed**; CI compiled/tested the executor and ran read-only Runtime System X-Ray/fixtures but did not execute a real runtime installer.
- Phase 8 public runtime mutation proof: **not claimed**; the opaque execution capability is not publicly constructible or issued by the CLI.
"""
new_closing = """

Phases 1–8 are closed at their recorded repository boundaries. Phase 5 public driver mutation still requires live attached-device proof. Phase 8 public runtime mutation still requires a separately reviewed capability-issuance/live-installer proof path. Phase 7 network acquisition, archive execution, Windows-feature mutation, Winget execution, and public vault write surfaces remain independently blocked until their own authority, verification, cleanup, and recovery contracts are frozen and proven.
"""
text = text[:closing_index] + proof_lines + new_closing

PATH.write_text(text, encoding="utf-8")
print("Phase 8 canonical implementation status patched")

#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
path = ROOT / "docs/IMPLEMENTATION_STATUS.md"
text = path.read_text(encoding="utf-8")

phase12_current = "- **Phase 12:** merged and engineering-proven — typed MCP/RPC-first external authority over exactly the three Phase 11 low-risk reversible HKCU DWORD tweaks, with trusted transport context, exact caller/scoped permission policy, prepare→confirm/apply fingerprint continuity, crate-private capability issuance, single-use replay-resistant sessions, and no public CLI mutation bypass. Live Registry mutation proof remains unclaimed."

phase11_current_prefix = "- **Phase 11:** merged and engineering-proven"
if phase12_current not in text:
    lines = text.splitlines()
    for idx, line in enumerate(lines):
        if line.startswith(phase11_current_prefix):
            lines.insert(idx + 1, phase12_current)
            text = "\n".join(lines) + ("\n" if text.endswith("\n") else "")
            break
    else:
        raise SystemExit("Phase 11 current-state bullet not found")

baseline_marker = "Phase 11 corrected implementation run `31894350194` and final documentation-state run `31894626669` passed the complete Ubuntu and Windows Phase 1–11 pipeline with all three CodeRabbit correctness threads resolved, and Phase 11 merged through PR #19 as `66cca16be15fe617590445c6bb8993c5a242caf0`."
phase12_baseline = "Phase 12 exact implementation-head run `31899810049` passed the complete Ubuntu and Windows Phase 1–12 pipeline, including the Phase 12 authority gate, Clippy with warnings denied, full units/adversarial proof, Windows live read-only state proof, Runtime System X-Ray, and all applicable inherited fixtures. Phase 12 merged through PR #22 as `e762369e1f71a67ef51ce216af792fdd00e74ad5`. CodeRabbit was explicitly triggered but had published no review submission or inline review thread at the merge gate, so no external CodeRabbit PASS is claimed."
if phase12_baseline not in text:
    if baseline_marker not in text:
        raise SystemExit("Phase 11 proven-baseline marker not found")
    text = text.replace(baseline_marker, baseline_marker + "\n\n" + phase12_baseline, 1)

phase12_section = """## Phase 12 frozen implementation

Phase 12 exposes the first external machine-changing orchestration contract over the proven Phase 11 tweak executor while keeping the executor itself bounded to exactly the same three low-risk reversible HKCU DWORD actions.

It adds:

- canonical MCP/RPC-first machine-changing orchestration in `docs/NEO_DRIVER_MASTER_PLAN.md`;
- MCP tools `neo_tweaks_prepare` and `neo_tweaks_apply`;
- workstation/local RPC methods `neo.tweaks.prepare` and `neo.tweaks.apply` under schema `neo-rpc-v1`;
- exact caller-kind + principal policy with separate `neo.tweaks.prepare` and `neo.tweaks.low-risk.apply` permission scopes;
- trusted server-side caller/scope context that is not deserializable from untrusted request JSON;
- trusted service-instance identity plus checked monotonic session sequencing;
- prepare-time policy/scope validation before any live Registry read;
- bounded request/action cardinality at the exact three-action Phase 11 ceiling;
- actual baseline capture and exact Phase 4 transaction fingerprint returned for review;
- explicit confirmed apply bound to the same caller, exact fingerprint, and complete exact action set;
- reuse of the existing Phase 4 `TransactionAuthorization` rather than a parallel authorization model;
- crate-private `TweakExecutorCapability::for_rpc()` issuance only after all RPC authority gates pass;
- one outstanding prepared tweak plan per caller, with newer preparation invalidating older unconfirmed authority;
- single-use apply authority consumed before capability issuance, so execution failure requires a fresh prepare and current-state recapture;
- stable typed RPC error classification;
- no dependency from `neo-cli` to the mutation service and no public tweak-apply CLI command;
- Phase 12 deterministic 20-lane static review integrated into normal Ubuntu/Windows CI.

Independent authority review closed trusted-context deserialization, replay/session identity, action-vector cardinality, duplicated pending mission state, and Windows-only fake-host import-scope findings before final proof.

Phase 12 does **not** broaden the Registry catalogue, add arbitrary Registry editing, expose CLI/GUI mutation, issue runtime/driver mutation authority, use GitHub as an interactive execution transport, or claim live ATHENA Registry mutation proof.

The detailed authority contract is frozen in `docs/decisions/0012-PHASE12-MCP-RPC-TWEAK-AUTHORITY.md` and `docs/PHASE12_20_LANE_REVIEW.md`.

"""
if "## Phase 12 frozen implementation" not in text:
    marker = "## Still deliberately blocked"
    if marker not in text:
        raise SystemExit("Still deliberately blocked marker not found")
    text = text.replace(marker, phase12_section + marker, 1)

blocked_marker = "Phase 9 remains the platform-neutral desired/current-state assessment authority and Phase 10 remains the fixed reviewed read-only Windows state resolver. Phase 11 now binds exactly three curated HKCU DWORD tweaks into the proven transaction engine and implements an internal capability-gated Registry executor, but it does **not** expose public CLI/GUI/MCP-RPC apply authority, broader Registry/service/AppX/feature mutation, or claim live ATHENA Registry mutation proof."
blocked_replacement = "Phase 9 remains the platform-neutral desired/current-state assessment authority and Phase 10 remains the fixed reviewed read-only Windows state resolver. Phase 11 binds exactly three curated HKCU DWORD tweaks into the proven transaction engine. Phase 12 now exposes those exact three low-risk tweaks through the typed MCP/RPC authority service, but it does **not** broaden Registry scope, add public CLI mutation, expose an independent GUI mutation backend, issue runtime/driver mutation authority, or claim live ATHENA Registry mutation proof."
if blocked_marker in text:
    text = text.replace(blocked_marker, blocked_replacement, 1)
elif blocked_replacement not in text:
    raise SystemExit("Phase 11 blocked-surface paragraph not found")

req_marker = "- `docs/decisions/0011-PHASE11-TWEAK-EXECUTOR.md`;"
req_phase12 = "- `docs/decisions/0012-PHASE12-MCP-RPC-TWEAK-AUTHORITY.md`."
if req_phase12 not in text:
    if req_marker not in text:
        raise SystemExit("Decision 0011 requirement marker not found")
    text = text.replace(req_marker, req_marker + "\n" + req_phase12, 1)

proof_marker = "- Phase 11: **PROVEN and merged** at the internal capability-gated three-tweak HKCU DWORD transaction/executor boundary; no public apply capability or live Registry mutation proof is claimed."
proof_phase12 = "- Phase 12: **PROVEN and merged** at the typed MCP/RPC-first external authority boundary for exactly the three Phase 11 low-risk HKCU DWORD tweaks; no broader Registry/runtime/driver authority, public CLI mutation, or live Registry mutation proof is claimed."
if proof_phase12 not in text:
    if proof_marker not in text:
        raise SystemExit("Phase 11 proof-status marker not found")
    text = text.replace(proof_marker, proof_marker + "\n" + proof_phase12, 1)

phase11_public_marker = "- Phase 11 public tweak mutation proof: **not claimed**; CLI/GUI/MCP-RPC capability issuance remains blocked pending its own reviewed authority contract."
phase11_public_replacement = "- Phase 11 public CLI/GUI tweak mutation proof: **not claimed**; Phase 12 supplies the reviewed MCP/RPC authority contract instead of opening a CLI mutation bypass."
phase12_proof_entries = """- Phase 12 exact implementation-head run `31899810049`: **PASS on Ubuntu and Windows** across Phase 1–12 static gates, lock integrity, rustfmt, locked workspace build, Clippy with warnings denied, full workspace units/adversarial regressions, Windows live read-only state proof, Runtime System X-Ray, and every applicable inherited fixture.
- Phase 12 independent authority review: **5 bounded findings closed** — trusted-context deserialization, replay/session identity, action-vector cardinality, duplicated pending mission state, and Windows-only fake-host import scope.
- Phase 12 PR #22 external CodeRabbit disposition at merge: **review explicitly triggered; no review submission or inline review thread had been published, so no external CodeRabbit PASS is claimed**.
- Phase 12 final implementation PR surface: **9 intended files; no temporary proof/helper workflow, patch script, or diagnostic artifact**.
- Phase 12 merged through PR #22 as `e762369e1f71a67ef51ce216af792fdd00e74ad5`.
- Phase 12 live Registry mutation proof: **not claimed**; automated proof remains fake-host/adversarial for mutation and Windows live proof remains read-only.
- Phase 12 broader mutation proof: **not claimed**; arbitrary Registry, runtime/driver MCP authority, services/AppX/features/BCD/security mutation, public CLI mutation, and independent GUI mutation remain separately gated."""
if phase11_public_marker in text:
    text = text.replace(phase11_public_marker, phase11_public_replacement, 1)
if "- Phase 12 exact implementation-head run `31899810049`" not in text:
    if phase11_public_replacement not in text:
        raise SystemExit("Phase 11 public-proof marker not found")
    text = text.replace(phase11_public_replacement, phase11_public_replacement + "\n" + phase12_proof_entries, 1)

old_final_prefix = "Phases 1–11 are closed at their recorded repository boundaries."
new_final = "Phases 1–12 are closed at their recorded repository boundaries. Phase 12 is the canonical MCP/RPC-first external authority boundary for exactly the three Phase 11 low-risk reversible HKCU DWORD tweaks. Live Registry mutation proof, broader tweak/debloat domains, runtime/driver MCP authority, public CLI mutation, independent GUI mutation, Phase 7 network acquisition, archive execution, Windows-feature mutation, Winget execution, and public vault write surfaces remain independently blocked until their own authority, verification, cleanup, recovery, and proof contracts are frozen and proven."
if old_final_prefix in text:
    start = text.index(old_final_prefix)
    paragraph_end = text.find("\n", start)
    if paragraph_end == -1:
        paragraph_end = len(text)
    text = text[:start] + new_final + text[paragraph_end:]
elif new_final not in text:
    raise SystemExit("final Phase 1–11 summary paragraph not found")

path.write_text(text, encoding="utf-8")

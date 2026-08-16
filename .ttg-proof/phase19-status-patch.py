from pathlib import Path

root = Path(r"D:\projects\neo-host-setup\neo-phase19-20260816")
path = root / "docs" / "IMPLEMENTATION_STATUS.md"
text = path.read_text(encoding="utf-8")

phase19_bullet = "- **Phase 19:** merged, corrected, and engineering-proven — trusted append-only persistent Debloat removal history under the canonical Neo-managed `NeoData/history/debloat-removals` root, with write provenance restricted to completed Phase 16 execution-derived Phase 17 receipts, exact lowercase receipt-fingerprint record identities, bounded/versioned records, no-follow retained capabilities, marker-owned nested staging + atomic namespace promotion, concurrent identical-writer convergence, best-effort cleanup that preserves primary outcomes, canonical on-disk directory spelling, trusted by-ID restore-readiness selection, and explicit Unix/Windows substitution-proof coverage. Public GUI/CLI restore writes, MCP/RPC Debloat restore authority, AppX mutation from history, Store/network recovery, provisioning/all-users/batch restore, plugin dependency, same-principal cryptographic tamper resistance, platform-independent sudden-power-loss directory-metadata durability, and live destructive AppX restore proof remain blocked/unclaimed."
if phase19_bullet in text:
    raise SystemExit("Phase 19 current-state bullet already present")
lines = text.splitlines()
phase18_indexes = [i for i, line in enumerate(lines) if line.startswith("- **Phase 18:**")]
if len(phase18_indexes) != 1:
    raise SystemExit(f"expected one Phase 18 current-state bullet, found {len(phase18_indexes)}")
lines.insert(phase18_indexes[0] + 1, phase19_bullet)
text = "\n".join(lines) + ("\n" if text.endswith("\n") else "")

heading = "\n\n## Phase 5 frozen implementation"
if text.count(heading) != 1:
    raise SystemExit(f"expected one Phase 5 heading boundary, found {text.count(heading)}")
if "Phase 19 final exact implementation head" in text:
    raise SystemExit("Phase 19 baseline paragraph already present")
phase19_baseline = """Phase 19 final exact implementation head `2f8a6ba7003622cb9f1f1105bb710e7580cb0a3f` passed complete independent push run `31973316651` and PR run `31973318490` on Ubuntu and Windows. Internal Pass-B review first found and corrected a canonical on-disk record-directory alias issue. CodeRabbit then identified two actionable trust-boundary findings—cleanup result masking and missing Windows reparse substitution coverage—plus bounded-read, duplicated path-constant, and durability robustness observations. Final correction `2f8a6ba7003622cb9f1f1105bb710e7580cb0a3f` preserved primary write/promotion outcomes under staging cleanup failure, added Windows capability-aware substitution coverage and static binding, bounded reads with `Read::take`, centralized `HISTORY_DIRECTORY_NAME`, and explicitly bounded power-loss durability claims. CodeRabbit verified both actionable corrections and both threads were resolved; GitGuardian scanned all 29 implementation commits with no secrets detected. ATHENA Oracle proof `0000-neo-phase19-review-gap-fix5-20260816-2121z` passed Phase 17/18/19 static reviews 20/20, Clippy with warnings denied, 12/12 Phase 19 tests, and `neo-vault` 13/13 unit plus 1/1 concurrency tests. Physical Builder proof `0000-neo-phase19-final-builder-proof-20260816-2124z` built the exact Neo head with Builder `4c42e4822c1a811b5b999fafbfc00aedb0ac1a03`, produced `dist/windows/neo.exe`, and recorded Builder runtime smoke `--help` exit 0 / passed true with local target execution. Phase 19 merged through PR #39 as `9c16c3b89dc345610013e3bd4bbadf99d8048cf4`, and merged-main run `31973579063` then passed the complete Ubuntu and Windows Phase 1–19 pipeline, including live Windows Phase 14/15/10 evidence and all applicable fixtures. Public GUI/CLI restore writes, MCP/RPC Debloat restore authority, AppX mutation from retained history, Store/network recovery, provisioning/all-users/batch restore, plugin dependency, same-principal cryptographic tamper resistance, platform-independent sudden-power-loss directory-metadata durability, and live destructive AppX restore proof remain explicitly unclaimed."""
text = text.replace(heading, "\n\n" + phase19_baseline + heading)
path.write_text(text, encoding="utf-8")
print("PHASE19_STATUS_PATCH=PASS")

# Neo Driver — Implementation Status

## Current state

**Implementation active.**

- **Phase 1:** merged and engineering-proven — shared Rust core/CLI contracts + early read-only System X-Ray foundation.
- **Phase 2:** merged and engineering-proven — normalized device evidence + typed package catalogue contracts.
- **Phase 3:** merged and engineering-proven — deterministic read-only driver candidate matching/ranking.
- **Phase 4:** corrected implementation proven — transaction, checkpoint, verification, reboot/resume, and rollback foundation; no machine-changing executor attached. Final documentation-state CI remains before merge.

The master plan remains frozen. This file is the live implementation-status record.

## Proven baseline

Phase 1 merged as `e363ae8154c8319daaa40d9d1129b9db31029e5a`.

Phase 2 final documentation-state run `31615112238` passed all configured Ubuntu and Windows gates, all four major external-review code threads were resolved, and Phase 2 merged as `86493f6c69efb14beb2267e2e8a5534670346dc1`.

Phase 3 final documentation-state run `31619460283` passed all configured Ubuntu and Windows gates with no unresolved review thread, and Phase 3 merged as `76e45bd6166dee4f89eecac519cfafde8a4c47e5`.

## Phase 4 implemented

Phase 4 adds no machine mutation. It introduces:

- `neo-transaction`;
- exact immutable transaction plans with SHA-256 fingerprints;
- validated root deserialization for transaction plans/checkpoints so direct Serde callers cannot bypass invariants;
- typed state targets and actual pre-state snapshots;
- case-normalized identity for typed Windows state targets so case variants cannot bypass ownership/duplicate checks;
- fail-closed rejection of overlapping snapshot ownership across actions;
- exact fingerprint-bound user authority;
- manual-override, HIGH/EXPERT-risk, and irreversible-action acknowledgements;
- permanent rejection of `REJECTED` action evidence;
- deterministic postcondition predicates/results;
- persistent required-reboot checkpoints and post-reboot proof gates;
- explicit recovery from a blocked post-reboot state by re-probing, rolling back fully reversible changes, or failing closed;
- rollback-to-captured-state contracts and rollback verification gates;
- persisted checkpoint stage/event invariants;
- read-only transaction plan/checkpoint CLI validation;
- synthetic reversible transaction fixture and recursive Phase 4 20-lane review.

## Still deliberately blocked

Driver staging/install/removal, runtime install, downloads, debloat/tweaks execution, BCD/security mutation, actual reboot/resume execution, actual rollback writes, GUI, and Device Lab writes—including Apple/DFU Pro binding changes—remain blocked.

## Frozen technician requirements

Implementation continues to honor:

- `docs/decisions/0001-APPLE-TECHNICIAN-STACK.md`;
- `docs/decisions/0002-DFU-PRO-TECHNICIAN-DRIVERS.md`;
- `docs/decisions/0003-PHASE3-WINDOWS-MATCHING-CONTRACT.md`;
- `docs/decisions/0004-PHASE4-TRANSACTION-CONTRACT.md`.

## Proof status

- Phase 1: **PROVEN and merged**.
- Phase 2: **PROVEN and merged**.
- Phase 3: **PROVEN and merged**.
- Phase 4 Cargo.lock: recovered exactly from CI and committed; no dependency version selected by hand.
- Phase 4 rustfmt finding: corrected using stable rustfmt only.
- External review found three valid major gaps: blocked-state recovery, case-insensitive Windows target identity, and non-recursive Phase 4 source scanning. All three were corrected and the reviewer automatically resolved all three threads.
- Post-proof API review found raw root Serde deserialization could bypass plan/checkpoint validation. Both root types now deserialize through private validated wire types; direct-Serde regressions pass while `from_json_str()` preserves Neo's specific error taxonomy.
- GitHub Actions run `31642441507`: **PASS on Ubuntu and Windows**.
  - Phase 1 20-lane review: PASS on both.
  - Phase 2 20-lane review: PASS on both.
  - Phase 3 20-lane review: PASS on both.
  - Phase 4 recursive 20-lane review: PASS on both.
  - Cargo.lock committed/tracked/current graph: PASS on both.
  - Rust formatting: PASS on both.
  - Locked Rust type/build proof: PASS on both.
  - Clippy with warnings denied: PASS on both.
  - Rust unit regressions: PASS on both.
  - Read-only catalogue CLI fixture: PASS on both.
  - Read-only matcher CLI fixture: PASS on both.
  - Read-only transaction-plan CLI fixture: PASS on both.
  - Read-only transaction-checkpoint CLI fixture: PASS on both.
- External review threads: **0 unresolved** at implementation-code proof close.
- Live attached-device proof: **not claimed**.
- Machine mutation proof: **not applicable; mutation remains disabled**.

Phase 4 has no known unresolved correctness/security finding. One final CI run on this documentation-closed branch state is required before merge.

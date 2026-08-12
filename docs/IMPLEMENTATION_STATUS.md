# Neo Driver — Implementation Status

## Current state

**Implementation active.**

- **Phase 1:** merged and engineering-proven — shared Rust core/CLI contracts + early read-only System X-Ray foundation.
- **Phase 2:** merged and engineering-proven — normalized device evidence + typed package catalogue contracts.
- **Phase 3:** merged and engineering-proven — deterministic read-only driver candidate matching/ranking.
- **Phase 4:** corrected implementation under proof — transaction, checkpoint, verification, reboot/resume, and rollback foundation; no machine-changing executor attached.

The master plan remains frozen. This file is the live implementation-status record.

## Proven baseline

Phase 1 merged as `e363ae8154c8319daaa40d9d1129b9db31029e5a`.

Phase 2 final documentation-state run `31615112238` passed all configured Ubuntu and Windows gates, all four major external-review code threads were resolved, and Phase 2 merged as `86493f6c69efb14beb2267e2e8a5534670346dc1`.

Phase 3 final documentation-state run `31619460283` passed all configured Ubuntu and Windows gates with no unresolved review thread, and Phase 3 merged as `76e45bd6166dee4f89eecac519cfafde8a4c47e5`.

## Phase 4 implementation under proof

Phase 4 adds no machine mutation. It introduces:

- `neo-transaction`;
- exact immutable transaction plans with SHA-256 fingerprints;
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
- Phase 4 initial Phase 1–4 static-review gates: **PASS on Ubuntu and Windows** before the dependency lock gate.
- Phase 4 Cargo.lock: recovered exactly from CI and committed; no dependency version selected by hand.
- Phase 4 rustfmt finding: corrected using stable rustfmt only.
- External review found three valid major gaps: blocked-state recovery, case-insensitive Windows target identity, and non-recursive Phase 4 source scanning. All three are corrected in source and require full re-proof.
- Phase 4 Rust/Windows/Ubuntu proof after those corrections: **pending**.
- Live attached-device proof: **not claimed**.
- Machine mutation proof: **not applicable; mutation remains disabled**.

Phase 4 is not merge-ready until the corrected source passes the complete Windows/Ubuntu proof, external-review disposition, and final documentation-state gate.

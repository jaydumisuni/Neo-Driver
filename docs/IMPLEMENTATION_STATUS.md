# Neo Driver — Implementation Status

## Current state

**Implementation active.**

- **Phase 1:** merged and engineering-proven — shared Rust core/CLI contracts + early read-only System X-Ray foundation.
- **Phase 2:** merged and engineering-proven — normalized device evidence + typed package catalogue contracts.
- **Phase 3:** merged and engineering-proven — deterministic read-only driver candidate matching/ranking.
- **Phase 4:** implementation under proof — transaction, checkpoint, verification, reboot/resume, and rollback foundation; no machine-changing executor attached.

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
- fail-closed rejection of overlapping snapshot ownership across actions;
- exact fingerprint-bound user authority;
- manual-override, HIGH/EXPERT-risk, and irreversible-action acknowledgements;
- permanent rejection of `REJECTED` action evidence;
- deterministic postcondition predicates/results;
- persistent required-reboot checkpoints and post-reboot proof gates;
- rollback-to-captured-state contracts and rollback verification gates;
- persisted checkpoint stage/event invariants;
- read-only transaction plan/checkpoint CLI validation;
- synthetic reversible transaction fixture and Phase 4 20-lane review.

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
- Phase 4 static 20-lane review: **PASS locally against the staged source contract**.
- Phase 4 Rust/Windows/Ubuntu proof: **pending GitHub CI**.
- Live attached-device proof: **not claimed**.
- Machine mutation proof: **not applicable; mutation remains disabled**.

Phase 4 is not merge-ready until the complete Windows/Ubuntu proof and final review gates pass.

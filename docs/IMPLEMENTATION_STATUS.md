# Neo Driver — Implementation Status

## Current state

**Implementation active.**

- **Phase 1:** merged and engineering-proven — shared Rust core/CLI contracts + early read-only System X-Ray foundation.
- **Phase 2:** merged and engineering-proven — normalized device evidence + typed package catalogue contracts.
- **Phase 3:** merged and engineering-proven — deterministic read-only driver candidate matching/ranking.
- **Phase 4:** next — transaction, checkpoint, verification, and rollback foundation before any machine-changing executor is allowed.

The master plan remains frozen. This file is the live implementation-status record.

## Proven baseline

Phase 1 merged as `e363ae8154c8319daaa40d9d1129b9db31029e5a`.

Phase 2 final documentation-state run `31615112238` passed all configured Ubuntu and Windows gates, all four major external-review code threads were resolved, and Phase 2 merged as `86493f6c69efb14beb2267e2e8a5534670346dc1`.

Phase 3 final documentation-state run `31619460283` passed the complete Ubuntu and Windows pipeline: Phase 1/2/3 20-lane reviews, tracked/current Cargo.lock, rustfmt, locked type/build, Clippy with warnings denied, unit regressions, catalogue CLI fixture, and matcher CLI fixture. PR #3 had no unresolved review threads and merged as `76e45bd6166dee4f89eecac519cfafde8a4c47e5`.

## Phase 3 proven capability

Phase 3 adds no machine mutation. It provides:

- `neo-match` deterministic read-only matcher;
- explicit Microsoft identifier match classes and checked identifier-score evidence;
- preservation of device hardware/compatible ID order;
- refined per-INF Models-entry catalogue structure and per-entry compatible-ID position;
- architecture/build hard gates;
- signature-state safety verdicts;
- conservative date/version tie-breakers after equal match quality;
- fail-closed handling for out-of-range identifier scores and incomplete tie-break metadata;
- explicit refusal to claim full Windows rank while exact signature-score/FeatureScore evidence is unavailable;
- read-only `neo match` CLI;
- matching fixtures and Phase 3 20-lane review.

## Still deliberately blocked

Driver staging/install/removal, runtime install, downloads, debloat/tweaks, BCD/security mutation, reboot/resume mutation, GUI, and Device Lab writes—including Apple/DFU Pro binding changes—remain blocked.

Rollback/transaction execution is also blocked until Phase 4 first proves the typed transaction state machine, pre-state capture contracts, verification predicates, persistent checkpoints, and rollback records without attaching a real Windows mutator.

## Frozen technician requirements

Implementation continues to honor:

- `docs/decisions/0001-APPLE-TECHNICIAN-STACK.md`;
- `docs/decisions/0002-DFU-PRO-TECHNICIAN-DRIVERS.md`;
- `docs/decisions/0003-PHASE3-WINDOWS-MATCHING-CONTRACT.md`.

## Proof status

- Phase 1: **PROVEN and merged**.
- Phase 2: **PROVEN and merged**.
- Phase 3: **PROVEN and merged**.
- Phase 3 final CI run `31619460283`: **PASS on Ubuntu and Windows**.
- External CodeRabbit review during Phase 3 was rate-limited; no full external-review PASS is claimed and no code review threads were produced.
- CodeRabbit docstring-coverage warning remains visible as a non-functional documentation-quality warning; it is not treated as a correctness/security proof gate.
- Live attached-device proof: **not claimed**.
- Machine mutation proof: **not applicable yet; mutation remains disabled**.

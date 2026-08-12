# Neo Driver — Implementation Status

## Current state

**Implementation active.**

- **Phase 1:** merged and engineering-proven — shared Rust core/CLI contracts + early read-only System X-Ray foundation.
- **Phase 2:** merged and engineering-proven — normalized device evidence + typed package catalogue contracts.
- **Phase 3:** corrected implementation proven — deterministic read-only driver candidate matching/ranking. Final documentation-state CI remains before merge.

The master plan remains frozen. This file is the live implementation-status record.

## Proven baseline

Phase 1 merged as `e363ae8154c8319daaa40d9d1129b9db31029e5a`.

Phase 2 final documentation-state run `31615112238` passed all configured Ubuntu and Windows gates, all four major external-review code threads were resolved, and Phase 2 merged as `86493f6c69efb14beb2267e2e8a5534670346dc1`.

## Phase 3 implemented

Phase 3 adds no machine mutation. It introduces:

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

Driver staging/install/removal, runtime install, downloads, debloat/tweaks, BCD/security mutation, reboot/resume mutation, rollback, GUI, and Device Lab writes—including Apple/DFU Pro binding changes—remain blocked.

## Frozen technician requirements

Implementation continues to honor:

- `docs/decisions/0001-APPLE-TECHNICIAN-STACK.md`;
- `docs/decisions/0002-DFU-PRO-TECHNICIAN-DRIVERS.md`;
- `docs/decisions/0003-PHASE3-WINDOWS-MATCHING-CONTRACT.md`.

## Proof status

- Phase 1: **PROVEN and merged**.
- Phase 2: **PROVEN and merged**.
- Phase 3 corrected implementation head: `7bd9471913cb13e583ff53293637d5a20c1dbe2e`.
- GitHub Actions run `31619245616`: **PASS on Ubuntu and Windows**.
  - Phase 1 20-lane review: PASS on both.
  - Strengthened Phase 2 20-lane review: PASS on both.
  - Phase 3 20-lane review: PASS on both.
  - Cargo.lock committed/tracked/current graph: PASS on both.
  - Rust formatting: PASS on both.
  - Locked Rust type/build proof: PASS on both.
  - Clippy with warnings denied: PASS on both.
  - Rust unit regressions: PASS on both.
  - Read-only catalogue CLI fixture: PASS on both.
  - Read-only matcher CLI fixture: PASS on both.
- External CodeRabbit review: **rate-limited during this cycle; no full external-review PASS claimed and no code review threads produced**.
- CodeRabbit docstring-coverage warning remains visible as a non-functional documentation-quality warning; it is not treated as a Phase 3 correctness/security proof gate.
- Live attached-device proof: **not claimed**.
- Machine mutation proof: **not applicable; mutation remains disabled**.

Phase 3 has no known unresolved correctness/security finding. One final CI run on the documentation-closed branch state is required before merge.

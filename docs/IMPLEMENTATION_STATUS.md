# Neo Driver — Implementation Status

## Current state

**Implementation active.**

- **Phase 1:** merged and engineering-proven — shared Rust core/CLI contracts + early read-only System X-Ray foundation.
- **Phase 2:** merged and engineering-proven — normalized device evidence + typed package catalogue contracts.
- **Phase 3:** implementation under proof — deterministic read-only driver candidate matching/ranking.

The master plan remains frozen. This file is the live implementation-status record.

## Proven baseline

Phase 1 merged as `e363ae8154c8319daaa40d9d1129b9db31029e5a`.

Phase 2 final documentation-state run `31615112238` passed all configured Ubuntu and Windows gates, all four major external-review code threads were resolved, and Phase 2 merged as `86493f6c69efb14beb2267e2e8a5534670346dc1`.

## Phase 3 implementation under proof

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
- Phase 3 targeted pre-publication checks: **PASS**; full Windows/Ubuntu workspace proof pending.
- Live attached-device proof: **not claimed**.
- Machine mutation proof: **not applicable; mutation remains disabled**.

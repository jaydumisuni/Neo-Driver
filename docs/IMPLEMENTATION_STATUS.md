# Neo Driver — Implementation Status

## Current state

**Implementation started.**

Current phase: **Phase 1 — shared Rust core/CLI contracts + early read-only System X-Ray foundation.**

The master plan remains frozen. The `Implementation status: NOT STARTED` line in the original architecture-freeze header records the state at freeze time; this file is the live implementation-status record.

## Implemented in this increment

- Rust workspace foundation.
- `neo-core` model-free product contracts.
- Three user-depth contracts: Beginner, Standard, Expert.
- Frozen user-intent contracts.
- Recommendation and evidence-verdict contracts.
- Typed risk/action/reboot/mission-stage contracts.
- Manual-authority invariants in plan validation.
- Mandatory mutation rationale and supporting evidence.
- Fail-closed prevention of preselecting HIGH/EXPERT-risk actions.
- Only CERTIFIED actions can be selected by default.
- Fail-closed prevention of default-selecting conflict/unsupported/DO-NOT-TOUCH/unknown actions.
- Duplicate mission action-ID rejection.
- Machine-profile contracts for OS/security evidence.
- `neo-probe` read-only command-evidence abstraction.
- Windows identity read probe.
- Native Windows architecture detection resilient to WOW64 environment variables.
- Test Signing and `nointegritychecks` represented as separate persistent BCD states.
- Secure Boot state probe foundation.
- Memory Integrity/HVCI state probe foundation.
- Pending-reboot evidence from CBS, Windows Update, and pending file rename indicators.
- Connected-device and problem-device evidence collection foundations.
- Driver Store read evidence collection foundation.
- Failure-honest probe continuation: one unavailable read command does not erase other evidence lanes.
- `neo` CLI foundation with `scan`, `plan`, and `status`.
- JSON output path for machine-readable integration.
- Unit fixtures for critical policy/parsing rules.
- Reproducible 20-lane Phase 1 static review tool.
- Windows + Linux CI definition for static review, formatting, build/type proof, Clippy, and unit tests.
- Explicit Phase 1 no-mutation boundary.

## Deliberately not implemented yet

- driver installation;
- runtime installation;
- package downloads;
- catalogue ingestion;
- candidate matching/ranking;
- debloat/tweak execution;
- security/BCD mutation;
- reboot/resume mutation workflow;
- rollback transactions;
- GUI;
- Device Lab writes.

These remain blocked by the planned transaction, authority, catalogue, and verification layers.

## Apple / technician requirements already frozen

Implementation must honor:

- `docs/decisions/0001-APPLE-TECHNICIAN-STACK.md`;
- `docs/decisions/0002-DFU-PRO-TECHNICIAN-DRIVERS.md`.

Those requirements will enter Device Lab implementation after the core transaction/catalogue safety layers exist.

## Proof status

- Deterministic local Phase 1 static review: **20/20 PASS**.
- GitHub Actions implementation-code proof (`e7962c1812f434c80a68f936894c319e38569346`, run `31589296740`): **PASS on Ubuntu and Windows**.
  - 20-lane static review: PASS on both.
  - Rust formatting proof: PASS on both.
  - Rust type/build proof: PASS on both.
  - Clippy with warnings denied: PASS on both.
  - Rust unit tests: PASS on both.
- External CodeRabbit review: **unavailable due provider rate limit; no external code finding claimed**.
- Local Rust compilation/runtime proof: **not available in this workspace** because Rust is not installed.
- Windows-specific live hardware probe proof: **not yet claimed**; GitHub Windows CI proves build/tests, not real attached-device behavior.

Phase 1 is engineering-proven for its current code/contracts and remains read-only. It is not a product release and does not yet prove live hardware coverage.

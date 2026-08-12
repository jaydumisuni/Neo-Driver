# Neo Driver — Implementation Status

## Current state

**Implementation active.**

- **Phase 1:** merged and engineering-proven — shared Rust core/CLI contracts + early read-only System X-Ray foundation.
- **Phase 2:** merged and engineering-proven — normalized device evidence + typed package catalogue contracts.
- **Phase 3:** next — deterministic candidate matching/ranking, still read-only.

The master plan remains frozen. This file is the live implementation-status record.

## Phase 1 proven baseline

Phase 1 established the model-free Rust core, three user-depth authority contracts, manual/risk invariants, read-only Windows identity/security/PnP/Driver Store evidence, CLI foundation, 20-lane static review, and Windows + Ubuntu proof. It merged to `main` as `e363ae8154c8319daaa40d9d1129b9db31029e5a`.

## Phase 2 proven baseline

Phase 2 adds, without machine mutation:

- `neo-device` normalized device evidence;
- ordered opaque hardware/compatible IDs;
- validated device/inventory deserialization;
- instance/problem/active-driver/service/filter evidence;
- `neo-catalogue` typed package manifests;
- provenance + SHA-256 + redistribution policy;
- per-INF applicability + catalogue/signature/signer evidence;
- Windows architecture/build applicability;
- dependency/conflict validation including unresolved-reference rejection;
- explicit security target states and reboot validation;
- read-only `neo catalogue validate <file>`;
- synthetic catalogue fixture;
- workspace-wide Phase 2 20-lane review;
- committed and Git-tracked Cargo.lock proof gate.

Phase 2 final documentation-state GitHub Actions run `31615112238` passed all configured gates on Ubuntu and Windows. All four major external-review code threads were resolved. Phase 2 merged to `main` as `86493f6c69efb14beb2267e2e8a5534670346dc1`.

## Still deliberately blocked

Driver staging/install/removal, runtime install, downloads, debloat/tweaks, BCD/security mutation, reboot/resume mutation, rollback, GUI, and Device Lab writes—including Apple/DFU Pro binding changes—remain blocked.

Candidate matching/ranking is the next read-only layer and does not grant install authority.

## Frozen technician requirements

Implementation continues to honor `docs/decisions/0001-APPLE-TECHNICIAN-STACK.md` and `docs/decisions/0002-DFU-PRO-TECHNICIAN-DRIVERS.md`.

## Proof status

- Phase 1: **PROVEN and merged**.
- Phase 2: **PROVEN and merged**.
- Phase 2 final CI run `31615112238`: PASS on Ubuntu and Windows for Phase 1 review, strengthened Phase 2 review, Cargo.lock tracking/current graph, rustfmt, locked build/type checks, Clippy with warnings denied, Rust unit tests, and the read-only catalogue CLI fixture.
- CodeRabbit docstring-coverage warning remains a visible non-functional documentation-quality warning; it is not represented as a correctness/security PASS.
- Live attached-device proof: **not claimed**.
- Machine mutation proof: **not applicable yet; mutation remains disabled**.

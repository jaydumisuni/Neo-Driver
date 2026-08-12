# Neo Driver — Implementation Status

## Current state

**Implementation active.**

- **Phase 1:** merged and engineering-proven — shared Rust core/CLI contracts + early read-only System X-Ray foundation.
- **Phase 2:** implementation-code proof complete — normalized device evidence + typed package catalogue contracts. Final documentation-state CI is the remaining merge gate.

The master plan remains frozen. This file is the live implementation-status record.

## Phase 1 proven baseline

Phase 1 established the model-free Rust core, three user-depth authority contracts, manual/risk invariants, read-only Windows identity/security/PnP/Driver Store evidence, CLI foundation, 20-lane static review, and Windows + Ubuntu proof. It merged to `main` as `e363ae8154c8319daaa40d9d1129b9db31029e5a`.

## Phase 2 implemented

Without machine mutation, Phase 2 adds:

- `neo-device` normalized device evidence;
- ordered opaque hardware/compatible IDs;
- instance/problem/active-driver/service/filter evidence;
- `neo-catalogue` typed package manifests;
- provenance + SHA-256 + redistribution policy;
- per-INF applicability + catalogue/signature/signer evidence;
- Windows architecture/build applicability;
- dependency/conflict validation;
- explicit security target states and reboot validation;
- read-only `neo catalogue validate <file>`;
- synthetic catalogue fixture;
- Phase 2 20-lane review;
- committed Cargo.lock drift/proof gate.

## Still deliberately blocked

Driver staging/install/removal, runtime install, downloads, candidate matching/ranking, debloat/tweaks, BCD/security mutation, reboot/resume mutation, rollback, GUI, and Device Lab writes—including Apple/DFU Pro binding changes—remain blocked.

## Frozen technician requirements

Implementation continues to honor `docs/decisions/0001-APPLE-TECHNICIAN-STACK.md` and `docs/decisions/0002-DFU-PRO-TECHNICIAN-DRIVERS.md`.

## Proof status

- Phase 1: **PROVEN and merged**.
- Phase 2 deterministic static review: **20/20 PASS**.
- Phase 2 implementation-code proof commit: `a2c6453ea4fe5e20ea5ab4da7d7894530612c777`.
- GitHub Actions run `31613855813`: **PASS on Ubuntu and Windows**.
  - Phase 1 20-lane review: PASS on both.
  - Phase 2 20-lane review: PASS on both.
  - Cargo.lock integrity/current graph: PASS on both.
  - Rust formatting: PASS on both.
  - Locked Rust type/build proof: PASS on both.
  - Clippy with warnings denied: PASS on both.
  - Rust unit tests: PASS on both.
  - Read-only catalogue CLI fixture: PASS on both.
- Live attached-device proof: **not claimed**.
- Machine mutation proof: **not applicable yet; mutation remains disabled**.

Phase 2 has no known implementation-code proof gap. A final CI run on the documentation-closed branch state is required before merge.

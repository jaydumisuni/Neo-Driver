# Neo Driver — Implementation Status

## Current state

**Implementation active.**

- **Phase 1:** merged and engineering-proven — shared Rust core/CLI contracts + early read-only System X-Ray foundation.
- **Phase 2:** corrected after external review; full re-proof pending — normalized device evidence + typed package catalogue contracts.

The master plan remains frozen. This file is the live implementation-status record.

## Phase 1 proven baseline

Phase 1 established the model-free Rust core, three user-depth authority contracts, manual/risk invariants, read-only Windows identity/security/PnP/Driver Store evidence, CLI foundation, 20-lane static review, and Windows + Ubuntu proof. It merged to `main` as `e363ae8154c8319daaa40d9d1129b9db31029e5a`.

## Phase 2 implemented under re-proof

Without machine mutation, Phase 2 adds:

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

## Still deliberately blocked

Driver staging/install/removal, runtime install, downloads, candidate matching/ranking, debloat/tweaks, BCD/security mutation, reboot/resume mutation, rollback, GUI, and Device Lab writes—including Apple/DFU Pro binding changes—remain blocked.

## Frozen technician requirements

Implementation continues to honor `docs/decisions/0001-APPLE-TECHNICIAN-STACK.md` and `docs/decisions/0002-DFU-PRO-TECHNICIAN-DRIVERS.md`.

## Proof status

- Phase 1: **PROVEN and merged**.
- Phase 2 earlier implementation-code run `31613855813`: passed configured Windows + Ubuntu gates but was later reopened by four valid external-review findings.
- Findings corrected: unresolved catalogue references, deserialization validation bypass, untracked-lock acceptance, and incomplete workspace scan scope.
- Corrected Phase 2 Windows/Ubuntu proof: **pending**.
- Live attached-device proof: **not claimed**.
- Machine mutation proof: **not applicable yet; mutation remains disabled**.

Phase 2 cannot merge until corrected-code CI is green and external review threads are resolved or explicitly dispositioned.

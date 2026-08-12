# Neo Driver — Implementation Status

## Current state

**Implementation active.**

- **Phase 1:** merged and engineering-proven — shared Rust core/CLI contracts + early read-only System X-Ray foundation.
- **Phase 2:** merged and engineering-proven — normalized device evidence + typed package catalogue contracts.
- **Phase 3:** merged and engineering-proven — deterministic read-only driver candidate matching/ranking.
- **Phase 4:** merged and engineering-proven — transaction, checkpoint, verification, reboot/resume, and rollback foundation.
- **Phase 5:** implementation frozen under full proof — controlled, manually selected Windows driver installation bound to the proven matcher + transaction engine; mutation engine remains internal pending live attached-device proof.

The master plan remains frozen. This file is the live implementation-status record.

## Proven baseline

Phase 1 merged as `e363ae8154c8319daaa40d9d1129b9db31029e5a`.

Phase 2 final documentation-state run `31615112238` passed all configured Ubuntu and Windows gates, all four major external-review code threads were resolved, and Phase 2 merged as `86493f6c69efb14beb2267e2e8a5534670346dc1`.

Phase 3 final documentation-state run `31619460283` passed all configured Ubuntu and Windows gates with no unresolved review thread, and Phase 3 merged as `76e45bd6166dee4f89eecac519cfafde8a4c47e5`.

Phase 4 final documentation-state run `31642625013` passed the complete Ubuntu and Windows pipeline with zero unresolved review threads, and Phase 4 merged as `bc9712a47e27a5930b918b45dcc65a48e62f70ae`.

## Phase 5 frozen implementation

Phase 5 introduces the first machine-changing backend, but keeps that backend behind library/transaction authority rather than exposing a CLI write command.

It adds:

- `neo-driverstore`;
- typed, root-validated `DriverInstallPlan` and persisted `DriverInstallSession` contracts;
- exact source-INF SHA-256 and canonical in-package path authority;
- Windows `SetupVerifyInfFileW` re-verification of the actual selected INF/catalogue;
- exact-INF Windows compatibility enumeration and exact equality with Neo catalogue/matcher impact;
- exact active-binding/problem-code baseline for every impacted device;
- exact resolved baseline Driver Store package for every impacted device before reversible authority;
- preflight re-proof of source bytes, signature, impact set, bindings, baseline packages, and target-store baseline immediately before mutation;
- exact target staging with Windows-published OEM INF/Driver Store identity;
- per-authorized-device forward best-match installation: Windows selects each device's best preinstalled match and Neo supplies no specific driver node;
- explicit exclusion of force-install and force-delete paths;
- typed outside-authority blast-radius failure;
- separation of API outcome from observed `machine_changed` evidence;
- healthy Windows no-op handling that removes an unused newly staged package when the target package was absent at baseline;
- conservative rollback routing when post-write observation fails;
- runtime install and rollback reboot evidence bound into persistent checkpoints;
- exact rollback to each captured baseline published package using a specific driver node only in rollback;
- non-force removal of only the exact target package Neo introduced, only after it is no longer in use;
- retryable verification and rollback-verification probes so observation failure cannot strand a valid persisted stage;
- Windows fail-closed handling for ConfigMgr device-status query failure;
- strict `oem<digits>.inf` published-name validation;
- Phase 5 20-lane static review integrated into normal Ubuntu/Windows CI.

## Phase 5 review findings closed before freeze

Engineering review found and corrected the following before the implementation was frozen:

1. Runtime reboot evidence was not represented by the Phase 4 transaction record. Apply/rollback records now carry runtime reboot evidence and rollback has a persistent reboot checkpoint.
2. API success and actual machine mutation were conflated. `machine_changed` is now independent evidence and rollback obligation follows observed change.
3. A baseline published INF name alone did not prove rollback availability. Every impacted device must resolve to an exact baseline Driver Store package before authority and again at preflight.
4. A global forward install could include a device that appeared after authority. Forward installation is now per captured/authorized instance ID while Windows still chooses the best preinstalled match.
5. Windows ConfigMgr status-query failure could be misread as a healthy device. The inventory now fails closed.
6. `oem.inf` could pass a loose published-name validator. The validator now requires a non-empty numeric OEM index.
7. Post-mutation inventory/Driver Store observation failure could escape while the transaction remained `Applying`. It is now conservatively recorded as changed and routed into recovery.
8. Verification/rollback verification could become stranded after a transient probe error. Explicit retry entry points now preserve the persisted state machine.
9. Blast-radius violation used a free-form message despite an existing typed error. The path now uses `UnexpectedBindingChange`, and Phase 5's lane 13 binds to that typed contract.

## Still deliberately blocked

Phase 5 does **not** expose a user/technician mutation CLI yet. Live attached-device mutation proof is required before that public write surface is opened.

The following also remain blocked:

- forced lower-ranked driver binding;
- force Driver Store deletion or broad stale-package cleanup;
- blanket USB/filter replacement;
- driver downloads and runtime installation;
- debloat/tweak execution;
- BCD/security mutation;
- GUI write actions;
- Device Lab writes, including Apple/DFU Pro binding changes.

## Frozen technician requirements

Implementation continues to honor:

- `docs/decisions/0001-APPLE-TECHNICIAN-STACK.md`;
- `docs/decisions/0002-DFU-PRO-TECHNICIAN-DRIVERS.md`;
- `docs/decisions/0003-PHASE3-WINDOWS-MATCHING-CONTRACT.md`;
- `docs/decisions/0004-PHASE4-TRANSACTION-CONTRACT.md`;
- `docs/decisions/0005-PHASE5-CONTROLLED-DRIVER-INSTALL.md`.

## Proof status

- Phase 1: **PROVEN and merged**.
- Phase 2: **PROVEN and merged**.
- Phase 3: **PROVEN and merged**.
- Phase 4: **PROVEN and merged**.
- Phase 5 platform-neutral transaction/driverstore core: compiler/Clippy/adversarial proof passed before freeze.
- Phase 5 Windows SetupAPI/NewDev backend: Windows compiler proof passed; Windows Clippy with warnings denied passed; Windows-specific validation regressions passed.
- Windows fail-closed observation correction run `31650621429`: **PASS**.
- Post-mutation recovery correction run `31650739273`: **PASS** with inherited Phase 4 20/20, workspace compiler, Clippy, 29 transaction tests, and expanded driverstore regressions.
- Typed blast-radius/final Phase 5 contract run `31651046054`: **PASS** with Phase 4 20/20, Phase 5 20/20, workspace compiler, Clippy, transaction tests, and driverstore tests before commit.
- Phase 5 normal two-OS CI on the final frozen implementation state: **pending**.
- External review disposition on the final PR: **pending**.
- Final documentation-state CI: **pending**.
- Live attached-device mutation proof: **not claimed**.
- CI machine mutation proof: **not claimed; CI compiles/tests the backend but does not execute Windows-changing calls**.

Phase 5 is not merge-ready until the final frozen implementation passes normal Ubuntu/Windows CI, external-review disposition, and the final documentation-state gate.

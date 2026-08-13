# Neo Driver — Implementation Status

## Current state

**Implementation active.**

- **Phase 1:** merged and engineering-proven — shared Rust core/CLI contracts + early read-only System X-Ray foundation.
- **Phase 2:** merged and engineering-proven — normalized device evidence + typed package catalogue contracts.
- **Phase 3:** merged and engineering-proven — deterministic read-only driver candidate matching/ranking.
- **Phase 4:** merged and engineering-proven — transaction, checkpoint, verification, reboot/resume, and rollback foundation.
- **Phase 5:** merged and engineering-proven — controlled, manually selected Windows driver installation bound to the proven matcher + transaction engine; mutation engine remains internal pending live attached-device proof.

The master plan remains frozen. This file is the live implementation-status record.

## Proven baseline

Phase 1 merged as `e363ae8154c8319daaa40d9d1129b9db31029e5a`.

Phase 2 final documentation-state run `31615112238` passed all configured Ubuntu and Windows gates, all four major external-review code threads were resolved, and Phase 2 merged as `86493f6c69efb14beb2267e2e8a5534670346dc1`.

Phase 3 final documentation-state run `31619460283` passed all configured Ubuntu and Windows gates with no unresolved review thread, and Phase 3 merged as `76e45bd6166dee4f89eecac519cfafde8a4c47e5`.

Phase 4 final documentation-state run `31642625013` passed the complete Ubuntu and Windows pipeline with zero unresolved review threads, and Phase 4 merged as `bc9712a47e27a5930b918b45dcc65a48e62f70ae`.

Phase 5 final documentation-state run `31655706797` passed the complete Ubuntu and Windows pipeline with zero unresolved review threads and a clean 21-file PR surface, and Phase 5 merged through PR #5 as `d05bd65d39de283c446a40e8c5e7b78a485b4868`.

## Phase 5 frozen implementation

Phase 5 introduces the first machine-changing backend, but keeps that backend behind library/transaction authority rather than exposing a CLI write command.

It adds:

- `neo-driverstore`;
- typed, root-validated `DriverInstallPlan` and persisted `DriverInstallSession` contracts;
- exact source-INF SHA-256 and canonical in-package path authority;
- Windows `SetupVerifyInfFileW` re-verification of the actual selected INF/catalogue;
- exact-INF Windows compatibility enumeration and exact equality with Neo catalogue/matcher impact;
- actual host Windows-build verification at planning and again immediately before mutation;
- exact active-binding/problem-code baseline for every impacted device;
- exact resolved baseline Driver Store package for every impacted device before reversible authority;
- preflight re-proof of source bytes, host build, signature, impact set, bindings, baseline packages, and target-store baseline immediately before mutation;
- exact target staging with Windows-published OEM INF/Driver Store identity;
- Windows-equivalent staged-package detection requiring binary-identical INF **and identical catalog bytes**;
- per-authorized-device forward best-match installation: Windows selects each device's best preinstalled match and Neo supplies no specific driver node;
- explicit exclusion of force-install and force-delete paths;
- typed outside-authority blast-radius failure;
- separation of API outcome from observed `machine_changed` evidence, including outcome-aware compatibility for persisted legacy apply records;
- healthy Windows no-op handling that removes an unused newly staged package when the target package was absent at baseline;
- recovered staging-error compensation: a recovered and validated unused package may be removed to restore an absent Driver Store baseline, yielding operational `Failed` with zero net mutation; an unrecoverable staging identity remains conservatively changed and routes recovery;
- conservative rollback routing when staging or post-write observation leaves mutation uncertain;
- runtime install and rollback reboot evidence bound into persistent checkpoints whose type is derived from the enclosing transaction stage;
- exact rollback to each captured baseline published package using a specific driver node only in rollback;
- non-force removal of only the exact target package Neo introduced, only after it is no longer in use;
- retryable verification and rollback-verification probes so observation failure cannot strand a valid persisted stage;
- Windows fail-closed handling for ConfigMgr device-status query failure;
- trusted Windows-directory discovery through the Windows API rather than `%WINDIR%` environment state;
- locked `windows 0.62.2` device-property typing proven by Windows compilation: `DEVPKEY_Device_DriverInfPath` and `DEVPROPTYPE` from `Win32::Devices::Properties`, with the `DEVPROPKEY` parameter type from `Win32::Foundation`;
- strict `oem<digits>.inf` published-name validation;
- Phase 5 production-only static contract scanning, with regressions evaluated separately;
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
10. Existing-target equivalence matched binary-identical INF bytes plus signer/catalog metadata, while Windows permits identical INFs with different catalogs. Neo now requires identical catalog bytes as well, and preflight/staged-target validation reuses that exact equivalence rule.
11. A staging API failure could have changed Driver Store state before returning an error, while an unrecovered package identity could be mistaken for no change. Recovered identities are validated before use; any staging attempt without proven baseline restoration is conservatively treated as changed and routed into recovery.
12. Directly deserialized driver plans could retain lexical `..` traversal while still passing a prefix check. Root validation now rejects parent-directory components, and the regression is explicit.
13. Windows-directory discovery depended on process-controlled environment state. The backend now uses `GetWindowsDirectoryW`. A separate review suggestion moved `DEVPROPKEY` into `Win32::Devices::Properties`, but the locked Windows compiler disproved that binding shape; the compile-proven `Win32::Foundation::DEVPROPKEY` parameter type was restored while property constants/types remain in `Devices::Properties`.
14. Persisted reboot checkpoint `resume_stage` could influence whether a checkpoint was interpreted as apply or rollback. The expected checkpoint type is now derived from the enclosing validated transaction stage, so JSON cannot rebind the checkpoint class.
15. Legacy persisted `ApplyRecord` objects that omit `machine_changed` needed outcome-aware compatibility. Missing legacy values now resolve to `true` for historical success and `false` for historical failure, preserving the pre-Phase-5 transaction meaning instead of manufacturing a change on failed records.
16. The Phase 5 static aggregate included test source, allowing a production marker to be satisfied accidentally by a regression body. Production contract lanes now scan production Rust separately from `tests.rs`; regression-presence checks remain explicit.
17. The planner accepted a caller-supplied Windows build without proving it against the host. The host build is now read from trusted Windows registry state, compared during planning, and rechecked at preflight so build drift blocks mutation.
18. A regression originally required every recovered staging API failure to enter rollback even when validated compensation restored the Driver Store exactly to baseline. The regression now follows the transaction's net-mutation law: restored baseline yields `Failed` with zero net mutation; unrecovered/unproven staging state still enters recovery.

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
- Phase 5: **PROVEN and merged**.
- Phase 5 platform-neutral transaction/driverstore core: compiler/Clippy/adversarial proof passed before freeze.
- Phase 5 Windows SetupAPI/NewDev backend: Windows compiler proof passed; Windows Clippy with warnings denied passed; Windows-specific validation regressions passed.
- Windows fail-closed observation correction run `31650621429`: **PASS**.
- Post-mutation recovery correction run `31650739273`: **PASS** with inherited Phase 4 20/20, workspace compiler, Clippy, 29 transaction tests, and expanded driverstore regressions.
- Typed blast-radius/final Phase 5 contract run `31651046054`: **PASS** with Phase 4 20/20, Phase 5 20/20, workspace compiler, Clippy, transaction tests, and driverstore tests before commit.
- Normal pre-catalog-correction PR run `31651538698`: **PASS on Ubuntu and Windows** across Phase 1–5 gates, lock, rustfmt, locked build, Clippy, workspace tests, and all four proven CLI fixtures.
- Exact catalog-equivalence Windows pre-commit run `31651850209`: **PASS** across Phase 4/5 gates, workspace compiler, Clippy, transaction regressions, driverstore regressions, and diff check.
- Exact-catalog normal two-OS PR run `31651989426`: **PASS on Ubuntu and Windows** across all configured release gates.
- External-review correction pre-commit run `31652793990`: **PASS on Windows** across Phase 4/5 static gates, full workspace compiler, Clippy with warnings denied, transaction regressions, driverstore regressions, and diff check.
- CodeRabbit external review findings: **all resolved; zero unresolved review threads on the frozen PR**.
- Post-review Windows correction helper run `31655434637`: **PASS** across Phase 4/5 static gates, rustfmt, Windows workspace type proof, Clippy with warnings denied, the complete Windows unit suite, and diff proof; the temporary diagnostic/helper workflow self-cleaned before the correction commit `37f20177d0fa5ac3d7d7fd758d53c9d107771186`.
- Final corrected implementation normal PR run `31655563452`: **PASS on Ubuntu and Windows** across Phase 1–5 gates, lock integrity, rustfmt, locked workspace build, Clippy with warnings denied, all workspace tests, and all four proven CLI fixtures.
- Final Phase 5 documentation-state run `31655706797`: **PASS on Ubuntu and Windows** across the complete configured release pipeline.
- Final PR review disposition: **zero unresolved review threads**.
- Final Phase 5 PR surface: **21 intended files; no temporary diagnostic/helper workflow**.
- Phase 5 merged through PR #5 as `d05bd65d39de283c446a40e8c5e7b78a485b4868`.
- Live attached-device mutation proof: **not claimed**.
- CI machine mutation proof: **not claimed; CI compiles/tests the backend but does not execute Windows-changing calls**.

Phase 5 is complete at the repository implementation/proof boundary. The next gate before any public mutation surface is live attached-device proof under the existing authority and rollback contracts.

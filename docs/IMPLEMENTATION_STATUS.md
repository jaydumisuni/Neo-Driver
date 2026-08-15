# Neo Driver — Implementation Status

## Current state

**Implementation active.**

- **Phase 1:** merged and engineering-proven — shared Rust core/CLI contracts + early read-only System X-Ray foundation.
- **Phase 2:** merged and engineering-proven — normalized device evidence + typed package catalogue contracts.
- **Phase 3:** merged and engineering-proven — deterministic read-only driver candidate matching/ranking.
- **Phase 4:** merged and engineering-proven — transaction, checkpoint, verification, reboot/resume, and rollback foundation.
- **Phase 5:** merged and engineering-proven — controlled, manually selected Windows driver installation bound to the proven matcher + transaction engine; mutation engine remains internal pending live attached-device proof.
- **Phase 6:** merged and engineering-proven — deterministic runtime/gaming assessment + read-only Windows runtime System X-Ray, including compiled DirectX June 2010 legacy-component completeness evidence. Runtime execution remains a bounded child stage.
- **Phase 7:** merged and engineering-proven — Builder/portable-rooted managed package vault (`NeoData`) with verified local/offline pack intake, pinned TTG source provenance, no-follow filesystem authority, concurrent-promotion protection, marker-owned cleanup, and read-only public vault inspection. Network acquisition and public vault mutation remain blocked.
- **Phase 8:** merged and engineering-proven — bounded internal runtime executor for exact local/offline single-file runtime payloads, bound to Phase 6 Certified assessment, Phase 7 vault authority, and Phase 4 irreversible transaction/reboot verification. Public runtime mutation remains blocked because the opaque execution capability is not issued to external callers or the CLI.
- **Phase 9:** merged and engineering-proven — read-only, platform-neutral tweak/state assessment foundation with typed desired state, validated evidence/catalogue, explicit selection, deterministic current-vs-desired comparison, and behavioral non-mutation proof. OS-specific probing, transaction binding, and tweak execution remain blocked.
- **Phase 10:** merged and engineering-proven — read-only Windows live-state resolution layered on Phase 9, with validated target→reader bindings, fixed reviewed System-X-Ray reader identities, captured-state provenance, real Windows live-state behavioral proof, and zero tweak mutation authority. Transaction binding and tweak execution remain blocked.
- **Phase 11:** merged and engineering-proven — internal capability-gated transaction-bound execution for exactly three curated reversible HKCU DWORD tweaks, with exact semantic value binding, actual pre-state capture, same-session serialization, complete rollback-attempt evidence, direct Windows Registry APIs, and zero public tweak-apply authority. Live Registry mutation proof is not claimed.
- **Phase 12:** merged and engineering-proven — typed MCP/RPC-first external authority over exactly the three Phase 11 low-risk reversible HKCU DWORD tweaks, with trusted transport context, exact caller/scoped permission policy, prepare→confirm/apply fingerprint continuity, crate-private capability issuance, single-use replay-resistant sessions, and no public CLI mutation bypass. Live Registry mutation proof remains unclaimed.
- **Phase 13:** merged and engineering-proven — read-only, platform-neutral AppX/debloat assessment foundation with validated catalogue/evidence/profile/selection contracts, installed-vs-provisioned evidence separation, strict candidate/default policy, mandatory consequence metadata for non-safe classes, declared restore metadata, synthetic donor-isolated fixtures, and explicit byte-for-byte non-mutation proof. Real AppX probing, package/provisioning removal, restoration, and public GUI/CLI/MCP/RPC debloat mutation authority remain blocked.
- **Phase 14:** merged, corrected, and engineering-proven — read-only Windows current-user and online provisioned AppX inventory resolution over the frozen Phase 13 debloat model, using exactly two source-owned fixed PowerShell inventory scripts through the existing read-only command-evidence boundary, raw command-evidence retention, fail-closed `Unavailable` semantics for command/parse/identity uncertainty, exact-command regression proof, case-insensitive Rust identity matching, and successful real Windows live behavioral proof. Package/provisioning removal, restore/re-registration, transaction binding, public mutation CLI/GUI, plugin dependency, and MCP/RPC debloat authority remain blocked.

The master plan remains frozen. This file is the live implementation-status record.

## Proven baseline

Phase 1 merged as `e363ae8154c8319daaa40d9d1129b9db31029e5a`.

Phase 2 final documentation-state run `31615112238` passed all configured Ubuntu and Windows gates, all four major external-review code threads were resolved, and Phase 2 merged as `86493f6c69efb14beb2267e2e8a5534670346dc1`.

Phase 3 final documentation-state run `31619460283` passed all configured Ubuntu and Windows gates with no unresolved review thread, and Phase 3 merged as `76e45bd6166dee4f89eecac519cfafde8a4c47e5`.

Phase 4 final documentation-state run `31642625013` passed the complete Ubuntu and Windows pipeline with zero unresolved review threads, and Phase 4 merged as `bc9712a47e27a5930b918b45dcc65a48e62f70ae`.

Phase 5 final documentation-state run `31655706797` passed the complete Ubuntu and Windows pipeline with zero unresolved review threads and a clean 21-file PR surface, and Phase 5 merged through PR #5 as `d05bd65d39de283c446a40e8c5e7b78a485b4868`.

Phase 6 final documentation-state run `31684943307` passed the complete Ubuntu and Windows pipeline with zero unresolved inline review threads and a clean 19-file PR surface, and Phase 6 merged through PR #8 as `4747aafdb53b5731738fb99e08ddf2778c0d8707`.

Phase 7 final exact-head documentation-state run `31687570246` passed the complete Ubuntu and Windows Phase 1–7 pipeline, and Phase 7 merged through PR #10 as `bca02a8a294a976debcc26b480cea0c3ba4da2e2`.

Phase 8 final documentation-state run `31698767919` passed the complete Ubuntu and Windows Phase 1–8 pipeline with zero unresolved review threads, and Phase 8 merged through PR #12 as `7a26d8d9dc86ac5f5db09eaf82b58424b1babd26`.

Phase 9 implementation-code run `31715322010` and final documentation-state run `31715738064` passed the complete Ubuntu and Windows pipeline with all three CodeRabbit review threads resolved, and Phase 9 merged through PR #14 as `ad75a557f4787b9e1b902971b017cb71ce3ac511`.

Phase 10 corrected implementation run `31887310279` and final documentation-state run `31887513599` passed the complete Ubuntu and Windows Phase 1–10 pipeline with both Major CodeRabbit review threads resolved, including the real Windows live-state proof, and Phase 10 merged through PR #17 as `15b62fcbab8d400fd5497b422243b85d7f3d5595`.

Phase 11 corrected implementation run `31894350194` and final documentation-state run `31894626669` passed the complete Ubuntu and Windows Phase 1–11 pipeline with all three CodeRabbit correctness threads resolved, and Phase 11 merged through PR #19 as `66cca16be15fe617590445c6bb8993c5a242caf0`.

Phase 12 exact implementation-head run `31899810049` passed the complete Ubuntu and Windows Phase 1–12 pipeline, including the Phase 12 authority gate, Clippy with warnings denied, full units/adversarial proof, Windows live read-only state proof, Runtime System X-Ray, and all applicable inherited fixtures. Phase 12 merged through PR #22 as `e762369e1f71a67ef51ce216af792fdd00e74ad5`. Canonical documentation-state run `31900134525` then passed the same complete Ubuntu and Windows Phase 1–12 pipeline on the one-file status branch. CodeRabbit was explicitly triggered but had published no review submission or inline review thread at the implementation merge gate, so no external CodeRabbit PASS is claimed.

Phase 13 exact implementation-head run `31902121137` and PR run `31902280716` passed the complete Ubuntu and Windows Phase 1–13 pipeline, including the Phase 13 20-lane gate, lock/format/build/Clippy/units, explicit byte-for-byte non-mutation proof, Windows live read-only state proof, Runtime System X-Ray, the debloat assessment fixture, and all applicable inherited fixtures. GitGuardian passed with no secrets detected. Phase 13 merged through PR #24 as `4f85a3593bc0d74a7db46b52ddd09aaed58b5b6b`. CodeRabbit full review was explicitly requested; CodeRabbit reported its included-review quota was reached and published no review submission or inline review thread before the merge gate, so no external CodeRabbit PASS is claimed. Canonical documentation-state run `31902598020` then passed the same complete Ubuntu and Windows Phase 1–13 pipeline on the one-file status branch.

Phase 14 initial exact final-head push run `31904774997` and PR run `31904777612` passed the complete Ubuntu and Windows Phase 1–14 pipeline on `ad807443f3ceec3b38222ea10e2bfcf468faa0a9`, including successful real Windows current-user and provisioned AppX inventory execution. Initial implementation merged through PR #26 as `21b6f910435d3b22c2f142488616ae9cfeee1060`. CodeRabbit then completed its delayed review and identified two valid proof defects after that merge: the fake runner did not record actual command arguments, and the Phase 14 static gate was not tightly bound to the executable live-proof/CI/negative-plugin-policy contracts. The first documentation PR #27 was therefore closed unmerged as superseded. Correction PR #28 fixed both findings at exact head `81052420439facf953db480840c65ecc959fa6c7`; full correction run `31905339875` passed Ubuntu and Windows, including the real Phase 14 live AppX inventory lane, and the correction merged as `6980339207b7151d9000df21bd230551d2f9c27b`. Both original CodeRabbit review threads were then resolved. Corrected merged-main run `31905542475` passed the complete Ubuntu and Windows Phase 1–14 pipeline. GitGuardian passed on both the initial implementation lineage and the correction lineage with no secrets detected. CodeRabbit re-review of PR #28 could not start because its included-review quota was exhausted, so no separate CodeRabbit PASS is claimed for the correction PR.

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

## Phase 6 frozen implementation

Phase 6 adds a read-only runtime/gaming intelligence layer over the already-proven core. It does **not** add a runtime mutator.

It adds:

- `neo-runtime` normalized runtime evidence, profile readiness, package binding and reviewable action planning;
- `neo-runtime-probe` live Windows runtime System X-Ray over the existing read-only command-evidence boundary;
- `neo-directx-legacy` compiled DirectX June 2010 legacy framework-component completeness evidence;
- runtime states `Installed`, `Missing`, `Broken`, `Partial`, and `Unknown`;
- Fresh Windows, Gaming, Technician and Developer runtime profiles;
- modern baseline rules for Visual C++ 2015+ x86/x64 and a deselectable DirectX June 2010 baseline for Fresh Windows/Gaming;
- optional Python, XNA, OpenAL, PhysX/Legacy, .NET 3.5 and DirectPlay behavior where appropriate;
- explicit typed runtime-component → Neo package bindings that must target existing `PackageKind::Runtime` catalogue entries;
- architecture/build hard gates and ambiguity rejection before a runtime package may become a planned action;
- dependency/conflict edges blocking standalone action authority until dependency closure is implemented;
- individual user selection and confirmation; profile baseline preselection remains deselectable;
- unknown/ambiguous evidence never becoming install authority;
- failed registry queries remaining `Unknown` rather than being converted into false `Missing` states;
- live read-only evidence for VC++ v14, .NET Framework 4.x, modern .NET/Desktop runtimes, NetFx3, DirectPlay, WebView2 and conservative Python launcher/PATH state;
- .NET Framework `Release` evidence mapped through documented version thresholds while preserving the raw release value;
- DISM `Enable Pending` / `Disable Pending` states classified `Partial` until reboot completion;
- Python displayed version parsed from `py -0p` version tokens rather than executable paths;
- DirectX legacy completeness using trusted `GetWindowsDirectoryW` plus Microsoft's documented D3DCompiler/D3DCSX/D3DX/X3DAudio/XACT/XAPOFX/XAudio/XInput filename ranges;
- DirectX x64 completeness requiring both native System32 and x86 SysWOW64 component sets; ARM64 remains `Unknown` until its architecture-specific layout is proven;
- explicit separation of legacy DirectX presence/completeness from modern DirectX capability and from binary-health/corruption certification;
- XNA/OpenAL/PhysX/PhysX Legacy live predicates remaining `Unknown` until independently proven rather than guessed;
- read-only CLI surfaces `neo runtime-scan`, `neo runtimes`, and `neo gaming`;
- Phase 6 20-lane static review and normal Ubuntu/Windows proof integration.

The detailed Phase 6 review corrections are frozen in `docs/decisions/0006-PHASE6-RUNTIMES-GAMING.md`.

## Phase 7 frozen implementation

Phase 7 adds the managed package-vault boundary without replacing or weakening Phase 6. It does **not** add network acquisition or a public vault mutator.

It adds:

- `neo-vault` as a first-class workspace crate;
- application-root authority inherited from THETECHGUY Software Builder or the portable Neo folder;
- exactly one Neo-owned child, `NeoData`, with explicit `catalogue`, `driver-packs`, `packages`, `runtimes`, `staging`, `sessions`, `backups`, `logs`, and `cache` directories;
- one installed/portable package identity and storage model;
- validated `VaultSegment`, `Sha256Digest`, and `DriverSourceMap` identities, including direct-Serde validation;
- pinned provenance for the approved Android, Exynos/UsbDk, Apple Windows, and TechGuy driver-source families;
- source, staged, and promoted SHA-256 verification;
- unique import staging identities and exclusive final creation so concurrent same-pack imports cannot overwrite promoted content;
- exact staging ownership markers before cleanup authority exists;
- retained no-follow directory capabilities for traversal and promotion, closing path check/use link/reparse races;
- application-root and existing-tree audit that fails closed on unsafe link/reparse state;
- read-only public CLI surfaces `neo vault describe`, `neo vault validate-sources`, and `neo vault audit`;
- Phase 7 20-lane static review integrated beside the inherited Phase 1–6 gates.

The detailed Phase 7 contract and recovered engineering findings are frozen in `docs/decisions/0007-PHASE7-MANAGED-PACKAGE-VAULT.md` and `docs/PHASE7_20_LANE_REVIEW.md`.

## Phase 8 frozen implementation

Phase 8 adds a bounded internal runtime-execution boundary without replacing or weakening the read-only Phase 6 assessment/System-X-Ray layer or Phase 7 vault.

It adds:

- `neo-runtime-executor` as a separate first-class workspace crate;
- runtime-only execution metadata in catalogue manifests for exact EXE/MSI payload contracts;
- execution-plan preparation that re-derives authority from a Phase 6 `Certified` runtime recommendation;
- exact package ID/version/SHA evidence and Builder/portable-rooted Phase 7 vault path derivation;
- absolute application-root validation and direct-Serde revalidation of persisted execution plans;
- dependency/conflict and boot/security-mutation hard blocks before runtime authority exists;
- marker-owned no-follow staging through Phase 7 vault capabilities;
- direct EXE execution and trusted System32 `msiexec.exe` MSI execution with no shell path;
- MSI argument validation, including rejection of bare empty `PROPERTY=` assignments while preserving explicit `PROPERTY=""` semantics;
- Windows 32-bit exit-status bit-pattern preservation, including high-bit HRESULT/Win32 representations;
- bounded same-session cross-process serialization through a fixed `Local\` named mutex with timeout handling;
- locked staged-file SHA-256 re-verification immediately before process launch;
- conservative `machine_changed` evidence whenever an installer process starts;
- irreversible Phase 4 transaction authority with exact captured runtime baseline and mandatory acknowledgement;
- reboot/resume and post-install re-probe through the proven transaction checkpoint engine;
- retryable verification after transient runtime observation failure;
- an opaque `RuntimeExecutorCapability` with no public constructor, so safe external callers cannot invoke mutation even though validated plans/sessions remain inspectable;
- crate-private raw host/invocation/process/Windows-host adapters, closing the initial library-authority bypass found during PR review;
- read-only public CLI plan validation only; no public runtime install/apply command;
- Phase 8 20-lane review integrated beside inherited Phase 1–7 gates.

The detailed Phase 8 authority, review corrections, and proof boundary are frozen in `docs/decisions/0008-PHASE8-RUNTIME-EXECUTOR.md` and `docs/PHASE8_20_LANE_REVIEW.md`.

## Phase 9 frozen implementation

Phase 9 opens the Tweaks product domain at a deliberately read-only state-assessment boundary. It does **not** add a Windows tweak mutator.

It adds:

- `neo-state-plan` as a first-class, platform-neutral workspace crate;
- typed `TweakValue`, `TweakTarget`, `TweakOperation`, catalogue, evidence, observation, and assessment-report contracts;
- ASCII-only opaque state-target identities with deterministic case-insensitive canonicalization;
- direct-Serde validation for both tweak catalogues and supplied evidence;
- explicit risk, recommendation, verdict, default-selection, admin, reboot, benefit/trade-off, warning, and desired-state metadata;
- fail-closed rejection of duplicate catalogue IDs/targets, duplicate evidence targets, high-risk preselection, non-Certified preselection, unsafe recommendation preselection, duplicate/unknown/Rejected selections, and missing/unavailable observations;
- deterministic current-vs-desired comparison with explicit `already_satisfied` evidence;
- an internal `neo-state-assess` proof binary while the default product binary remains `neo`;
- a behavioral read-only regression that launches both Phase 9 subcommands against an isolated fixture tree and proves identical paths, file bytes, file/directory shape, and modification timestamps before and after;
- a CI-covered Rust contract that restricts the Phase 9 production state surface to its two existing JSON `read_to_string` calls, forbids process authority in production, and keeps the proof CLI free of filesystem/process authority;
- exact `1..=20` frozen review-lane sequence proof;
- all three CodeRabbit review findings closed: non-ASCII target identity, exact lane-sequence validation, and behavioral read-only proof.

Phase 9 remains assessment-only. It does not resolve abstract state targets to Windows registry/service/AppX/feature targets, probe those targets live, bind them into the transaction engine, or execute a tweak.

The detailed Phase 9 authority, review corrections, and proof boundary are frozen in `docs/decisions/0009-PHASE9-STATE-ASSESSMENT-FOUNDATION.md` and `docs/PHASE9_20_LANE_REVIEW.md`.

## Phase 10 frozen implementation

Phase 10 adds the first Windows live-state resolution layer for Tweaks while preserving the Phase 9 assessment boundary. It does **not** add a tweak mutator or transaction-bound write surface.

It adds:

- validated opaque `ReaderId` identities with direct-Serde revalidation;
- canonical Phase 9 target → reader bindings with duplicate-target rejection;
- validated captured-state roots with explicit provenance, duplicate-reader rejection, and revalidation before indexing even for directly constructed public values;
- deterministic resolution of approved captured reader evidence into the existing Phase 9 `TweakEvidence` contract;
- missing reader capture normalized to `ObservedState::Unavailable`, preserving Phase 9 fail-closed assessment;
- reuse of the proven `neo-probe::scan_current_machine()` System X-Ray boundary instead of introducing a second low-level Windows command surface;
- an exact fixed nine-reader Windows catalogue covering OS identity, Test Signing, no-integrity-checks, Secure Boot, Memory Integrity, and pending reboot evidence;
- unknown reader IDs returning unavailable evidence rather than executing fallback logic;
- read-only `neo-state-assess live` proof flow using validated domain JSON readers and the existing Phase 9 assessment engine;
- a real Windows behavioral proof that captures `windows.os.current_build`, reports `Machine changes: none`, and preserves an isolated fixture tree unchanged;
- a strengthened Phase 10 20-lane gate that structurally inspects Rust blocks, exact reader match arms, the frozen Phase 9 CLI blob, named regression tests, and active CI step definitions/commands before the executable unit/live proof chain runs;
- two Major CodeRabbit findings closed and re-proven: directly constructed captured-state validation and structural/executable 20-lane enforcement.

Phase 10 remains read-only. Registry/service/AppX/feature mutation, transaction binding, rollback, public tweak apply, and GUI write actions remain outside this boundary.

The detailed Phase 10 authority, review corrections, and proof boundary are frozen in `docs/decisions/0010-PHASE10-WINDOWS-STATE-RESOLUTION.md` and `docs/PHASE10_20_LANE_REVIEW.md`.

## Phase 11 frozen implementation

Phase 11 adds the first bounded Tweaks mutator over the Phase 9/10 state model while keeping mutation authority internal.

It adds:

- `neo-tweak-executor` as a first-class workspace crate;
- exactly three curated one-way HKCU DWORD mutations: show file extensions (`HideFileExt=0`), show hidden files (`Hidden=1`), and centered taskbar icons (`TaskbarAl=1`);
- fixed crate-private Registry paths/value names with no caller-supplied hive/subkey/value-name authority;
- exact approved forward DWORD binding per curated ID, including direct and persisted-plan revalidation;
- actual Registry baseline capture before authorization and a second drift check before apply;
- Phase 4 reversible transactions with exact postconditions and `MatchesBaseline` rollback verification;
- complete rollback attempts for every changed tweak through the additive shared `record_rollback_results_batch` contract before terminal rollback failure is decided;
- absent-state rollback by exact value deletion and present-state rollback to the exact captured DWORD;
- unsupported Registry type/size fail-closed behavior, including `ERROR_MORE_DATA` classification as unsupported state;
- direct Windows Registry APIs only, with no PowerShell, `reg.exe`, `cmd.exe`, shell, or arbitrary process path;
- a bounded same-session `Local\THETECHGUY.NeoDriver.TweakExecutor.v1` mutex covering the pre-apply recheck through write, verification, and rollback;
- a real Windows mutex acquire/release regression with no Registry mutation;
- an opaque `TweakExecutorCapability` with no public constructor;
- no `neo` CLI/GUI mutation command and no Phase 11 MCP/RPC capability issuance; future higher-level invocation remains behind a separately reviewed typed control-plane boundary;
- three CodeRabbit correctness findings closed and regression-bound: curated semantic value binding, complete multi-tweak rollback attempts, and oversized Registry-value classification.

CI compiles the real Windows Registry backend and exercises the real named mutex, but write/rollback behavior remains fake-host driven. Phase 11 does **not** claim live ATHENA Registry mutation proof.

The detailed Phase 11 authority, external-review corrections, and proof boundary are frozen in `docs/decisions/0011-PHASE11-TWEAK-EXECUTOR.md` and `docs/PHASE11_20_LANE_REVIEW.md`.

## Phase 12 frozen implementation

Phase 12 exposes the first external machine-changing orchestration contract over the proven Phase 11 tweak executor while keeping the executor itself bounded to exactly the same three low-risk reversible HKCU DWORD actions.

It adds:

- canonical MCP/RPC-first machine-changing orchestration in `docs/NEO_DRIVER_MASTER_PLAN.md`;
- MCP tools `neo_tweaks_prepare` and `neo_tweaks_apply`;
- workstation/local RPC methods `neo.tweaks.prepare` and `neo.tweaks.apply` under schema `neo-rpc-v1`;
- exact caller-kind + principal policy with separate `neo.tweaks.prepare` and `neo.tweaks.low-risk.apply` permission scopes;
- trusted server-side caller/scope context that is not deserializable from untrusted request JSON;
- trusted service-instance identity plus checked monotonic session sequencing;
- prepare-time policy/scope validation before any live Registry read;
- bounded request/action cardinality at the exact three-action Phase 11 ceiling;
- actual baseline capture and exact Phase 4 transaction fingerprint returned for review;
- explicit confirmed apply bound to the same caller, exact fingerprint, and complete exact action set;
- reuse of the existing Phase 4 `TransactionAuthorization` rather than a parallel authorization model;
- crate-private `TweakExecutorCapability::for_rpc()` issuance only after all RPC authority gates pass;
- one outstanding prepared tweak plan per caller, with newer preparation invalidating older unconfirmed authority;
- single-use apply authority consumed before capability issuance, so execution failure requires a fresh prepare and current-state recapture;
- stable typed RPC error classification;
- no dependency from `neo-cli` to the mutation service and no public tweak-apply CLI command;
- Phase 12 deterministic 20-lane static review integrated into normal Ubuntu/Windows CI.

Independent authority review closed trusted-context deserialization, replay/session identity, action-vector cardinality, duplicated pending mission state, and Windows-only fake-host import-scope findings before final proof.

Phase 12 does **not** broaden the Registry catalogue, add arbitrary Registry editing, expose CLI/GUI mutation, issue runtime/driver mutation authority, use GitHub as an interactive execution transport, or claim live ATHENA Registry mutation proof.

The detailed authority contract is frozen in `docs/decisions/0012-PHASE12-MCP-RPC-TWEAK-AUTHORITY.md` and `docs/PHASE12_20_LANE_REVIEW.md`.

## Still deliberately blocked

Phase 5 does **not** expose a user/technician driver mutation CLI yet. Live attached-device mutation proof is required before that public write surface is opened.

Phase 6 remains the read-only runtime/gaming assessment and System-X-Ray authority layer. Phase 8 implements the bounded internal EXE/MSI runtime executor, but does **not** issue its opaque execution capability to external callers or expose a public runtime installation/repair CLI.

Phase 7 does **not** expose online package acquisition, archive execution, public pack import/cleanup writes, or any new driver/security mutation authority.

Phase 9 remains the platform-neutral desired/current-state assessment authority and Phase 10 remains the fixed reviewed read-only Windows state resolver. Phase 11 binds exactly three curated HKCU DWORD tweaks into the proven transaction engine. Phase 12 now exposes those exact three low-risk tweaks through the typed MCP/RPC authority service, but it does **not** broaden Registry scope, add public CLI mutation, expose an independent GUI mutation backend, issue runtime/driver mutation authority, or claim live ATHENA Registry mutation proof.

The following remain blocked:

- runtime downloads and automatic vault/network package acquisition;
- public EXE/MSI runtime execution and any Winget runtime execution;
- public issuance of Phase 8 `RuntimeExecutorCapability`;
- public vault import/cleanup mutation commands;
- archive extraction/execution as install authority;
- .NET 3.5 or DirectPlay feature mutation;
- generic runtime rollback claims for third-party installers without a proven package-specific restoration path;
- forced lower-ranked driver binding;
- force Driver Store deletion or broad stale-package cleanup;
- blanket USB/filter replacement;
- public/general debloat or tweak execution beyond the internal Phase 11 three-tweak capability;
- BCD/security mutation;
- GUI write actions;
- Device Lab writes, including Apple/DFU Pro binding changes.

## Frozen technician requirements

Implementation continues to honor:

- `docs/decisions/0001-APPLE-TECHNICIAN-STACK.md`;
- `docs/decisions/0002-DFU-PRO-TECHNICIAN-DRIVERS.md`;
- `docs/decisions/0003-PHASE3-WINDOWS-MATCHING-CONTRACT.md`;
- `docs/decisions/0004-PHASE4-TRANSACTION-CONTRACT.md`;
- `docs/decisions/0005-PHASE5-CONTROLLED-DRIVER-INSTALL.md`;
- `docs/decisions/0006-PHASE6-RUNTIMES-GAMING.md`;
- `docs/decisions/0007-PHASE7-MANAGED-PACKAGE-VAULT.md`;
- `docs/decisions/0008-PHASE8-RUNTIME-EXECUTOR.md`;
- `docs/decisions/0009-PHASE9-STATE-ASSESSMENT-FOUNDATION.md`;
- `docs/decisions/0010-PHASE10-WINDOWS-STATE-RESOLUTION.md`;
- `docs/decisions/0011-PHASE11-TWEAK-EXECUTOR.md`.
- `docs/decisions/0012-PHASE12-MCP-RPC-TWEAK-AUTHORITY.md`.

## Proof status

- Phase 1: **PROVEN and merged**.
- Phase 2: **PROVEN and merged**.
- Phase 3: **PROVEN and merged**.
- Phase 4: **PROVEN and merged**.
- Phase 5: **PROVEN and merged**.
- Phase 6: **PROVEN and merged** at the read-only Runtime/Gaming System X-Ray + assessment boundary.
- Phase 7: **PROVEN and merged** at the managed local/offline package-vault + read-only public inspection boundary.
- Phase 8: **PROVEN and merged** at the bounded internal EXE/MSI runtime-executor + read-only public plan-validation boundary.
- Phase 9: **PROVEN and merged** at the read-only state-assessment foundation boundary.
- Phase 10: **PROVEN and merged** at the read-only Windows live-state resolution + Phase 9 assessment boundary.
- Phase 11: **PROVEN and merged** at the internal capability-gated three-tweak HKCU DWORD transaction/executor boundary; no public apply capability or live Registry mutation proof is claimed.
- Phase 12: **PROVEN and merged** at the typed MCP/RPC-first external authority boundary for exactly the three Phase 11 low-risk HKCU DWORD tweaks; no broader Registry/runtime/driver authority, public CLI mutation, or live Registry mutation proof is claimed.
- Phase 5 platform-neutral transaction/driverstore core: compiler/Clippy/adversarial proof passed before freeze.
- Phase 5 Windows SetupAPI/NewDev backend: Windows compiler proof passed; Windows Clippy with warnings denied passed; Windows-specific validation regressions passed.
- Windows fail-closed observation correction run `31650621429`: **PASS**.
- Post-mutation recovery correction run `31650739273`: **PASS** with inherited Phase 4 20/20, workspace compiler, Clippy, 29 transaction tests, and expanded driverstore regressions.
- Typed blast-radius/final Phase 5 contract run `31651046054`: **PASS** with Phase 4 20/20, Phase 5 20/20, workspace compiler, Clippy, transaction tests, and driverstore tests before commit.
- Normal pre-catalog-correction PR run `31651538698`: **PASS on Ubuntu and Windows** across Phase 1–5 gates, lock, rustfmt, locked build, Clippy, workspace tests, and all four proven CLI fixtures.
- Exact catalog-equivalence Windows pre-commit run `31651850209`: **PASS** across Phase 4/5 gates, workspace compiler, Clippy, transaction regressions, driverstore regressions, and diff check.
- Exact-catalog normal two-OS PR run `31651989426`: **PASS on Ubuntu and Windows** across all configured release gates.
- External-review correction pre-commit run `31652793990`: **PASS on Windows** across Phase 4/5 static gates, full workspace compiler, Clippy with warnings denied, transaction regressions, driverstore regressions, and diff check.
- CodeRabbit Phase 5 external review findings: **all resolved; zero unresolved review threads on the frozen PR**.
- Post-review Windows correction helper run `31655434637`: **PASS** across Phase 4/5 static gates, rustfmt, Windows workspace type proof, Clippy with warnings denied, the complete Windows unit suite, and diff proof; the temporary diagnostic/helper workflow self-cleaned before the correction commit `37f20177d0fa5ac3d7d7fd758d53c9d107771186`.
- Final corrected Phase 5 implementation normal PR run `31655563452`: **PASS on Ubuntu and Windows** across Phase 1–5 gates, lock integrity, rustfmt, locked workspace build, Clippy with warnings denied, all workspace tests, and all four proven CLI fixtures.
- Final Phase 5 documentation-state run `31655706797`: **PASS on Ubuntu and Windows** across the complete configured release pipeline.
- Final Phase 5 PR review disposition: **zero unresolved review threads**.
- Final Phase 5 PR surface: **21 intended files; no temporary diagnostic/helper workflow**.
- Phase 5 merged through PR #5 as `d05bd65d39de283c446a40e8c5e7b78a485b4868`.
- Live attached-device mutation proof: **not claimed**.
- CI machine mutation proof: **not claimed; CI compiles/tests the backend but does not execute Windows-changing calls**.
- Phase 6 DirectX integration pre-commit run `31682838602`: **PASS on Windows** across Phase 1–6 static reviews, workspace compiler, Clippy with warnings denied, all workspace unit tests, and self-cleaning proof helper.
- Phase 6 pre-review frozen implementation run `31683167998`: **PASS on Ubuntu and Windows** across Phase 1–6 static gates, lock integrity, rustfmt, locked workspace build, Clippy with warnings denied, all workspace tests, runtime/gaming/catalogue/matcher/transaction fixtures, plus a live Windows `neo runtime-scan --json` execution.
- Phase 6 live Windows runtime scan in run `31683167998`: VC++ v14 x86/x64 installed; DirectX June 2010 legacy framework set classified `Missing` with 0/180 expected x64+x86 files present on the clean runner; NetFx3, .NET Framework 4.x, modern .NET/Desktop, Python and WebView2 detected; DirectPlay disabled; XNA/OpenAL/PhysX/PhysX Legacy remained explicitly `Unknown`.
- Phase 6 independent-review correction pre-commit run `31684439927`: **PASS on Windows** across Phase 1–6 static gates, lock integrity, compiler, Clippy with warnings denied, all workspace units, live runtime scan, runtime fixture, and gaming fixture before the helper self-cleaned and committed `cb911bd72887e02d7d648294d793b9364c085d67`.
- Phase 6 corrected implementation normal PR run `31684665638`: **PASS on Ubuntu and Windows** across the complete configured Phase 1–6 release pipeline, including Windows live runtime System X-Ray and all CLI fixtures.
- Final Phase 6 documentation-state run `31684943307`: **PASS on Ubuntu and Windows** across the complete configured release pipeline.
- Phase 6 external CodeRabbit disposition: **full review unavailable because the reviewer was rate-limited; no external-review PASS is claimed**.
- Phase 6 unresolved inline review threads at merge: **0**.
- Phase 6 independent source challenge: **5 valid fail-closed findings closed and proven**; see Decision 0006.
- Final Phase 6 PR surface: **19 intended files; no temporary proof/helper workflows or scripts**.
- Phase 6 merged through PR #8 as `4747aafdb53b5731738fb99e08ddf2778c0d8707`.
- Phase 6 runtime mutation proof: **not claimed**.
- Phase 7 rebase preservation run `31686937725`: **Phase 1–6 20/20 on Ubuntu and Windows**; the only failure was an obsolete Phase 7 regression-name binding, which was corrected to the existing eight-worker concurrency regression.
- Phase 7 combined-lock helper run `31687136770`: **PASS**; Cargo generated the combined runtime + vault dependency graph, re-proved Phase 6/7, and the write-enabled helper self-deleted in the same commit.
- Phase 7 branch run `31687246986`: **PASS on Ubuntu and Windows** across the complete Phase 1–7 pipeline.
- Phase 7 PR run `31687289312`: **PASS on Ubuntu and Windows** across the complete Phase 1–7 pipeline.
- Phase 7 frozen-head run `31687514717`: **PASS on Ubuntu and Windows** across the complete Phase 1–7 pipeline.
- Final exact-head Phase 7 documentation-state run `31687570246`: **PASS on Ubuntu and Windows** across Phase 1–7 static gates, lock integrity, rustfmt, locked workspace build, Clippy with warnings denied, all workspace unit/integration tests, Windows live runtime System X-Ray, and every catalogue/matcher/runtime/gaming/vault/transaction fixture.
- Phase 7 prior CodeRabbit findings from stale PR #7: **3 Major findings resolved** — concurrent staging identity, invalid-root audit, and path TOCTOU/no-follow promotion.
- Phase 7 PR #10 fresh CodeRabbit review: **quota-blocked; no new external-review PASS is claimed**.
- Phase 7 unresolved PR #10 review threads at merge: **0**.
- Final Phase 7 PR surface: **18 intended files; no temporary proof/helper workflow**.
- Phase 7 merged through PR #10 as `bca02a8a294a976debcc26b480cea0c3ba4da2e2`.
- Phase 7 online acquisition/public vault mutation proof: **not claimed**.

- Phase 8 Windows review-correction pre-proof run `31697713764`: **PASS** across Phase 8 20/20, locked workspace build, Clippy with warnings denied, runtime-executor tests, catalogue tests, and diff validation before the temporary helper self-cleaned.
- Phase 8 Linux cfg-hygiene pre-proof run `31698343953`: **PASS** across Phase 8 20/20, locked workspace build, Clippy with warnings denied and no warning suppression, runtime-executor tests, and diff validation before the temporary helper self-cleaned.
- Phase 8 corrected implementation run `31698473273`: **PASS on Ubuntu and Windows** across Phase 1–8 static gates, lock integrity, rustfmt, locked workspace build, Clippy with warnings denied, complete workspace tests, Windows live read-only Runtime System X-Ray, and every applicable CLI fixture.
- Final Phase 8 documentation-state run `31698767919`: **PASS on Ubuntu and Windows** across the complete configured Phase 1–8 pipeline.
- Phase 8 PR #12 external review: **all review threads resolved or explicitly dispositioned as outdated; zero unresolved review threads at merge**.
- Phase 8 merged through PR #12 as `7a26d8d9dc86ac5f5db09eaf82b58424b1babd26`.
- Phase 8 live runtime-installer mutation proof: **not claimed**; CI compiled/tested the executor and ran read-only Runtime System X-Ray/fixtures but did not execute a real runtime installer.
- Phase 8 public runtime mutation proof: **not claimed**; the opaque execution capability is not publicly constructible or issued by the CLI.

- Phase 9 implementation-code run `31715322010`: **PASS on Ubuntu and Windows** across inherited Phase 1–8 gates, lock integrity, rustfmt, locked workspace build, Clippy with warnings denied, complete workspace tests, Windows live read-only Runtime System X-Ray, and every applicable inherited CLI fixture. The workspace unit proof executed `state_assess_subcommands_leave_isolated_fixture_tree_unchanged` on both OSes; the Windows log explicitly records that regression as `ok`.
- Final Phase 9 documentation-state run `31715738064`: **PASS on Ubuntu and Windows** across the complete configured pipeline.
- Phase 9 PR #14 external review: **3/3 CodeRabbit review threads resolved** — non-ASCII identity, exact 1..20 frozen lane sequence, and behavioral read-only proof.
- Phase 9 merged through PR #14 as `ad75a557f4787b9e1b902971b017cb71ce3ac511`.
- Phase 9 machine mutation proof: **not claimed**; Phase 9 is assessment-only and does not probe or change live Windows tweak state.

- Phase 10 corrected implementation run `31887310279`: **PASS on Ubuntu and Windows** across Phase 1–10 structural/static gates, lock integrity, rustfmt, locked build, Clippy with warnings denied, complete workspace units including direct-construction captured-state regressions, Windows live-state proof, Runtime System X-Ray, and every applicable fixture.
- Final Phase 10 documentation-state run `31887513599`: **PASS on Ubuntu and Windows** across the complete configured Phase 1–10 pipeline.
- Phase 10 PR #17 external review: **2/2 Major CodeRabbit threads resolved** — direct captured-state validation and structural/executable Phase 10 proof enforcement.
- Phase 10 merged through PR #17 as `15b62fcbab8d400fd5497b422243b85d7f3d5595`.
- Phase 10 machine mutation proof: **not claimed**; Phase 10 captures and resolves live Windows state read-only and does not execute a tweak.

- Phase 11 corrected implementation run `31894350194`: **PASS on Ubuntu and Windows** across Phase 1–11 static gates, lock integrity, rustfmt, locked build, Clippy with warnings denied, full workspace units/adversarial regressions, Windows live read-only state proof, Runtime System X-Ray, and every applicable fixture.
- Final Phase 11 documentation-state run `31894626669`: **PASS on Ubuntu and Windows** across the complete configured Phase 1–11 pipeline.
- Phase 11 PR #19 external review: **3/3 CodeRabbit correctness threads resolved** — exact curated DWORD semantic binding, complete rollback attempts/evidence for all changed tweaks, and `ERROR_MORE_DATA` fail-closed Registry-state classification.
- Phase 11 Windows synchronization proof: **real named mutex acquisition/release executed in CI without Registry mutation**.
- Phase 11 merged through PR #19 as `66cca16be15fe617590445c6bb8993c5a242caf0`.
- Phase 11 live Registry mutation proof: **not claimed**; real Registry write/rollback behavior remains behind the opaque internal capability and fake-host proof boundary.
- Phase 11 public CLI/GUI tweak mutation proof: **not claimed**; Phase 12 supplies the reviewed MCP/RPC authority contract instead of opening a CLI mutation bypass.
- Phase 12 exact implementation-head run `31899810049`: **PASS on Ubuntu and Windows** across Phase 1–12 static gates, lock integrity, rustfmt, locked workspace build, Clippy with warnings denied, full workspace units/adversarial regressions, Windows live read-only state proof, Runtime System X-Ray, and every applicable inherited fixture.
- Phase 12 canonical documentation-state run `31900134525`: **PASS on Ubuntu and Windows** across the complete Phase 1–12 pipeline on the one-file `docs/IMPLEMENTATION_STATUS.md` recovery branch.
- Phase 12 independent authority review: **5 bounded findings closed** — trusted-context deserialization, replay/session identity, action-vector cardinality, duplicated pending mission state, and Windows-only fake-host import scope.
- Phase 12 PR #22 external CodeRabbit disposition at merge: **review explicitly triggered; no review submission or inline review thread had been published, so no external CodeRabbit PASS is claimed**.
- Phase 12 final implementation PR surface: **9 intended files; no temporary proof/helper workflow, patch script, or diagnostic artifact**.
- Phase 12 merged through PR #22 as `e762369e1f71a67ef51ce216af792fdd00e74ad5`.
- Phase 12 live Registry mutation proof: **not claimed**; automated proof remains fake-host/adversarial for mutation and Windows live proof remains read-only.
- Phase 12 broader mutation proof: **not claimed**; arbitrary Registry, runtime/driver MCP authority, services/AppX/features/BCD/security mutation, public CLI mutation, and independent GUI mutation remain separately gated.

Phases 1–12 are closed at their recorded repository boundaries. Phase 12 is the canonical MCP/RPC-first external authority boundary for exactly the three Phase 11 low-risk reversible HKCU DWORD tweaks. Live Registry mutation proof, broader tweak/debloat domains, runtime/driver MCP authority, public CLI mutation, independent GUI mutation, Phase 7 network acquisition, archive execution, Windows-feature mutation, Winget execution, and public vault write surfaces remain independently blocked until their own authority, verification, cleanup, recovery, and proof contracts are frozen and proven.

## Phase 13 frozen implementation

Phase 13 begins Neo's Debloat product domain as a deliberately read-only AppX assessment layer. `RemovalCandidate` is a review classification only; Phase 13 creates no package-changing authority.

It adds:

- `neo-debloat` as a first-class workspace crate with no Windows, transaction, tweak-executor, runtime-executor, or driverstore dependency;
- typed Neo debloat IDs distinct from AppX package identities;
- typed classes `SafeOptional`, `FeatureDependent`, `DependencySensitive`, and `ProtectedManualOnly`;
- independent current-user installed and provisioned-image evidence states using `Present`, `Absent`, and `Unavailable`;
- Safe Cleanup, Gaming, Technician, Developer, and Custom preservation profiles, with Custom receiving no hidden defaults;
- declared Store, provisioned-image, vendor-source, or absent restore metadata without misrepresenting metadata as proven rollback;
- validated catalogue and evidence deserialization with case-insensitive package-identity uniqueness;
- default-selection law requiring SAFE OPTIONAL + LOW + CERTIFIED + removal-appropriate recommendation + declared restore route;
- explicit non-empty selection with duplicate/unknown ID rejection and complete selected-package installed/provisioned evidence requirements;
- candidate law requiring SAFE OPTIONAL + LOW + CERTIFIED + `Recommended`/`OptionalComponent` + restore route + no profile block;
- hard `BlockedProtected` and `BlockedPolicy` states for protected/manual-only and rejected/unsupported/conflicting/do-not-touch policy;
- mandatory non-empty consequence/dependency notes for every feature-dependent, dependency-sensitive, or protected/manual-only definition;
- synthetic `Contoso.*` catalogue/evidence fixtures so donor metadata cannot become unsupported Microsoft/OEM safety knowledge;
- internal `neo-debloat-assess` engineering proof binary whose text surface states `Machine changes: none`;
- behavioral proof that snapshots the fixture directory before assessment and proves it byte-for-byte unchanged afterward;
- Phase 13 20-lane static review integrated into normal Ubuntu/Windows CI beside every inherited Phase 1–12 gate.

WinUtil `config/appx.json` is accepted only as a donor for catalogue breadth/metadata shape. Its package list is not Neo evidence that a package is safe, low-risk, dependency-free, restorable, or suitable for any profile.

## Phase 13 review findings closed before freeze

Engineering review found and corrected two authority/policy defects before Phase 13 was frozen:

1. Manual selection originally could bypass the strict default-selection policy and allow an otherwise SAFE OPTIONAL item to become `RemovalCandidate` despite higher risk, non-certified evidence, or unsafe/unknown recommendation state. Candidate classification now independently requires LOW + CERTIFIED + explicitly removal-appropriate recommendation + restore route; rejected/do-not-touch/conflicting/unsupported states are blocked, and protected/manual-only classification takes precedence over profile-preservation messaging.
2. Feature-dependent, dependency-sensitive, and protected/manual-only definitions originally could exist with an empty side-effect/dependency list. Every non-safe class now requires at least one non-empty consequence/dependency note, with dedicated regressions and permanent Phase 13 static-gate coverage.

## Phase 13 deliberate non-claims

Phase 13 does **not** claim:

- real Windows AppX inventory probing;
- current-user, all-users, or provisioned-package removal;
- AppX restore/re-registration or Store availability;
- transaction-bound AppX rollback;
- PowerShell/AppX cmdlet execution correctness;
- real Microsoft/OEM package safety certification;
- public GUI/CLI debloat mutation;
- MCP/RPC debloat capability issuance or mutation methods.

Those remain separately gated. The detailed contract and proof boundary are frozen in `docs/decisions/0013-PHASE13-DEBLOAT-ASSESSMENT.md` and `docs/PHASE13_20_LANE_REVIEW.md`.

## Phase 14 frozen implementation

Phase 14 extends the proven Phase 13 debloat assessment model with real Windows evidence capture while remaining read-only. It adds `neo-debloat-probe`, which uses the existing `neo-probe::CommandRunner` evidence boundary and exactly two source-owned fixed PowerShell inventory scripts: current-user `Get-AppxPackage` and online `Get-AppxProvisionedPackage -Online`.

Catalogue package identities never enter command text. Neo enumerates the approved inventory surfaces first, retains raw `CommandEvidence`, then performs case-insensitive matching in Rust. Command start failure, non-zero result, malformed JSON, or an identity-incomplete inventory record makes that entire evidence side `Unavailable`; none can manufacture a false `Absent` result. Current-user package version evidence is retained only when exactly one distinct non-empty version is recovered. The normalized observations are rebuilt through the frozen Phase 13 `DebloatEvidence::new` constructor.

The Windows live proof requires both fixed inventory commands to actually succeed and proves the fixture tree remains byte-for-byte unchanged. The proof binary reports `Machine changes: none`. Phase 14 adds no package removal, provisioned-package mutation, restore/re-registration, transaction binding, public debloat mutation CLI/GUI, plugin dependency, MCP/RPC debloat authority, or capability issuance.

## Phase 14 review findings closed before freeze

1. Identity-incomplete AppX inventory records were initially ignored, which could manufacture false absence. Any missing/empty package identity now makes that entire inventory side `Unavailable`.
2. The live-proof static lane initially depended on rustfmt line layout. The gate now binds stable behavioral identities/assertion markers instead of formatting.
3. `SystemCommandRunner` was imported on non-Windows builds even though live construction is Windows-only. The import is target-gated rather than warning-suppressed.
4. The initial no-interpolation static lane was weaker than the source contract. It was strengthened to bind the two source-owned static script call sites and the no-catalogue-argument regression.
5. Review text initially described JSON output as bounded without a hard stdout-size bound in code. The canonical claim was corrected to typed, fail-closed JSON parsing; no hard output-size bound is claimed.
6. CodeRabbit later proved the no-interpolation regression was still vacuous because fake returned `CommandEvidence.args` stayed empty. The fake runner now records the actual `program` and `args`, and the regression asserts the exact two PowerShell argument vectors/scripts before checking that catalogue identity never appears in command arguments.
7. CodeRabbit also proved the Phase 14 static gate could pass without binding exact executable proof behavior. The gate now requires the command-count/success/no-change/before-after expressions, exact CI Phase 14 commands with the Windows condition, and the explicit negative plugin-dependency policy. Failed lanes also emit self-identifying GitHub annotations.

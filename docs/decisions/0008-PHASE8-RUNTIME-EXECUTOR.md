# Decision 0008 — Phase 8 Runtime Executor

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Scope:** internal execution of exact, already-present single-file runtime payloads from the Phase 7 managed vault  
**Public mutation authority:** none in this phase

## Decision

Phase 8 is the bounded execution child of the frozen Phase 6 Runtime/Gaming assessment and the Phase 7 managed package vault.

It adds a separate `neo-runtime-executor` mutation boundary. `neo-runtime` remains the pure read-only assessment model and `neo-runtime-probe` remains the read-only Windows evidence adapter.

Phase 8 may execute only a runtime action that is re-derived from a Phase 6 `Certified` recommendation, resolves to one exact `PackageKind::Runtime` catalogue entry, has an explicit Phase 8 execution contract, and resolves to the exact SHA-256-addressed runtime payload beneath the Builder/portable `NeoData` vault.

## Supported execution boundary

Phase 8 supports only direct single-file payloads whose entire promoted vault object is the installer itself:

- EXE runtime installer;
- MSI runtime installer through a trusted Windows `msiexec.exe` path.

Phase 8 does **not** extract or execute archive contents. A self-extracting redistributable is only eligible when the promoted object itself is the executable that owns the documented unattended install semantics; Neo does not unpack an archive in order to discover an executable.

Still blocked:

- network acquisition/download;
- Winget/Chocolatey execution;
- ZIP/CAB/7z/archive extraction;
- arbitrary command/script execution;
- PowerShell/cmd shell execution;
- `.NET 3.5`, DirectPlay, or other Windows Feature mutation;
- public runtime apply/repair commands;
- security/BCD changes;
- generic uninstall/rollback claims.

## Catalogue execution contract

`neo-catalogue` gains an optional runtime-only execution contract.

The contract declares:

- installer kind: EXE or MSI;
- exact install arguments;
- optional exact repair arguments;
- successful process exit codes;
- reboot-signalling exit codes, which must be a subset of successful codes;
- verification rule: installed-state predicate or an exact detected version;
- an explicit unattended-execution assertion.

The field is optional so existing read-only runtime packages remain valid. Missing execution metadata means **no Phase 8 execution authority**.

Execution metadata on a non-runtime package is invalid.

Phase 8 refuses an execution contract that is not explicitly unattended. MSI execution additionally constrains custom arguments to property assignments; Neo owns `/i`, `/qn`, and `/norestart` itself so a catalogue entry cannot repurpose `msiexec` into an unrelated operation.

## Exact package authority

The execution plan stores the Builder/portable application root, vault mode, package ID, package version, and package SHA-256. The runtime payload path is derived from `VaultLayout::runtime_pack_destination`; it is not accepted as an arbitrary caller-controlled executable path.

Before execution Neo:

1. validates the catalogue and Phase 6 runtime policy;
2. re-runs the Phase 6 assessment for the requested profile/component;
3. requires one `Certified` runtime install/repair action;
4. requires empty dependency/conflict edges until dependency closure is implemented;
5. rejects any runtime package that asks for boot/security-state changes;
6. resolves the exact vault path from package identity/version/hash;
7. verifies the promoted vault object against the catalogue SHA-256;
8. re-probes Windows build, architecture, and component baseline immediately before mutation;
9. rejects host/baseline drift before opening the apply stage.

Direct Serde construction of a plan re-runs structural validation and cannot bypass the derived-path or action/state contract.

## Staging and process boundary

The verified runtime object is copied into one marker-owned unique Phase 7 staging session using the installer extension implied by the typed execution kind. The staged bytes are SHA-256 verified again before process authority is granted.

The Windows backend:

- launches the exact staged EXE directly with `CreateProcess` semantics through Rust `Command`, never through a shell;
- resolves MSI execution through trusted Windows System32 rather than PATH/environment state;
- keeps one fixed `Local\` named mutex while an installer is running, serializing concurrent Neo runtime-executor processes **within the same Windows session**;
- does not claim system-wide cross-session serialization; Microsoft defines the `Local\` kernel-object namespace as session-local, while `Global\` is the cross-session namespace;
- opens the staged payload read-only with write/delete sharing denied, re-hashes that locked handle immediately before process launch, and keeps it open until the child exits;
- rejects link/reparse payload state before launch.

No installer result is inferred from stdout text.

## Process result and mutation evidence

Exit-code success is necessary but not sufficient.

A process that successfully started is conservatively recorded as `machine_changed=true` because installer internals may mutate state even when the final runtime predicate is unchanged or the process exits with failure.

A process that could not start is recorded as no observed machine mutation.

A successful configured exit code records apply success. A configured reboot exit code additionally creates the inherited reboot checkpoint path. Unknown/non-success exit codes record apply failure.

Because Phase 8 has no proven generic runtime restoration path, a started failed installer cannot be described as safely rolled back.

## Verification

After successful apply without a required reboot, Neo re-runs the read-only runtime probe and verifies the exact component predicate before the transaction may complete.

After a required reboot, the persisted Phase 4 checkpoint is resumed only after the same runtime predicate is re-proven.

Supported verification rules:

- `installed_state` — the frozen component-specific detector must report `Installed`;
- `exact_detected_version` — the detector must report `Installed` and the exact configured detected-version string.

Host build/architecture drift is an unavailable verification result, not a PASS.

A temporary probe failure returns an operational error without fabricating a verification result, leaving the transaction at a retryable verification/reboot stage.

Installer exit code 0 alone never completes a runtime mission.

## Repair semantics

`RuntimeRepair` authority exists only when the Phase 6 baseline is `Broken` or `Partial` and the exact catalogue execution contract contains repair arguments.

A missing runtime uses `RuntimeInstall` and install arguments.

Installed or Unknown state never becomes Phase 8 mutation authority.

## Rollback and recovery

Phase 8 makes **no generic runtime rollback claim**.

Every Phase 8 transaction action is therefore encoded as transaction-irreversible and requires the existing explicit irreversible acknowledgement before authorization.

Neo still captures the exact normalized baseline state/version for history and drift detection.

If verification fails after a successful/started installer, the transaction enters `Failed` rather than pretending rollback occurred. A later package-specific uninstall/restoration executor may add reversible authority only after its own capture/apply/verify/restore contract is frozen and proven.

## CLI boundary

Phase 8 may add read-only plan/spec validation so the CLI and future GUI can inspect the exact contract.

Phase 8 does **not** add a CLI command that executes, installs, repairs, downloads, enables a Windows feature, advances an authorized transaction, or reboots the machine.

The internal Rust backend is callable only by code that possesses the prepared plan/session and valid transaction authorization.

## Proof boundary

Phase 8 proof must include:

- inherited Phase 1–7 static gates;
- Phase 8 20-lane deterministic review;
- Cargo lock integrity;
- rustfmt;
- locked workspace build on Ubuntu and Windows;
- Clippy with warnings denied on both OSes;
- complete workspace tests;
- adversarial plan/Serde/path/hash/exit-code/reboot/verification tests;
- fake-host execution tests covering install, repair, failure, reboot, drift, and retryable probe failure;
- Windows compilation of the real no-shell EXE/MSI backend;
- existing Windows live read-only Runtime System X-Ray;
- all inherited CLI fixtures plus a read-only Phase 8 plan fixture;
- external-review disposition with every correctness/security finding reconciled before merge.

CI does **not** execute a real runtime installer and therefore does not claim live Windows mutation proof. The backend may be merged internally after compiler/adversarial proof while the public write surface remains closed.

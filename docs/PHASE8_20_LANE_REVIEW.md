# Neo Driver — Phase 8 Runtime Executor 20-Lane Review

**Scope:** internal, transaction-bound execution of exact local/offline single-file runtime payloads from the Phase 7 vault.

**Predecessor preservation:** Phase 6 remains read-only Runtime/Gaming assessment + System X-Ray. Phase 7 remains the managed vault. Phase 8 adds a separate execution boundary and opens no public mutation CLI.

## Phase 8 lanes

1. `neo-runtime-executor` is a separate first-class mutation crate; `neo-runtime` remains read-only.
2. Only one exact `PackageKind::Runtime` catalogue package may become execution authority.
3. Execution authority is re-derived from a Phase 6 `Certified` recommendation, not caller free-form action data.
4. Missing/Unknown/Installed state cannot be mis-bound to the wrong install/repair operation.
5. Runtime repair requires explicit package repair arguments and Broken/Partial baseline evidence.
6. Runtime package dependencies/conflicts remain a hard block until dependency closure is proven.
7. Runtime packages requiring boot/security-state changes remain blocked in Phase 8.
8. Builder/portable application root + package ID/version/SHA derive the vault payload path; arbitrary executable paths are not accepted.
9. The promoted vault payload SHA-256 is verified before apply authority.
10. Absolute application-root, host build, architecture and component baseline are re-probed/validated before mutation; drift blocks apply.
11. The promoted payload is copied into marker-owned unique staging and staged bytes are re-hashed before launch.
12. EXE execution is direct/no-shell; MSI uses trusted System32 `msiexec.exe` and fixed install/quiet/no-restart switches.
13. MSI catalogue arguments cannot replace Neo's install operation with arbitrary msiexec switches or a bare empty property assignment.
14. The Windows backend serializes concurrent Neo runtime-executor processes **within one Windows session** through one fixed, bounded-wait `Local\` named mutex and rejects link/reparse payload state; it does not claim system-wide cross-session serialization or create a path-based lock outside the retained vault capability model.
15. The Windows backend re-hashes a write/delete-locked staged file immediately before launch and retains the handle through process exit.
16. Exit-code success and reboot semantics preserve Windows 32-bit status bit patterns; reboot codes remain a subset of successful codes.
17. A started installer is conservatively considered potentially machine-changing even when it exits with failure.
18. Completion requires re-probe verification; exit code alone cannot complete a transaction, and transient probe failure remains retryable.
19. Required reboot uses the inherited persistent checkpoint/resume contract; host/build drift cannot become post-reboot PASS.
20. No generic runtime rollback or public raw-host authority is claimed: external mutation transitions require an opaque `RuntimeExecutorCapability` with no public constructor, raw host/invocation/process/Windows-host adapters remain crate-private, irreversible acknowledgement is mandatory internally, and CI does not claim live runtime mutation proof.

## Engineering and review findings closed before freeze

- The original catalogue extension preserved JSON compatibility but Rust compiler proof identified exactly three literal/binding follow-ups: `runtime_execution: None` in existing test constructors plus the `Win32_Security` feature required by the locked Windows `CreateMutexW` binding. Those compiler-proven corrections were applied without widening runtime authority.
- The Phase 7 vault was extended with one narrow `stage_managed_file` primitive that reuses retained no-follow capabilities, exact staging ownership markers, and SHA-256 verification instead of introducing path-based copy authority.
- The initial Phase 8 static review exposed two proof-harness phrase dependencies after the implementation evidence was already present. Those lanes were corrected to bind to production path derivation/managed-root enforcement, adversarial tests, CLI source absence of execution authority, and irreversible-acknowledgement evidence rather than documentation wording.
- Cargo lock helper run `31692195361` generated the complete CLI + runtime-executor graph and passed Phase 6, Phase 7, and Phase 8 static reviews before committing the lock and self-deleting its temporary workflow.
- Microsoft documents `Local\` named kernel objects as session-local and `Global\` as cross-session. Phase 8 therefore freezes only same-session cross-process serialization; no system-wide serialization claim is made.
- Independent PR-surface review found that the first crate surface publicly exposed `RuntimeHost`, `RuntimeInvocation`, `RuntimeProcessResult`, `RuntimeExecutionSession`, and `WindowsRuntimeHost`, allowing a safe Rust library caller to bypass the certified session path. A pure crate-private correction then exposed legitimate dead-code under `-D warnings`. The final architecture keeps validated plan/session inspection public while every public mutation transition requires opaque `RuntimeExecutorCapability`; that token has no public constructor/field, and raw host/invocation/process/Windows-host adapters remain crate-private. Phase 8 lane 20 fails if that boundary reopens.
- CodeRabbit identified and Neo corrected bare empty MSI `PROPERTY=` acceptance while preserving case-insensitive valid property names and explicit `PROPERTY=""` authority.
- CodeRabbit identified Windows high-bit process status handling. Catalogue validation now preserves raw 32-bit Windows exit-code bit patterns represented through Rust `i32` rather than rejecting negative representations.
- CodeRabbit identified staging leaks before process launch. Invocation-building and checkpoint-transition failures now clean marker-owned staging before returning.
- CodeRabbit identified relative `application_root` authority. Direct/Serde validation now rejects non-absolute application roots before managed-path derivation.
- CodeRabbit identified duplicate package evidence keys. Phase 8 now requires exactly one item for each authority key and then validates its value.
- CodeRabbit identified the missing-observation path using the wrong diagnostic. `MissingObservation` now distinguishes absent runtime evidence from absent certified action authority.
- CodeRabbit identified an unbounded Windows mutex wait. The `Local\` named mutex now uses a finite 300,000 ms wait and returns typed host failure on `WAIT_TIMEOUT`.
- A CodeRabbit `windows`-module-shadowing suggestion became outdated and was explicitly dispositioned after repeated exact Windows compilation proved the locked `windows 0.62.2` imports compile in this crate layout.
- The PathBuf formatting thread became outdated after the corrected error formatting and exact compiler proof.

## Review correction pre-proof

Windows one-shot correction run `31697713764` passed:

- Phase 8 20/20 static review;
- full locked workspace type/build proof;
- Clippy with warnings denied;
- `neo-runtime-executor` regression tests;
- `neo-catalogue` regression tests;
- diff validation;
- self-clean of the temporary correction workflow/tool.

All PR #12 review threads are resolved or explicitly dispositioned as outdated. This run is a bounded correction pre-proof only; the corrected helper-free head still requires the complete normal Ubuntu + Windows pipeline before merge.

## Merge proof requirements

Phase 8 must pass:

- Phase 1–8 static gates;
- lock integrity;
- rustfmt;
- locked workspace build on Ubuntu and Windows;
- Clippy with warnings denied;
- all workspace unit/integration tests;
- Windows compilation of the real runtime backend;
- live read-only Windows Runtime System X-Ray inherited from Phase 6;
- all existing CLI fixtures;
- a read-only Phase 8 runtime-execution-plan fixture;
- zero unresolved correctness/security review findings;
- clean PR surface with no temporary helper workflow at merge.

No test or green CI run is allowed to imply that a real runtime installer was executed on CI.

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
10. Host build/architecture/component baseline is re-probed at preflight and drift blocks mutation.
11. The promoted payload is copied into marker-owned unique staging and staged bytes are re-hashed before launch.
12. EXE execution is direct/no-shell; MSI uses trusted System32 `msiexec.exe` and fixed install/quiet/no-restart switches.
13. MSI catalogue arguments cannot replace Neo's install operation with arbitrary msiexec switches.
14. The Windows backend serializes concurrent Neo runtime-executor processes **within one Windows session** through one fixed `Local\` named mutex and rejects link/reparse payload state; it does not claim system-wide cross-session serialization or create a path-based lock outside the retained vault capability model.
15. The Windows backend re-hashes a write/delete-locked staged file immediately before launch and retains the handle through process exit.
16. Exit-code success and reboot semantics are typed; reboot codes are a subset of successful codes.
17. A started installer is conservatively considered potentially machine-changing even when it exits with failure.
18. Completion requires re-probe verification; exit code alone cannot complete a transaction, and transient probe failure remains retryable.
19. Required reboot uses the inherited persistent checkpoint/resume contract; host/build drift cannot become post-reboot PASS.
20. No generic runtime rollback or public apply CLI is claimed; irreversible acknowledgement is mandatory and CI does not claim live runtime mutation proof.

## Pre-review proof recovered during implementation

- The original catalogue extension preserved JSON compatibility but Rust compiler proof identified exactly three literal/binding follow-ups: `runtime_execution: None` in existing test constructors plus the `Win32_Security` feature required by the locked Windows `CreateMutexW` binding. Those compiler-proven corrections were applied without widening runtime authority.
- The Phase 7 vault was extended with one narrow `stage_managed_file` primitive that reuses retained no-follow capabilities, exact staging ownership markers, and SHA-256 verification instead of introducing path-based copy authority.
- The initial Phase 8 static review exposed two proof-harness phrase dependencies after the implementation evidence was already present. Those lanes were corrected to bind to production path derivation/managed-root enforcement, adversarial tests, CLI source absence of execution authority, and irreversible-acknowledgement evidence rather than documentation wording.
- Cargo lock helper run `31692195361` generated the complete CLI + runtime-executor graph and passed Phase 6, Phase 7, and Phase 8 static reviews before committing the lock and self-deleting its temporary workflow.
- Microsoft documents `Local\` named kernel objects as session-local and `Global\` as cross-session. Phase 8 therefore freezes only same-session cross-process serialization; no system-wide serialization claim is made.

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

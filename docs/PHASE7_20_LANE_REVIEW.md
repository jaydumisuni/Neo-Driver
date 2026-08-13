# Neo Driver — Phase 7 Managed Package Vault 20-Lane Review

**Scope:** clean managed package/offline-pack storage beneath the application root supplied by THETECHGUY Software Builder or the portable Neo folder.

**Predecessor preservation:** merged Phase 6 remains the runtime/gaming assessment + read-only runtime System X-Ray. Phase 7 adds the vault without replacing that work.

**Machine mutation boundary:** Phase 7 does not install drivers, execute runtimes, download packages, alter Windows security state, or expose a public machine-changing vault command.

## Phase 7 lanes

1. `neo-vault` is a first-class workspace crate.
2. Phase 6 runtime/gaming crates and review gate remain present.
3. Vault production code does not choose ProgramData/Program Files; Builder supplies the application root.
4. A single `NeoData` child is the managed root.
5. Application roots must be resolved absolute paths.
6. Installed and portable modes share the same child-layout contract.
7. Catalogue, driver-packs, packages, runtimes, staging, sessions, backups, logs, and cache are explicit Neo-owned directories.
8. Package/session/version path segments reject traversal/separators and validate during Serde.
9. SHA-256 is a validated typed identity.
10. Source-map root validation cannot be bypassed through direct Serde.
11. Initial source map is restricted to the four approved TTG driver repositories and pinned hashes.
12. Vault production code contains no network acquisition implementation.
13. Pack intake proves source, staged and promoted bytes by SHA-256.
14. Concurrent imports use unique staging plus exclusive final creation and cannot overwrite promoted content.
15. Staging cleanup requires an exact Neo ownership marker.
16. Destructive cleanup stays inside Neo-managed disposable space.
17. Directory traversal/promotion uses retained no-follow filesystem capabilities.
18. Audit validates the application root and rejects symlink/reparse escapes.
19. Public vault CLI remains read-only: describe, validate-sources and audit.
20. Driver/security mutation authority remains outside the vault while Phase 6 runtime CLI remains intact.

## Rebase correction

PR #7 was built while `main` still ended at Phase 5. Before it could merge, PR #8 established the canonical runtime/gaming Phase 6 and PR #9 recorded that merge state. A direct merge of stale PR #7 would therefore conflict with and semantically overwrite canonical Phase 6 history.

Corrective action: create `implementation/phase-7-managed-vault` from current `main`, transplant the proven vault crate and source map, merge only shared integration surfaces, rename the proof/decision layer to Phase 7, and re-run the complete Phase 1–7 Ubuntu/Windows pipeline.

## Rebase proof findings

- Rebase run `31686937725` proved Phase 1–6 remained 20/20 on both Ubuntu and Windows. Phase 7 stopped only because lane 14 referenced an obsolete regression name.
- The actual concurrency regression is `concurrent_same_pack_import_never_overwrites_or_leaves_staging_noise`; the gate was corrected to bind to that existing eight-worker proof rather than adding duplicate coverage.
- Run `31687088235` then passed Phase 1–7 20/20 on Ubuntu and stopped only at the expected stale Cargo.lock gate.
- One-shot lock run `31687136770` generated the exact combined runtime + vault dependency graph with Cargo, re-proved Phase 6 and Phase 7, committed the generated lock, and deleted its temporary write-enabled workflow in the same commit. No dependency graph was hand-authored.

## Proof requirements before merge

Phase 7 must pass:

- Phase 1–7 static gates;
- lock integrity;
- rustfmt;
- locked workspace build;
- Clippy with warnings denied;
- all workspace unit/integration tests, including vault concurrency regressions;
- Windows live read-only runtime System X-Ray proof inherited from Phase 6;
- catalogue, matcher, runtime, gaming, vault source-map, and transaction CLI fixtures;
- zero unresolved review threads;
- clean PR surface with no temporary proof/helper workflow.

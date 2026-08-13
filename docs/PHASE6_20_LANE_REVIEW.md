# Neo Driver — Phase 6 Managed Package Vault 20-Lane Review

**Scope:** clean managed package/offline-pack storage beneath the application root supplied by THETECHGUY Software Builder or the portable Neo folder.

**Machine mutation boundary:** Phase 6 does not install drivers, execute runtimes, download packages, alter Windows security state, or expose a new public machine-changing command.

## Recovered Builder evidence

The Software Builder has two relevant path contracts:

- its reusable Windows installer shell defines a fallback base of `C:\Program Files (x86)\THETECHGUY Digital Solutions`;
- its current example project configuration overrides `installer.installBase` to `C:\Program Files\THETECHGUY DIGITAL SOLUTIONS`.

Therefore Neo must not choose either location itself. The installer/Builder supplies the application root. Neo owns only `NeoData` below that root. Portable mode uses the same child layout beneath the portable Neo folder.

## Phase 6 lanes

1. `neo-vault` is a first-class workspace crate.
2. Production vault code contains no ProgramData/Program Files root selection.
3. A single `NeoData` child is the managed root.
4. Application roots must be resolved absolute paths.
5. Installed and portable modes share the same child-layout contract.
6. Catalogue, driver-packs, packages, runtimes, staging, sessions, backups, logs, and cache are explicit Neo-owned directories.
7. Package/session/version path segments reject traversal/separators and validate during Serde.
8. SHA-256 is a validated typed identity.
9. Source-map root validation cannot be bypassed through direct Serde.
10. Initial source map is restricted to the four approved TTG driver repositories.
11. Phase 6 contains no network acquisition implementation.
12. Local pack intake hashes source bytes, staging bytes, and promoted bytes.
13. Final promotion uses capability-relative `create_new` semantics: identical content is idempotent, drift fails closed, and concurrent same-pack imports cannot overwrite each other.
14. Staging cleanup requires an exact Neo ownership marker and import staging is collision-proof.
15. Destructive cleanup is capability-relative to retained `NeoData/staging` handles rather than an ambient absolute path.
16. Application-root traversal, managed-directory creation, file open/create, audit, promotion, and cleanup use retained no-follow filesystem capabilities.
17. Builder/portable application root must already exist and be opened as an existing no-follow capability.
18. Public vault CLI remains read-only: describe, validate-sources, and audit only.
19. Aggregate TTG driver source assets are pinned by release tag and published SHA-256.
20. The vault has no dependency on the Phase 5 mutator and cannot invoke driver install/security mutation.

## Engineering and review findings corrected before freeze

### P6-F1 — ProgramData was incorrectly proposed as Neo's data root

Recovered Builder evidence disproved that convention. Corrected contract: Neo accepts the Builder/portable application root and owns only its `NeoData` child.

### P6-F2 — Builder fallback and project override are different

Hard-coding the reusable fallback would also be incorrect because Builder projects can override `installBase`. Corrected: location authority belongs to Builder/installer configuration, not the Neo engine.

### P6-F3 — Recursive directory creation could follow an intermediate junction/symlink

The initial draft used recursive directory creation for promoted pack parents. A malicious pre-created intermediate link could redirect writes outside `NeoData`. The first correction performed component-by-component checking. The later external security review correctly showed that path checking still leaves a check-to-use race; P6-F9 records the final stronger solution.

### P6-F4 — Cleanup must prove ownership, not merely location

Being under a path named `staging` is insufficient deletion authority. Corrected: each staging session has an exact Neo marker bound to its validated session ID; missing/mismatched markers block cleanup.

### P6-F5 — Source-map provenance cannot become install authority

Release repositories/assets are only acquisition provenance. Existing catalogue, hash, signature, compatibility, transaction, reboot, verification, and rollback gates remain authoritative for driver installation.

### P6-F6 — Cargo.lock had to include the new workspace crate exactly

The first authoritative PR run passed Phase 1–6 static reviews on both Ubuntu and Windows and stopped only at the lockfile guard. Cargo generated the updated graph for `neo-vault`; no dependency version was selected manually. A one-shot branch-only helper committed Cargo's exact lock and deleted itself.

### P6-F7 — Initial Phase 6 source required rustfmt normalization

After the exact lock was committed, the next authoritative Ubuntu gate stopped only at rustfmt. The formatter changes were layout-only across `neo-cli` and `neo-vault`. A one-shot branch-only formatter applied `cargo fmt --all`, re-ran Phase 6's 20-lane review and `cargo fmt --check`, committed only formatter output, and deleted itself.

### P6-F8 — Concurrent same-pack imports shared staging/promotion state

Internal review found that deterministic staging names could make concurrent Neo processes interfere with each other's `payload.pack` and cleanup. Corrected first with unique import sessions, then strengthened so final package creation is atomic `create_new` relative to the retained package-version directory capability. A multi-worker regression proves exactly one import promotes the pack; other workers may observe the identical promoted pack or fail busy, and all worker-owned staging is cleaned.

### P6-F9 — Path checks could still race against junction/symlink replacement

CodeRabbit raised a valid major security finding: checking path components and later reusing those path strings cannot prove a writable tree was not swapped for a symlink/reparse point between check and use. Corrected by replacing managed write authority with `cap-std`/`cap-fs-ext` retained directory capabilities. The application root is opened from the filesystem root and each descendant is opened with no-follow semantics. Managed directory creation, staging, cleanup, final destination creation, verification, and audit operate relative to retained directory handles. Final promotion does not use a bare path `rename`; it creates the destination with exclusive `create_new` relative to the retained version-directory capability, copies/syncs, then reopens and SHA-256 verifies the promoted bytes. A regression proves a symlinked package directory cannot redirect writes outside `NeoData`.

### P6-F10 — Capability filesystem dependencies required a new generated lock graph

The handle-based security correction introduced `cap-std` and `cap-fs-ext`. A one-shot capability proof regenerated Cargo.lock, applied rustfmt, passed the Phase 6 20-lane gate, full workspace compiler, Clippy with warnings denied, and all `neo-vault` tests before committing. The helper self-deleted in the same commit. The resulting lock pins `cap-std`/`cap-fs-ext` 4.0.2 and their transitive graph exactly.

## External review disposition

PR #7 produced three CodeRabbit threads. The unique-staging and invalid-root findings were corrected first. The remaining major TOCTOU finding was corrected by the capability-held no-follow rewrite described in P6-F9. CodeRabbit subsequently marked all three threads resolved/outdated; there are currently zero unresolved review threads on the corrected Phase 6 source.

## Source-map evidence frozen for the first offline pack

- `jaydumisuni/android-drivers` / `v1` / `android.drivers.zip`;
- `jaydumisuni/Exynos-driver` / `v1` / Exynos EUB + UsbDk assets;
- `jaydumisuni/Apple-windows-drivers` / `v1` / `Apple.windows.drivers.zip`;
- `jaydumisuni/TechGuyDrivers` / `v1.0.0` / `TechGuyDrivers.zip`.

Their published SHA-256 values are recorded in `config/driver-pack-sources.json`.

## Deliberately deferred

- network download/acquisition;
- archive extraction/parsing;
- automatic package refresh;
- runtime installation;
- public pack-import/cleanup CLI writes;
- live attached-device mutation proof from Phase 5.

Phase 6 must pass Phase 1–6 static gates, lock integrity, rustfmt, locked build, Clippy with warnings denied, all workspace tests, existing CLI fixtures, and the vault source-map fixture on Ubuntu and Windows before merge.

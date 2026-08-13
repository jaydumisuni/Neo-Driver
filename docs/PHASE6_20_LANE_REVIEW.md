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
12. Local pack intake hashes source bytes and copied staging bytes.
13. Promoted packs are idempotent when identical and fail closed on content drift.
14. Staging cleanup requires an exact Neo ownership marker.
15. Destructive cleanup is limited to owned staging/cache descendants.
16. Existing symlink/reparse paths are rejected and directory creation is checked component by component.
17. Builder/portable application root must already exist as a normal directory.
18. Public vault CLI remains read-only: describe, validate-sources, and audit only.
19. Aggregate TTG driver source assets are pinned by release tag and published SHA-256.
20. The vault has no dependency on the Phase 5 mutator and cannot invoke driver install/security mutation.

## Pre-proof engineering findings already corrected

### P6-F1 — ProgramData was incorrectly proposed as Neo's data root

Recovered Builder evidence disproved that convention. Corrected contract: Neo accepts the Builder/portable application root and owns only its `NeoData` child.

### P6-F2 — Builder fallback and project override are different

Hard-coding the reusable fallback would also be incorrect because Builder projects can override `installBase`. Corrected: location authority belongs to Builder/installer configuration, not the Neo engine.

### P6-F3 — Recursive directory creation could follow an intermediate junction/symlink

The initial draft used recursive directory creation for promoted pack parents. A malicious pre-created intermediate link could redirect writes outside `NeoData`. Corrected: directories are created component-by-component, checking every existing component for symlink/reparse state before descending.

### P6-F4 — Cleanup must prove ownership, not merely location

Being under a path named `staging` is insufficient deletion authority. Corrected: each staging session has an exact Neo marker bound to its validated session ID; missing/mismatched markers block cleanup.

### P6-F5 — Source-map provenance cannot become install authority

Release repositories/assets are only acquisition provenance. Existing catalogue, hash, signature, compatibility, transaction, reboot, verification, and rollback gates remain authoritative for driver installation.

### P6-F6 — Cargo.lock had to include the new workspace crate exactly

The first authoritative PR run passed Phase 1–6 static reviews on both Ubuntu and Windows and stopped only at the lockfile guard. Cargo generated the updated graph: `neo-cli` gains `neo-vault`, and `neo-vault` depends only on the already-locked `serde`, `serde_json`, `sha2`, and `thiserror`. No external crate version changed. The generated lock was committed verbatim by a one-shot branch-only helper which deleted itself in the same commit.

### P6-F7 — Initial Phase 6 source required rustfmt normalization

After the exact lock was committed, the next authoritative Ubuntu gate stopped only at rustfmt. The formatter changes were layout-only across `neo-cli` and `neo-vault`. A one-shot branch-only formatter applied `cargo fmt --all`, re-ran Phase 6's 20-lane review and `cargo fmt --check`, committed only formatter output, and deleted itself in the same commit.

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

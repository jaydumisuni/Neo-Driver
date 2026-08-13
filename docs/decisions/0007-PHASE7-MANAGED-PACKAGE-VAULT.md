# Decision 0007 — Phase 7 Managed Package Vault

**Status:** Accepted for Phase 7 implementation

## Context

The managed package-vault work was originally developed on PR #7 while `main` still ended at the Phase 5 boundary. During that work, a separate runtime/gaming System X-Ray implementation became the canonical Phase 6 through PR #8, with its merged status recorded through PR #9.

Therefore the vault work is rebased as **Phase 7**. Phase 6 runtime/gaming history and contracts remain intact.

## Decision

Neo Driver inherits the application root supplied by THETECHGUY Software Builder or by the portable Neo folder. The vault engine does not hard-code `ProgramData`, `Program Files`, or `Program Files (x86)` as storage authority.

Neo owns exactly one managed child beneath that application root: `NeoData`.

Installed example:

```text
<Builder-selected Neo application root>\
├── NeoDriver.exe
└── NeoData\
```

Portable/offline mode uses the same `NeoData` layout relative to the portable Neo folder.

## Managed layout

`NeoData` contains only Neo-owned data:

- `catalogue/` — package/catalogue metadata;
- `driver-packs/` — retained approved driver archives/offline packs;
- `packages/` — promoted verified package payloads;
- `runtimes/` — retained approved runtime packages;
- `staging/` — transaction/import-scoped temporary material;
- `sessions/` — restart-safe Neo session/checkpoint data;
- `backups/` — Neo-owned recovery artifacts;
- `logs/` — Neo audit/log output;
- `cache/` — disposable Neo cache.

## Safety laws

1. Every managed path remains under `NeoData`.
2. Package IDs, versions and session IDs reject traversal, separators, alternate roots, trailing-dot/space ambiguity and empty components.
3. SHA-256 identity is typed and normalized.
4. Existing symlink/reparse paths are rejected.
5. Directory traversal and final promotion use retained no-follow filesystem capabilities rather than check-then-use path validation.
6. Temporary staging is uniquely owned per import and carries an exact Neo ownership marker.
7. Cleanup may remove only Neo-owned disposable material; retained packs, promoted content, backups and logs are not broad cleanup targets.
8. Pack intake hashes source bytes, copied staging bytes and promoted bytes.
9. Final promotion uses exclusive creation; concurrent imports cannot overwrite a promoted pack.
10. Identical promoted content is idempotent; content drift fails closed.
11. Offline and installed modes share one package identity/layout model.
12. Network acquisition is not enabled by this phase.
13. Repository/release provenance is not install authority. Existing catalogue, signature, compatibility, transaction, verification and rollback gates remain authoritative.
14. Phase 7 adds no driver binding, runtime execution, BCD/security mutation or public cleanup/import command.

## Approved source families

The initial source map records these TTG repositories as acquisition provenance:

- `jaydumisuni/android-drivers`;
- `jaydumisuni/Exynos-driver`;
- `jaydumisuni/Apple-windows-drivers`;
- `jaydumisuni/TechGuyDrivers`.

Pinned release assets and published SHA-256 values live in `config/driver-pack-sources.json`.

## Recovered engineering findings from the pre-rebase proof

The vault implementation was already challenged on the stale PR #7 branch before this Phase 7 rebase. Valid findings closed there are retained as design evidence:

1. Builder root authority replaced an incorrect ProgramData proposal.
2. Builder fallback and per-project install-root override were kept distinct.
3. Recursive path creation was rejected in favor of safe component/capability handling.
4. Cleanup authority was bound to exact Neo ownership markers.
5. Source repositories were kept as provenance, never install authority.
6. Cargo.lock was regenerated rather than hand-edited.
7. Rustfmt drift was corrected before compiler proof.
8. Concurrent imports received unique staging identities and exclusive promotion semantics.
9. Application-root audit now fails closed for invalid/link-like roots.
10. Path-based TOCTOU protection was upgraded to retained no-follow directory capabilities.
11. Windows handle lifetimes were shortened before cleanup after Win32 sharing error 32 exposed retained-handle deletion conflicts.

The final stale-branch proof head `e1a3013927b4a622816e2d3b923670d3a5f51d56` passed the complete Ubuntu and Windows pipeline before merge was attempted. The merge was deliberately rejected after discovering canonical `main` had advanced with a different Phase 6; this Phase 7 rebase is the corrective integration path.

## Deliberately not enabled

- automatic online downloading;
- archive extraction/execution;
- public pack import/cleanup mutation commands;
- runtime installation;
- forced driver binding or broad Driver Store cleanup;
- BCD/security mutation;
- any weakening of merged Phase 1–6 contracts.

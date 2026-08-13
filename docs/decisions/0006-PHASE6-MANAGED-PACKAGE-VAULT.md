# Decision 0006 — Phase 6 Managed Package Vault

**Status:** Accepted for Phase 6 implementation

## Decision

Neo Driver inherits the Windows application root supplied by THETECHGUY Software Builder. The Neo engine does not hard-code `ProgramData`, `Program Files`, or `Program Files (x86)` as its own authority. Builder/installer configuration owns that choice.

Neo owns exactly one managed child beneath that application root: `NeoData`.

Installed example when Builder uses its current project template:

```text
C:\Program Files\THETECHGUY DIGITAL SOLUTIONS\Neo Driver\
├── NeoDriver.exe
└── NeoData\
```

Installed example when Builder uses its reusable installer fallback:

```text
C:\Program Files (x86)\THETECHGUY Digital Solutions\Neo Driver\
├── NeoDriver.exe
└── NeoData\
```

Portable/offline mode uses the same layout relative to the portable Neo folder rather than inventing a second storage model.

## Managed layout

`NeoData` contains only Neo-owned data:

- `catalogue/` — package/catalogue metadata;
- `driver-packs/` — retained approved driver archives/offline packs;
- `packages/` — promoted verified package payloads;
- `runtimes/` — retained approved runtime packages;
- `staging/` — transaction-scoped temporary extraction/work areas;
- `sessions/` — restart-safe Neo session/checkpoint data;
- `backups/` — Neo-owned recovery artifacts;
- `logs/` — Neo audit/log output;
- `cache/` — disposable Neo cache.

## Safety laws

1. Every managed path must remain lexically and canonically inside `NeoData`.
2. Package IDs, versions and relative paths may not contain traversal, absolute paths, alternate-root prefixes, or empty path components.
3. Neo does not follow symlinks/reparse-style links while importing an offline package tree.
4. Windows path identity is compared case-insensitively where collision matters.
5. Temporary staging directories are transaction/session scoped and contain an ownership marker before cleanup is permitted.
6. Cleanup may delete only marker-owned paths under `NeoData/staging` or `NeoData/cache`.
7. Promoted package content is immutable by contract and identified by approved SHA-256 metadata.
8. Failed or cancelled work removes only Neo-owned temporary material; retained packs, promoted packages, backups and audit logs remain untouched.
9. Offline and installed modes use the same catalogue/package identity model.
10. Network acquisition is not enabled by this phase. Phase 6 establishes the clean storage/import boundary first.

## Initial TTG driver-source families

The offline/source map records these existing TTG repositories as package origins:

- `jaydumisuni/android-drivers` — Android/ADB/OEM USB driver collection;
- `jaydumisuni/Exynos-driver` — Samsung Exynos/EUB and UsbDk source family;
- `jaydumisuni/Apple-windows-drivers` — Apple Windows/Recovery/DFU driver source family;
- `jaydumisuni/TechGuyDrivers` — broader TechGuy driver collection.

Source metadata is provenance only. A repository/release is never trusted as install authority by itself; the existing Neo catalogue, hash, signature, compatibility, transaction and rollback gates still apply.

## Deliberately not enabled

- arbitrary filesystem cleanup;
- writes outside the supplied application root/portable root;
- automatic online downloading;
- silent archive execution;
- public driver-mutation CLI changes;
- weakening any Phase 1–5 safety contract.

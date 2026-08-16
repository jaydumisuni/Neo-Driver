# Decision 0019 — Trusted Persistent Debloat History Store and Receipt Selection

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Product domain:** seventh bounded Debloat child  
**Authority:** append-only NeoData history persistence + trusted store-owned receipt selection; no AppX mutation capability issuance

## Decision

Phase 19 closes the persistence/provenance gap deliberately left open by Phases 17 and 18.

Phase 17 creates a versioned, fingerprinted `DebloatRemovalReceipt`, but explicitly states that the receipt fingerprint is tamper detection rather than a signature or caller-authentication mechanism. Phase 18 accepts only a constructor-produced prepared inverse transaction and deliberately does not issue its opaque restore capability through CLI, GUI, plugin, MCP, or RPC.

Phase 19 therefore adds a Neo-owned persistent Debloat history store that can:

1. record a receipt only by deriving it directly from a completed Phase 16 execution session;
2. persist that receipt beneath Neo's existing Builder/portable-owned `NeoData` root;
3. load/list only records discovered through the store's own no-follow filesystem capabilities;
4. select a receipt by a typed store record id rather than a caller-supplied filesystem path; and
5. prepare Phase 17 restore readiness only after reloading and revalidating that store-owned record.

Phase 19 does **not** issue `DebloatRestoreExecutorCapability` and does not execute AppX registration/removal.

## Storage authority

Decision 0007 remains the application-root authority. Phase 19 does not choose `ProgramData`, `Program Files`, a profile directory, or a second managed root. `VaultLayout` owns the canonical `NeoData/history` child.

The bounded extension is:

```text
<Builder/portable application root>/
└── NeoData/
    └── history/
        └── debloat-removals/
            ├── .staging/
            │   └── <unique-record-session>/
            │       ├── .neo-history-staging.json
            │       └── record/
            │           └── receipt.json
            └── <receipt-fingerprint>/
                └── receipt.json
```

`NeoData/history` is retained Neo-owned data. It is not cache and is never broad-cleanup authority. Installed and portable modes use the same child layout relative to the supplied application root.

## Record identity

The store record id is the validated lowercase 64-hex Phase 17 receipt fingerprint. It is used as one filesystem segment only after typed validation.

The persisted envelope is schema-versioned and binds:

- store schema version;
- record id;
- the complete Phase 17 `DebloatRemovalReceipt`.

On every load, the store requires:

- supported store schema;
- directory record id == envelope record id;
- envelope record id == receipt fingerprint;
- full Phase 17 receipt deserialization/validation success;
- a bounded record size.

A raw path is never accepted as record-selection authority.

## Write provenance

There is intentionally no public `save_receipt(path, json)` or `save_receipt(DebloatRemovalReceipt)` surface.

The write entry point accepts a completed Phase 16 `DebloatExecutionSession` and calls the already-proven Phase 17 `receipt_from_completed_execution` constructor internally. This preserves the completed-execution/fingerprint/baseline authority chain before bytes enter the trusted store.

Persistence is append-only:

- an absent record may be created;
- an identical existing record is idempotent;
- the same record id with different bytes/authority is a conflict;
- Phase 19 exposes no update, overwrite, delete, import, arbitrary-path, or cleanup API for retained records.

## No-follow and promotion law

All managed traversal follows the Phase 7 retained-capability pattern:

- absolute application root supplied by Builder/portable mode;
- `NeoData`, `history`, `debloat-removals`, `.staging`, unique staging sessions, nested `record`, final record directories, and record files opened without following symlinks/reparse paths;
- no `..`, caller separators, or caller-controlled relative paths;
- each unique staging session has a Neo ownership marker bound to the exact record id;
- the nested `record/receipt.json` is created exclusively, synced, re-read, and fully validated while the session marker remains intact;
- only the marker-free nested `record/` directory is renamed into the final record-id directory as the namespace promotion;
- the marker-owned session remains independently cleanable after successful promotion or a rename race;
- concurrent identical writers converge on one valid record plus idempotent already-present evidence;
- unexpected files/directories or link-like entries fail store audit/listing closed.

A crash may leave an inert marker-owned staging session containing no nested record or one validated nested `record/`. Final record enumeration never treats staging as history evidence. The store does not delete an unowned or marker-mismatched staging directory.

## Trust boundary and ACL policy

Phase 19 defines **store provenance against API/caller substitution**, not magical protection against an arbitrary process already running with the same operating-system principal and filesystem write rights.

The store inherits the ACL/security descriptor of the Builder/portable application root and `NeoData`; it does not silently rewrite ownership or grant broader filesystem rights. The no-follow/capability boundary prevents path substitution and traversal through the store API. Full same-principal tamper resistance would require a stronger service/key boundary and is not falsely claimed here.

A persisted receipt still does not grant mutation authority. Future MCP/RPC restore capability issuance must separately authenticate the caller, enforce permission/policy, bind explicit confirmation to the exact prepared transaction fingerprint, and issue the opaque Phase 18 capability.

## Trusted selection

Phase 19 may return read-only stored-record metadata and may load a validated stored receipt by typed record id.

The restore-preparation path must load the receipt from the same store by record id and then invoke the existing Phase 17 fresh inventory/readiness logic. It may not accept caller-supplied receipt JSON or a caller-supplied receipt path.

Prepared state remains read-only. Phase 18 execution is unreachable because Phase 19 does not own or issue `DebloatRestoreExecutorCapability`.

## Deliberate limits

Phase 19 does not implement or prove:

- public GUI/CLI restore mutation;
- MCP/RPC Debloat restore capability issuance;
- any constructor for `DebloatRestoreExecutorCapability`;
- AppX registration/removal;
- Store/network/vendor recovery acquisition;
- package staging/download;
- provisioned-image/all-users/batch restore;
- plugin dependency;
- deletion/editing/import of retained history records;
- resistance to malicious modification by another process running with the same OS principal and inherited write rights;
- live destructive AppX restore proof.

Those remain separately gated.

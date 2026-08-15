# Decision 0014 — Phase 14 Windows Live Debloat Inventory

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Product domain:** second bounded child of the frozen Debloat domain  
**Authority:** read-only Windows AppX inventory resolution only

## Decision

Phase 14 extends the proven Phase 13 debloat assessment model with real Windows evidence capture. It does not add package removal, provisioning mutation, restore/re-registration, transaction binding, public write commands, plugin dependency, MCP/RPC debloat authority, or capability issuance.

The flow is:

```text
validated Phase 13 debloat catalogue
        ↓
fixed Windows current-user AppX inventory
        +
fixed Windows online provisioned-AppX inventory
        ↓
Rust identity normalization/matching
        ↓
Phase 13 DebloatEvidence
```

## Windows read boundary

Neo uses exactly two fixed Microsoft-supported PowerShell inventory surfaces:

- `Get-AppxPackage` for packages installed in the current user profile;
- `Get-AppxProvisionedPackage -Online` for packages provisioned in the running Windows image for future users.

Both commands run through the already-proven `neo-probe::CommandRunner` evidence boundary using `powershell.exe -NoLogo -NoProfile -NonInteractive -Command` and a fixed script owned by Neo source.

Catalogue package IDs are never interpolated into PowerShell. Neo enumerates the approved inventory surfaces first, retains raw `CommandEvidence`, then performs case-insensitive package-name matching in Rust. A crafted catalogue therefore cannot turn package identity into executable text.

No `ExecutionPolicy Bypass` is used.

## Evidence law

A successful, valid current-user inventory produces `Present` or `Absent` installed evidence. A successful, valid provisioned inventory independently produces `Present` or `Absent` provisioned evidence.

A command start failure, non-zero command result, malformed JSON output, or identity-incomplete inventory record never proves absence. That entire evidence side becomes `Unavailable` and a warning is retained. Identity-incomplete records are not silently discarded because doing so could manufacture a false `Absent` result for a selected package.

Current-user package version is retained only when exactly one non-empty version is recovered for the matched package identity. Ambiguous/missing version data remains `None`; presence evidence is not discarded merely because version is unavailable.

The resulting observations are reconstructed through `DebloatEvidence::new`, so the frozen Phase 13 validation and identity-uniqueness laws remain authoritative.

## Isolation

Phase 13 `neo-debloat` remains platform-neutral and unchanged. Phase 14 lives in a separate `neo-debloat-probe` adapter crate depending only on `neo-debloat`, `neo-probe`, Serde/JSON, and error typing.

The Phase 14 crate has no dependency on `neo-transaction`, `neo-driverstore`, `neo-runtime-executor`, `neo-tweak-executor`, a plugin layer, or a Windows mutation crate.

Production Phase 14 source must not contain `Remove-AppxPackage`, `Remove-AppxProvisionedPackage`, `Add-AppxPackage`, `Add-AppxProvisionedPackage`, package-manager mutation APIs, DISM remove/add operations, Winget execution, or arbitrary caller-supplied command text.

## Proof binary

`neo-debloat-live-scan <catalogue.json>` is an internal engineering proof binary. It validates the Phase 13 catalogue, captures the fixed live Windows inventories, emits the normalized evidence/report, and states `Machine changes: none`.

It is not a public debloat apply command.

## Proof boundary

Phase 14 proves:

- successful real Windows current-user AppX inventory execution in Windows CI;
- successful real Windows provisioned-AppX inventory execution in Windows CI;
- fixed command ownership and non-interpolation of catalogue identities;
- raw command evidence retention;
- fail-closed normalization to `Unavailable` on query, parse, or identity-completeness uncertainty;
- case-insensitive identity matching;
- Phase 13 evidence-constructor reuse;
- a Windows CI live read-only proof with fixture-tree before/after equality;
- continued absence of debloat mutation authority.

Phase 14 does **not** prove:

- all-users installed AppX state;
- package or provisioning removal;
- restore/re-registration or Store availability;
- transaction-bound rollback;
- real-package safety certification;
- public GUI/CLI debloat mutation;
- plugin/MCP/RPC debloat execution.

Those remain separately gated.

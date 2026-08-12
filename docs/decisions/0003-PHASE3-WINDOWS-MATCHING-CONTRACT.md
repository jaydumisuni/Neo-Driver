# Decision 0003 — Phase 3 Windows Matching Contract

**Status:** Accepted for Phase 3 implementation  
**Scope:** Read-only driver candidate matching/ranking

## Decision

Neo Phase 3 will preserve Windows driver-selection evidence rather than inventing a generic update score.

1. Device hardware IDs and compatible IDs remain opaque ordered strings.
2. INF applicability is represented as individual Models entries: the optional `hw-id` slot plus that entry's ordered compatible-ID list, with at least one identifier required.
3. Phase 3 computes the Microsoft identifier-score component for the four device/INF match classes:
   - device hardware → INF hardware: `0x0000 + device hardware position`;
   - device hardware → INF compatible: `0x1000 + device hardware position`;
   - device compatible → INF hardware: `0x2000 + device compatible position`;
   - device compatible → INF compatible: `0x3000 + device compatible position + (INF compatible position * 0x100)`.
4. Identifier scores are emitted only when list positions fit the documented `0x0000..=0x3fff` THHH range; Neo does not wrap or saturate out-of-range evidence.
5. Architecture and Windows build applicability are hard gates.
6. Invalid signature evidence is rejected. Unknown/unsigned signature evidence remains `INVESTIGATE`, never `CERTIFIED`.
7. Driver date and version are tie-breakers only after equal available safety/match evidence. Version is consulted only when both candidate dates are known and equal; missing/unparseable higher-priority evidence cannot manufacture a winner. A newer generic package cannot outrank a better identifier match merely because it is newer.
8. Phase 3 does **not** claim the complete Windows driver rank. Windows rank also includes signature score and FeatureScore; Phase 2 does not yet contain exact values for those rank components.
9. Match output is explanatory/read-only. It creates no install authority and performs no staging, binding, Driver Store, BCD, service, filter, or reboot mutation.

## Why the Phase 2 schema changed

`DriverArtifact.ids: OrderedDeviceIds` flattened INF-side entries into a device-shaped list. That loses the per-Models-entry compatible-ID position used by Windows identifier scoring and can corrupt candidate rank evidence.

Phase 3 therefore refines the catalogue to `DriverArtifact.models: Vec<InfModelEntry>` before implementing ranking.

## Primary evidence

- Microsoft Learn — Identifier Score:
  https://learn.microsoft.com/en-us/windows-hardware/drivers/install/identifier-score--windows-vista-and-later-
- Microsoft Learn — How Windows Ranks Driver Packages:
  https://learn.microsoft.com/en-us/windows-hardware/drivers/install/how-windows-ranks-driver-packages
- Microsoft Learn — INF Models Section:
  https://learn.microsoft.com/en-us/windows-hardware/drivers/install/inf-models-section
- Microsoft Learn — Device Identification Strings:
  https://learn.microsoft.com/en-us/windows-hardware/drivers/install/device-identification-strings

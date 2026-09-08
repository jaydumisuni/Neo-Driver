# Decision 0022 — Phase 22 Driver Store / PnP Repair Assessment Foundation

**Status:** REOPENED FOR FINAL CLOSEOUT / EXACT-HEAD REPROOF

## Why this phase exists

The frozen Neo Driver master plan places Driver Store/PnP repair and device re-enumeration in the Repair domain. Phase 21 completed the first Repair slice around DISM, SFC, and Windows Features while explicitly deferring Driver Store/PnP repair beyond the existing driver executor authority.

Phase 22 opens that deferred child conservatively. It does **not** add another driver executor. It consumes the exact read-only device and Driver Store evidence already owned by Phase 2 and Phase 5 and derives a bounded repair candidate.

## Authority binding

Phase 22 is derived from exact canonical Neo `main` `5e791fd6509a818b8f6632d57e1c74ffbc258461`, the frozen master plan, the Phase 5 controlled-driver decision, and the frozen/proven Phase 21 decision.

The real Tenfold scope campaign `neo-phase22-scope-tenfold-workspace` independently passed before implementation authority was opened. It produced four deterministic authority evidence packets, zero worker failures, zero semantic failures, no Council disagreement, and exposed only `PHASE22_DRIVER_PNP_ASSESSMENT` as the ready frontier.

## Phase 22 product boundary

Phase 22 may:

1. read exact present-device identity and PnP health evidence;
2. read the current active driver binding;
3. when the active binding is a Phase 5 OEM published INF (`oem<digits>.inf`), resolve it to the exact current Driver Store package through the existing Phase 5 read authority; inbox/system INF identities remain valid active-binding evidence but do not become Phase 5 reversible package authority;
4. retain current upper/lower filter evidence without inferring that a filter is faulty merely because it exists;
5. derive one conservative, typed repair candidate from that evidence;
6. expose the result through read-only `neo repair drivers` inspection.

Phase 22 has `machine_changes = false`.

## Existing authority must be reused

Phase 22 does not duplicate SetupAPI/NewDev mutation logic. Live Windows collection uses the existing Phase 5 `DriverHost` contract:

- `inventory()` for present devices and current binding/problem evidence;
- `resolve_published_package()` only for active Phase 5 OEM published INF identities (`oem<digits>.inf`); inbox/system INF identities remain visible binding evidence and are conservatively represented without exact reversible package evidence.

The public Windows `DriverHost` is a fail-closed inventory wrapper around the established Phase 5 mutation backend. The wrapper owns the present-device evidence read boundary; mutation methods continue delegating to the established backend and are not widened by Phase 22.

No Phase 22 path may call:

- `stage_driver()`;
- `install_best_match()`;
- `restore_specific_driver()`;
- `remove_published_package()`.

A regression host deliberately panics if those methods are reached.

## Inherited PnP-status semantic law

Phase 22 must consume the actual Phase 5 Windows host contract rather than reinterpret the generic `Option<u32>` field:

- Phase 5 calls `CM_Get_DevNode_Status` for every present device;
- a Config Manager query failure aborts inventory and cannot produce a device record;
- a successful Windows problem code of zero is normalized by Phase 5 to `problem_code = None`;
- a successful nonzero Windows problem code is retained as `problem_code = Some(code)`.

Therefore Phase 22 normalizes every accepted device into an explicit `PnpStatusEvidence`:

- `NoProblem` must correspond exactly to inherited `problem_code = None`;
- `Problem { code }` must contain a nonzero code exactly equal to the inherited `problem_code = Some(code)`;
- `Some(0)` is non-canonical Phase 5 evidence and is rejected;
- fixture/import evidence must provide the explicit Phase 22 `pnp_status` field, so omission/defaulting cannot turn absent evidence into a no-problem claim.

Windows `CM_PROB_DISABLED` / Device Manager Code 22 is treated as `Disabled` even when the generic Phase 2 `disabled` field is unavailable. If an explicit `disabled` field contradicts Code 22, the normalized evidence fails closed.

## Evidence law

Each device assessment retains:

- exact device instance ID;
- description when available;
- explicit normalized PnP status plus the inherited problem code representation;
- disabled evidence when available;
- exact active published INF when available;
- exact resolved Driver Store package when available;
- upper/lower filter evidence;
- typed assessment state;
- typed bounded repair route;
- human-readable explanation;
- report-level SHA-256 over normalized evidence.

Case-insensitive duplicate device instance IDs fail closed. PnP status that disagrees with inherited Phase 5 problem-code evidence fails closed. Driver Store package evidence without an active published INF fails closed. A resolved package whose published identity does not equal the active published INF fails closed.

Raw SetupAPI hardware/compatible-ID lists are stable-deduplicated case-insensitively at the Windows evidence boundary while preserving the exact spelling and ranking position of the first occurrence. This matches Neo's case-insensitive hardware/compatible-ID matching semantics and prevents a case-only duplicate from consuming a later ranking position. Inbox/system INF bindings such as `machine.inf` remain valid active-binding evidence but are never promoted into Phase 5 OEM rollback-package authority. Imported or fixture `current_package` evidence is accepted only when both the active binding and the package published identity are canonical, unpadded Phase 5 `oem<digits>.inf` identities **and** `driver_store_inf` has the fully qualified Windows-root `System32\DriverStore\FileRepository\<package>\<original>.inf` shape with exactly one root component before `System32`. When imported `active_driver.original_name` is supplied, it must be canonical and nonempty and must equal the final `driver_store_inf` filename case-insensitively; supplied identity is never trimmed into authority or silently ignored.

Imported Phase 22 JSON uses a private strict deserialization wire at every authority layer: root evidence, per-device evidence, device record, ordered IDs, active binding, PnP-status envelope, and exact package all reject unknown fields. Root `DriverRepairEvidence` Serde deserialization performs semantic `validate()` before returning, so direct root deserialization cannot bypass the same PnP/package/device consistency law enforced by `from_json_str()`.

The public Windows inventory boundary uses the unified device-property model (`SetupDiGetDevicePropertyW` with `DEVPKEY_Device_*`) for hardware IDs, compatible IDs, active INF, description/manufacturer/class metadata, and upper/lower filters. This is deliberate: the legacy `SetupDiGetDeviceRegistryPropertyW` contract uses `ERROR_INVALID_DATA` for both a missing property and invalid property data, so it cannot prove absence without also risking malformed evidence being collapsed into emptiness. On the public boundary, only unified-property `ERROR_NOT_FOUND` is accepted as absence; insufficient-buffer, type, size, and UTF-16 failures propagate. String evidence must contain exactly one final NUL with no embedded/trailing data, while string-list evidence must use the canonical list terminator and cannot hide trailing values behind an early empty element. The anti-drift guard normalizes whitespace before checking that the sizing call's result is not discarded, so equivalent Rust formatting cannot bypass the proof. `ClassGuid` is taken directly from the enumerated `SP_DEVINFO_DATA.ClassGuid` field. Missing filter properties may therefore produce empty lists, but malformed/query-failed filter evidence cannot be manufactured as empty.

## Assessment states

Phase 22 uses exactly these assessment states:

- `Healthy`;
- `Disabled`;
- `MissingDriverBinding`;
- `PnpProblem`;
- `EvidenceUnavailable`.

A successful Phase 5 `NoProblem` observation can become `Healthy` only when exact active binding and exact Driver Store package continuity are also proven.

## Repair routes

Phase 22 emits only non-executable candidate routes:

- `NoAction` — PnP reports no problem and exact current binding/package continuity is proven;
- `CurrentExactDriverReinstallCandidate` — PnP reports a non-disabled problem and the exact current published INF plus exact Driver Store package are proven; a later authority phase may evaluate the actual reinstall;
- `DriverSelectionRequired` — PnP reports an actual problem and there is no active binding, so any future repair must return to the existing matcher/catalogue authority;
- `ManualInvestigation` — evidence is incomplete, contradictory, disabled, or otherwise insufficient for a bounded candidate.

A no-problem device with no active driver binding does **not** automatically become `DriverSelectionRequired`; Phase 22 will not manufacture a driver problem that Windows did not report.

A route is not mutation authority.

## Fail-closed rules

- Config Manager status-query failure never reaches Phase 22 as a healthy device because Phase 5 aborts that inventory read.
- `problem_code = None` means a successful inherited no-problem observation; it is not missing evidence inside the Phase 5 Windows host contract.
- `problem_code = Some(0)` is rejected as non-canonical Phase 5 evidence.
- Imported/fixture `pnp_status` must explicitly agree with the inherited `problem_code` representation.
- Imported Phase 22 JSON rejects unknown fields at every authority layer, and direct root Serde deserialization performs semantic validation before returning evidence.
- Device Manager Code 22 is `Disabled`, never an exact-current-driver reinstall candidate.
- An active binding without a valid published `.inf` identity cannot establish exact Driver Store continuity.
- A Phase 5 OEM published INF that cannot resolve to an exact current package cannot establish reversible repair readiness.
- A valid inbox/system INF binding is not an error, but because it is outside Phase 5 OEM rollback-package authority it carries no exact reversible-package claim and cannot become an exact-current-driver reinstall candidate.
- Imported or fixture `current_package` evidence for an inbox/system or otherwise non-OEM INF is rejected rather than promoted into exact-package authority.
- Imported or fixture exact-package authority rejects surrounding whitespace in the active published INF rather than trimming it into a Phase 5 OEM identity.
- Imported or fixture exact-package evidence whose `driver_store_inf` is not a fully qualified Windows-root `System32\DriverStore\FileRepository\<package>\<original>.inf` path with exactly one root component before `System32` is rejected.
- When imported `active_driver.original_name` is supplied with exact package evidence, it must be canonical and nonempty and must match the final Driver Store INF filename case-insensitively; mismatch, padding, or whitespace-only evidence fails closed.
- On the public Windows inventory boundary, only unified-property `ERROR_NOT_FOUND` is absence; sizing, property-type, byte-count, missing/early UTF-16 terminators, trailing UTF-16 data, invalid UTF-16, or other SetupAPI failures abort that inventory read rather than becoming empty evidence.
- A disabled device is recorded as disabled; Phase 22 does not enable or re-enumerate it.
- Filters are retained as evidence but are not blamed automatically.
- No caller-supplied raw SetupAPI/PnP command or arbitrary shell adapter exists.

## CLI boundary

`neo repair drivers` is read-only.

- On Windows with no `--evidence`, it reads the live host through the existing Phase 5 read authority and the fail-closed public Windows inventory boundary.
- With `--evidence <file>`, it requires and validates normalized Phase 22 PnP-status evidence and derives the same deterministic assessment on any supported CI host. A filesystem read failure identifies the exact evidence path as well as the underlying OS error.
- `--json` emits the complete typed report.

The CLI cannot construct or obtain Phase 5 driver mutation authority.

## Explicitly deferred

Phase 22 does not authorize:

- device re-enumeration execution;
- device enable/disable execution;
- driver staging or installation;
- current-driver reinstall execution;
- rollback execution;
- Driver Store package deletion or cleanup;
- forced/lower-ranked driver binding;
- Windows Update repair;
- networking repair;
- Winget repair;
- AppX repair;
- restore/recovery mutation;
- arbitrary PnP or SetupAPI commands.

Those require later typed authority phases.

## Proof requirement

Phase 22 requires:

- a dedicated 20-lane static review bound into the normal Ubuntu/Windows matrix;
- locked workspace build and Clippy with warnings denied;
- complete workspace tests;
- focused `neo-driver-repair` unit/adversarial proof;
- deterministic fixture proof through `neo repair drivers --evidence ...` on both CI platforms;
- a live Windows `neo repair drivers --json` proof using only present-device inventory and exact current-package resolution where Phase 5 OEM package authority applies;
- regression proof that inherited Phase 5 `None` is a successful no-problem observation and that non-canonical/mismatched PnP evidence fails closed;
- regression proof that Code 22 is disabled and cannot become a reinstall candidate;
- regression proof that duplicate SetupAPI IDs are normalized case-insensitively while preserving the first occurrence's exact spelling/order and that only `oem<digits>.inf` enters Phase 5 exact-package resolution;
- regression proof that imported/fixture `current_package` evidence cannot bypass the Phase 5 OEM published-INF boundary, claim exact Driver Store authority with an arbitrary non-Driver-Store path, or hide extra path components before `System32`;
- regression proof that imported Phase 22 JSON rejects unknown fields at every authority layer and that direct root Serde deserialization cannot bypass semantic validation;
- regression proof that imported exact-package authority rejects padded active published INF identity, requires any supplied original INF to be canonical and nonempty, and cross-checks that original INF against the final Driver Store path filename;
- anti-drift proof that the exported Windows host uses the unified `DEVPKEY_Device_*` property boundary for IDs, active INF, and filter evidence, accepts only `ERROR_NOT_FOUND` as property absence, validates property types and exact canonical UTF-16/string-list termination, normalizes whitespace before checking for discarded SetupAPI sizing results, and does not use `SetupDiGetDeviceRegistryPropertyW` for public inventory evidence;
- no unresolved material external-review finding before freeze;
- final exact-head proof before merge.

## Frozen implementation proof history and current reopen

The earlier production-source implementation proof head before this correction cycle was `9ffd3175a23068a1c513b88ada27f86aa7c96f55`. Neo Driver CI run `32555744807` completed successfully on both Ubuntu and Windows. That historical proof remains evidence for the state it tested, but it is **not** sufficient proof for the later authority-boundary corrections below.

The corrected implementation head `1eb464370c97872ebdbbd83d200a8838f4735c5a` passed exact-head Neo Driver CI run `34036106250` on Ubuntu and Windows. That run includes locked build/type proof, Clippy with warnings denied, complete workspace units, the focused Phase 22 assessment proof, deterministic fixture proof, and the live Windows `neo repair drivers --json` source proof. It establishes the implementation state before the final review/closeout-only corrections described below.

The evidence campaign, external review, and correction cycle found the following Windows/authority-boundary and proof defects:

1. raw SetupAPI hardware/compatible-ID lists can contain duplicate entries, so the Windows adapter stable-deduplicates them while retaining first-occurrence ranking order;
2. valid inbox/system INF bindings such as `machine.inf` are binding evidence but do not enter Phase 5's OEM-only exact rollback-package resolver;
3. imported evidence could pair a non-OEM binding such as `machine.inf` with matching `current_package` data and falsely claim exact-package authority, so shared validation rejects non-OEM package evidence and a dedicated regression proves the boundary;
4. live Windows inventory previously emitted empty upper/lower filter arrays regardless of actual device filter properties, so the inventory boundary was changed to collect real filter evidence;
5. imported OEM-shaped `current_package` evidence could still claim exact-package continuity with an arbitrary non-Driver-Store path, so normalized evidence now requires a fully qualified Driver Store `FileRepository` INF path shape;
6. the legacy registry-property sizing probe discarded its first SetupAPI result and the subsequent strict wrapper still treated `ERROR_INVALID_DATA` as property absence, even though that code also represents invalid property data. The exported Windows inventory host now uses the unified `SetupDiGetDevicePropertyW`/`DEVPKEY_Device_*` model, where `ERROR_NOT_FOUND` is the distinct absence signal, and fails closed on sizing/type/UTF-16 errors;
7. the strict UTF-16 decoders still accepted unterminated strings and could stop at an early NUL without proving that no trailing evidence remained. The public boundary now requires canonical final termination for both strings and string lists and rejects missing, early, malformed, or trailing termination data;
8. the imported Driver Store path validator required a `System32\DriverStore\FileRepository` segment but only rejected the zero-prefix case, so a forged path such as `C:\Temp\Nested\System32\DriverStore\FileRepository\...` could be promoted into exact-package evidence. Exact imported authority now requires precisely one Windows-root component before `System32`;
9. SetupAPI ID deduplication was exact-case while Neo matching is case-insensitive, allowing a case-only duplicate to consume a ranking position and demote the next distinct ID. The Windows evidence boundary now deduplicates hardware/compatible IDs case-insensitively while retaining the first occurrence's spelling and order;
10. the evidence-file CLI read error omitted the input path and surfaced only generic filesystem text. The read-only CLI now reports the exact evidence path together with the underlying error;
11. the anti-drift proof for a discarded `SetupDiGetDevicePropertyW` sizing result matched one indentation shape, so equivalent reformatting could bypass both the Rust boundary test and Python static review. Both gates now normalize whitespace before applying that prohibition;
12. imported Phase 22 JSON previously used ordinary Serde shape at nested authority layers, so unknown fields could be silently discarded before the normalized evidence digest. Phase 22 now uses strict private import wires at every authority layer, and root Serde deserialization also invokes semantic validation so the explicit helper is not a privileged validation path;
13. imported exact-package evidence could supply an `active_driver.original_name` that contradicted the final filename of `driver_store_inf`. When original INF evidence is supplied it is now cross-checked case-insensitively against that Driver Store filename;
14. active imported published INF identity was trimmed before the Phase 5 OEM predicate, allowing a padded value such as `" oem40.inf "` to masquerade as exact package authority. Exact authority now preserves the supplied identity and rejects surrounding whitespace rather than normalizing it into trust;
15. supplied imported original-INF evidence was also trimmed and whitespace-only values were silently ignored. Supplied original INF is now required to be canonical and nonempty before filename continuity is evaluated.

Corrections 5 through 15 do not grant mutation authority. They narrow what may become trusted read evidence, preserve the ranking semantics of that read evidence, or strengthen the proof/diagnostic boundary. The existing Phase 5 OEM mutation boundary remains unchanged.

This decision returns to **FROZEN AND PROVEN** only after the final closeout head containing these documentation and anti-drift updates passes the complete exact-head Ubuntu/Windows CI matrix and no further material review finding remains.

Live driver/PnP mutation is explicitly unclaimed.

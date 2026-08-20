# Decision 0022 — Phase 22 Driver Store / PnP Repair Assessment Foundation

**Status:** ACCEPTED FOR IMPLEMENTATION PROOF

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
3. resolve the active Windows-published INF to the exact current Driver Store package through the existing Phase 5 read authority;
4. retain current upper/lower filter evidence without inferring that a filter is faulty merely because it exists;
5. derive one conservative, typed repair candidate from that evidence;
6. expose the result through read-only `neo repair drivers` inspection.

Phase 22 has `machine_changes = false`.

## Existing authority must be reused

Phase 22 does not duplicate SetupAPI/NewDev logic. Live Windows collection uses the existing Phase 5 `DriverHost` contract:

- `inventory()` for present devices and current binding/problem evidence;
- `resolve_published_package()` for exact current Driver Store package resolution.

No Phase 22 path may call:

- `stage_driver()`;
- `install_best_match()`;
- `restore_specific_driver()`;
- `remove_published_package()`.

A regression host deliberately panics if those methods are reached.

## Evidence law

Each device assessment retains:

- exact device instance ID;
- description when available;
- PnP problem code when available;
- disabled evidence when available;
- exact active published INF when available;
- exact resolved Driver Store package when available;
- upper/lower filter evidence;
- typed assessment state;
- typed bounded repair route;
- human-readable explanation;
- report-level SHA-256 over normalized evidence.

Case-insensitive duplicate device instance IDs fail closed. Driver Store package evidence without an active published INF fails closed. A resolved package whose published identity does not equal the active published INF fails closed.

## Assessment states

Phase 22 uses exactly these assessment states:

- `Healthy`;
- `Disabled`;
- `MissingDriverBinding`;
- `PnpProblem`;
- `EvidenceUnavailable`.

Unknown PnP problem-code evidence never becomes `Healthy`.

## Repair routes

Phase 22 emits only non-executable candidate routes:

- `NoAction` — PnP reports no problem and exact current binding/package continuity is proven;
- `CurrentExactDriverReinstallCandidate` — PnP reports a problem and the exact current published INF plus exact Driver Store package are proven; a later authority phase may evaluate the actual reinstall;
- `DriverSelectionRequired` — there is no active binding and any future repair must return to the existing matcher/catalogue authority;
- `ManualInvestigation` — evidence is incomplete, contradictory, disabled, or otherwise insufficient for a bounded candidate.

A route is not mutation authority.

## Fail-closed rules

- `problem_code = None` is evidence unavailable, never healthy.
- An active binding without a valid published `.inf` identity cannot establish exact Driver Store continuity.
- A published INF that cannot resolve to an exact current package cannot establish reversible repair readiness.
- A disabled device is recorded as disabled; Phase 22 does not enable or re-enumerate it.
- Filters are retained as evidence but are not blamed automatically.
- No caller-supplied raw SetupAPI/PnP command or arbitrary shell adapter exists.

## CLI boundary

`neo repair drivers` is read-only.

- On Windows with no `--evidence`, it reads the live host through the existing Phase 5 read authority.
- With `--evidence <file>`, it validates normalized Phase 22 evidence and derives the same deterministic assessment on any supported CI host.
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
- a live Windows `neo repair drivers --json` proof using only present-device inventory and exact current-package resolution;
- no unresolved material external-review finding before freeze;
- final exact-head proof before merge.

Live driver/PnP mutation is explicitly unclaimed.

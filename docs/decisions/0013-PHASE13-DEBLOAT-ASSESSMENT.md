# Decision 0013 — Phase 13 Debloat Assessment Foundation

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Product domain:** Debloat  
**Authority:** read-only AppX/debloat catalogue and evidence assessment only

## Decision

Phase 13 begins Neo's frozen Debloat product domain with a deliberately read-only, platform-neutral assessment layer.

It does **not** remove AppX packages, change provisioning, invoke PowerShell, issue a mutation capability, or bind debloat actions into the transaction engine. Its job is to make the evidence and policy model correct before any Windows package-changing backend exists.

The Phase 13 flow is:

```text
validated Neo debloat catalogue
        +
validated captured AppX evidence
        +
explicit selected Neo debloat IDs
        +
selected profile preservation policy
        ↓
read-only deterministic assessment
        ↓
RemovalCandidate / AlreadyAbsent / NeedsReview /
BlockedByProfile / BlockedProtected / BlockedPolicy
```

`RemovalCandidate` is a review classification, **not execution authority**.

## Donor boundary

`jaydumisuni/winutil/config/appx.json` is an approved donor for catalogue breadth and metadata shape only. It demonstrates useful fields such as package identity, Store identity, category, title and description.

WinUtil package presence in that file is **not** accepted as evidence that a package is safe to remove, safe to provisionally remove, restorable, low risk, dependency-free, or appropriate for a Neo profile. Neo must independently establish those facts before a real Windows package can receive a certified removal classification.

Phase 13 fixtures therefore use synthetic `Contoso.*` package identities. This phase makes no safety claim about a real Microsoft, OEM, Store, gaming, technician or Windows system package.

## Typed catalogue contract

Every `DebloatDefinition` contains:

- a Neo-owned action ID distinct from package identity;
- AppX package identity;
- title, category and description;
- one of the frozen classes `SafeOptional`, `FeatureDependent`, `DependencySensitive`, or `ProtectedManualOnly`;
- intended scope: current user, provisioned image, or both;
- Neo risk, recommendation and evidence verdict;
- explicit default-selection state;
- declared restore route;
- side-effect text;
- profiles in which the item must be preserved.

Catalogue construction and Serde deserialization run the same validation. Duplicate Neo IDs and case-insensitive duplicate package identities fail closed.

## Default-selection law

A debloat item may be selected by default only when all of the following are true:

- class is `SafeOptional`;
- risk is exactly `Low`;
- evidence verdict is `Certified`;
- recommendation is explicitly suitable for optional removal (`Recommended` or `OptionalComponent`);
- a declared restore route exists;
- the Safe Cleanup profile does not preserve it.

`Custom` never receives hidden defaults. Gaming, Technician and Developer profiles remove a default from their preselection whenever that profile preserves the item.

These are recommendation/preselection rules only. Phase 13 does not apply them to Windows.

## Evidence contract

`DebloatEvidence` records captured package observations independently from catalogue policy.

Every observation contains:

- AppX package identity;
- current-user installed presence;
- provisioned-image presence;
- optional observed version;
- evidence source.

Presence is one of `Present`, `Absent`, or `Unavailable`.

Case-insensitive duplicate package observations fail closed. For a selected item, Phase 13 requires both installed and provisioned evidence to be available even when the requested future removal scope is narrower. This deliberately favors complete state capture over a permissive partial assessment.

Missing or unavailable selected-package evidence does not become a removal candidate.

## Explicit selection and preservation

Assessment requires a non-empty explicit list of Neo debloat IDs. Duplicate and unknown selected IDs fail closed.

Profile preservation is a hard assessment boundary. If the selected profile preserves an otherwise removable package, the result is `BlockedByProfile` rather than a candidate.

`ProtectedManualOnly` is a stronger boundary than profile policy and returns `BlockedProtected` whenever the package is present. It never becomes normal debloat authority.

## Candidate law

A present package can be labelled `RemovalCandidate` only when it is:

- `SafeOptional`;
- `Low` risk;
- `Certified`;
- explicitly recommended as `Recommended` or `OptionalComponent`;
- not preserved by the selected profile;
- backed by a declared restore route.

A non-certified, higher-risk, unknown-recommendation, feature-dependent, dependency-sensitive, or restore-less item remains `NeedsReview`. A rejected/unsupported/conflicting/do-not-touch policy state is `BlockedPolicy`. Protected/manual-only remains `BlockedProtected`.

A package already absent for its declared scope is `AlreadyAbsent`.

## Restore semantics

Phase 13 models a **declared restore route**, not proven successful rollback.

Supported metadata forms are:

- Store ID;
- provisioned-image restore;
- validated non-empty vendor source;
- none.

A syntactically valid Store ID or vendor source does not prove that restoration currently succeeds. Future mutation authority must independently prove the selected package's actual restoration path and captured pre-state before claiming reversible removal.

## Proof binary

`neo-debloat-assess` is an internal engineering proof binary. It reads validated catalogue/evidence JSON, applies profile and selection policy, and emits text or JSON assessment.

Its text surface must state `Machine changes: none`. The behavioral regression snapshots the fixture directory before execution and proves it is byte-for-byte unchanged afterward.

This binary is not the installed `neo` product mutation CLI and does not expose debloat apply/remove authority.

## Phase 13 authority isolation

`neo-debloat` has no dependency on:

- Windows APIs;
- `neo-transaction`;
- `neo-tweak-executor`;
- `neo-runtime-executor`;
- `neo-driverstore`.

Production Phase 13 code must not launch PowerShell, `cmd.exe`, AppX commands, DISM, Winget, package managers, or arbitrary child commands.

MCP/RPC remains Neo's primary future mutation control plane, but Phase 13 issues no MCP/RPC debloat capability and adds no debloat RPC method.

## Proof boundary

Phase 13 proof covers:

- typed catalogue and evidence validation;
- duplicate/case-insensitive identity rejection;
- default-selection safety;
- profile preservation;
- explicit selection;
- missing/unavailable evidence handling;
- protected/policy blocking;
- candidate eligibility;
- declared restore metadata;
- deterministic read-only behavior;
- no production Windows/process/transaction/executor dependency.

It does **not** prove:

- real Windows AppX inventory probing;
- package removal;
- provisioned-package removal;
- package restore/re-registration;
- transaction-bound AppX rollback;
- Store availability;
- real-package safety classification;
- all-users mutation;
- PowerShell/AppX cmdlet correctness;
- public GUI/CLI/MCP/RPC debloat mutation authority.

Those remain separately gated.

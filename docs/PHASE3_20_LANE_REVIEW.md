# Neo Driver — Phase 3 20-Lane Review

**Scope:** deterministic read-only driver candidate matching/ranking.  
**Mutation boundary:** no staging, install, removal, Driver Store write, device binding, filter change, BCD/security change, download, or reboot operation.

## Pre-publication findings / corrections

1. Phase 2's INF applicability was shaped like device evidence (`OrderedDeviceIds`) and could flatten multiple Models entries. Phase 3 refines it to explicit `InfModelEntry` records before matching.
2. A simple "newer version wins" model was rejected. Identifier match class/order precedes date/version.
3. A fake full Windows rank was rejected. Exact signature-score and FeatureScore values are not yet available, so Phase 3 exposes only identifier-score evidence plus Neo safety state and marks full rank unavailable.
4. Missing architecture metadata fails closed rather than silently meaning "all architectures."
5. Invalid signatures reject candidates; unknown/unsigned signatures remain `INVESTIGATE`.
6. Device and INF IDs are compared as opaque strings. Neo does not parse VID/PID/SUBSYS fragments to manufacture compatibility.
7. INF compatible-ID position is retained and resets for each Models entry for the fourth identifier-score class.
8. Equal available match quality uses normalized driver date and numeric version only as tie-break evidence; unparseable values do not create a fabricated advantage.
9. CLI exposure is read-only and reports `Machine changes: none`.
10. Identifier-score arithmetic is checked against the documented `0x0000..=0x3fff` range; out-of-range matches fail closed instead of wrapping/saturating.
11. A missing/unparseable driver date blocks version from manufacturing a winner because Windows date precedence cannot be proven.
12. Case-only duplicate INF compatible IDs are rejected because matcher equality is case-insensitive.
13. The Phase 3 review was corrected to inspect external `tests.rs` after tests were split from the library module.
14. Calendar validation now rejects impossible non-leap dates so malformed date evidence cannot participate in a Windows tie-break.

## Required proof before merge

- Phase 1 20-lane review;
- strengthened Phase 2 20-lane review;
- Phase 3 20-lane review;
- tracked/current Cargo.lock;
- rustfmt;
- locked workspace type/build proof;
- Clippy with warnings denied;
- unit tests;
- catalogue CLI fixture;
- matcher CLI fixture;
- external PR review and recursive correction of any valid finding.

Phase 3 does not prove live attached-device behavior and does not enable installation.

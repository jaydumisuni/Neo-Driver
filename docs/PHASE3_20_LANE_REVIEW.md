# Neo Driver — Phase 3 20-Lane Review

**Scope:** deterministic read-only driver candidate matching/ranking.  
**Mutation boundary:** no staging, install, removal, Driver Store write, device binding, filter change, BCD/security change, download, or reboot operation.

## Findings corrected

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
15. First CI found a brittle Phase 3 review assertion that searched for one exact prose phrase to prove "no full rank claim." The gate now checks structural markers (`full_windows_rank_available: false`, `ranking_complete`, `FeatureScore`, and the explicit no-full-rank note) instead of capitalization-sensitive prose.
16. Adding `neo-match` changed the workspace dependency graph. CI regenerated the exact `Cargo.lock`; that generated lock was committed unchanged and subsequently proved current/tracked with `--locked` gates.
17. Rustfmt found layout-only drift in `neo-catalogue`, `neo-cli`, `neo-match`, and matcher tests. The exact formatter output was applied without changing matching behavior.
18. Clippy found the internal score-evidence helper accepted eight arguments. The finding was corrected structurally with a typed `MatchCoordinates` record; no `allow` suppression was added and score/evidence semantics remain unchanged.

No warning or finding was suppressed with an allow-list escape hatch.

## Implementation-code proof

Corrected implementation head: `7bd9471913cb13e583ff53293637d5a20c1dbe2e`  
GitHub Actions run: `31619245616`

Both **Ubuntu** and **Windows** passed:

- Phase 1 20-lane static review;
- strengthened Phase 2 20-lane static review;
- Phase 3 20-lane static review;
- committed + Git-tracked Cargo.lock/current dependency graph guard;
- Rust formatting;
- locked workspace type/build check;
- Clippy with warnings denied;
- Rust unit tests, including matcher ranking/tie/overflow/calendar regressions;
- read-only catalogue CLI fixture;
- read-only matcher CLI fixture.

External CodeRabbit review was rate-limited during this proof cycle and produced no code review threads. Therefore no full external-review PASS is claimed. The provider's generic docstring-coverage warning remains a visible non-functional documentation-quality warning and is not represented as a correctness/security finding.

Phase 3 remains read-only. Live attached-device behavior and machine mutation are not claimed by this proof. A final CI run on the documentation-closed branch state is required before merge.

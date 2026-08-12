# Neo Driver — Phase 4 20-Lane Review

**Scope:** transaction plan, checkpoint, authority, reboot/resume, verification, and rollback contracts.  
**Mutation boundary:** no Windows-changing executor is attached.

## Pre-publication review obligations

1. Reuse `neo-core` action/risk/evidence/reboot contracts rather than duplicate policy.
2. Bind authority to an exact SHA-256 plan fingerprint.
3. Preserve actual pre-state as typed baseline evidence.
4. Require exact snapshot coverage for declared state targets and reject cross-action snapshot ownership.
5. Fail closed when a reversible rollback target cannot be captured.
6. Require authorization to cover exactly the transaction action set.
7. Require manual override for uncertainty while permanently rejecting `REJECTED` evidence.
8. Require separate HIGH/EXPERT risk acknowledgement.
9. Require explicit irreversible-action acknowledgement with a reason.
10. Ensure apply success never transitions directly to COMPLETE.
11. Persist required reboot state with exact plan identity and restoration obligations.
12. Require post-reboot re-probe proof before continuing.
13. Recompute verification status from observations/baseline; do not persist a trusted PASS bit.
14. Require exact postcondition proof before COMPLETE.
15. Restore captured baseline rather than presumed defaults.
16. Require rollback application records plus rollback verification before ROLLED_BACK.
17. Route partial apply failure to rollback only when every changed action is reversible.
18. Enforce checkpoint stage invariants and event-log ordering on persisted state.
19. Keep transaction CLI strictly validation/template-only; no authority or advancement calls.
20. Keep the Phase 4 crate free of Windows/process execution paths and exercise the contract with a synthetic reversible fixture.

## Findings closed before final proof

1. The first CI cycle intentionally exposed a stale Phase 3 `Cargo.lock`; the exact Phase 4 lock graph was recovered from CI and committed without manually selecting dependency versions.
2. Stable `rustfmt` found layout drift before compilation; only formatter output was applied.
3. External review found `Blocked` had no recovery path after failed post-reboot proof. Added bounded re-probe recovery: prove and continue, roll back fully reversible changed actions, or fail closed.
4. External review found byte-exact `StateTarget` identity could let case variants of a Windows target bypass snapshot-ownership checks. Typed Windows targets now compare using case-normalized identity, with an adversarial case-variant regression.
5. External review found the Phase 4 static scanner was not recursive. Transaction and CLI source collection now uses deterministic recursive Rust-source scans.
6. Post-proof API review found root `TransactionPlan` / `TransactionCheckpoint` raw Serde deserialization could bypass their validation convenience methods. Both root types now deserialize through private validated wire types. Direct-Serde regressions prove invalid plans/checkpoints cannot be obtained, while `from_json_str()` still preserves Neo's specific `TransactionError` taxonomy.
7. The first validated-deserialization pre-proof correctly rejected an implementation that wrapped convenience-parser validation errors as generic Serde errors. The parsing path was corrected to validate private wire types directly; the helper then passed Phase 4 review, workspace check, Clippy `-D warnings`, and all `neo-transaction` tests before committing.

## Required proof before merge

- Phase 1, Phase 2, Phase 3, and Phase 4 20-lane static reviews;
- committed/current `Cargo.lock`;
- `rustfmt`;
- locked workspace type/build proof;
- Clippy with warnings denied;
- unit tests;
- catalogue and matcher CLI fixtures;
- transaction-plan CLI fixture;
- generated Planned checkpoint plus checkpoint-validation fixture proof;
- Windows and Ubuntu CI;
- external PR review where available and recursive correction of valid findings;
- final documentation-state CI.

Phase 4 does **not** prove live machine mutation. Machine-changing executors remain blocked.

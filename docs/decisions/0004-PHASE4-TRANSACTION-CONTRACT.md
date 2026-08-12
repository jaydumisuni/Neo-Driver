# Decision 0004 — Phase 4 Transaction / Rollback Contract

**Status:** Accepted for Phase 4 implementation  
**Scope:** transaction state, checkpoints, verification, rollback evidence; no real machine mutator.

## Decision

Neo Phase 4 proves the transaction safety model before any driver/runtime/tweak/debloat/security executor is attached.

1. A transaction plan is exact and immutable for the lifetime of its authorization.
2. The exact serialized plan is SHA-256 fingerprinted. Authorization and reboot checkpoints carry the matching fingerprint.
3. Before authorization, declared snapshot targets are captured from the actual machine state. Reversible actions cannot authorize when a required rollback baseline is unavailable.
4. A state target cannot be snapshot-owned by more than one transaction action; overlapping ownership is rejected because rollback ordering would be ambiguous.
5. Authorization approves exactly every action in the transaction plan. Lower-confidence actions, HIGH/EXPERT risk, and irreversible actions require explicit additional acknowledgement.
6. A `REJECTED` action cannot enter a transaction. Manual authority cannot convert rejected evidence into permission.
7. Apply success or installer exit code is never completion proof. The transaction enters verification after apply/resume.
8. Verification results contain predicates plus observed evidence; PASS/FAIL is recomputed from that evidence and the captured baseline instead of being trusted from persisted JSON.
9. Reboot/resume is restart-safe at the contract level. A reboot checkpoint carries the transaction ID, exact plan fingerprint, expected post-reboot predicates, restoration obligations, and resume stage.
10. After reboot, Neo re-probes and proves expected state before continuing. Failure or unknown state blocks continuation.
11. Rollback restores captured pre-state, not a generic presumed default. Rollback verification uses `MatchesBaseline` predicates.
12. `ROLLED_BACK` requires successful rollback application records and successful rollback verification.
13. Checkpoint validation enforces stage invariants, exact plan fingerprint, exact proof coverage, and ordered event history.
14. Phase 4 CLI operations are read-only: validate a plan, emit a Planned checkpoint template, or validate a checkpoint. They cannot authorize, apply, roll back, reboot, or change Windows state.

## Deliberate Phase 4 limitation

The crate records future executor outcomes but contains no executor that performs those machine changes. Controlled driver installation remains a later phase. If a recovery path cannot be represented safely by this contract, it remains blocked rather than being approximated.

## Frozen-plan basis

This decision implements the master-plan laws and sections covering:

- capture actual state;
- verify after change;
- fail closed on uncertainty;
- security-state reboot/resume workflow;
- persistent restart-safe missions;
- transaction / rollback engine;
- verification rules;
- rollback to actual captured state.

# Neo Driver — Engineering Execution Doctrine

This document operationalizes the frozen master plan during implementation.

## Source of truth

`docs/NEO_DRIVER_MASTER_PLAN.md` remains the product/architecture authority. This document does not change product scope.

## THETECHGUY cycle

Every implementation task follows:

1. Recover evidence.
2. Understand.
3. Build.
4. Review.
5. Freeze.
6. Prove.
7. Submit / ship.

Tests are proof. They are not a substitute for recovering evidence and reasoning about correctness first.

## Sergeant 10-for-2 / 20-private minimum

For every substantial task, Neo work uses the Sergeant doctrine:

- estimate the ordinary human-equivalent worker requirement;
- deploy tenfold machine-scale evidence coverage;
- work equivalent to two ordinary workers receives at least 20 distinct private/evidence lanes;
- lanes must be bounded and non-duplicative;
- findings are reconciled by the responsible review role before PASS;
- a discovered child task receives the same doctrine recursively;
- speed never weakens evidence, authority, rollback, or proof gates.

The 20 lanes can be implementation obligations, static review obligations, compatibility cases, negative cases, security checks, documentation checks, or deterministic proof checks. They do not mean writing twenty copies of the same code.

## No skipped findings

A task cannot become DONE while a named material gap is unresolved.

Allowed outcomes for a finding:

- confirmed and fixed;
- rejected with evidence;
- narrowed and fixed;
- proven not applicable;
- explicitly BLOCKED with the parent task also blocked.

`deferred`, `not checked`, and `probably fine` are not PASS states.

## Phase-1 mutation boundary

The initial Neo implementation is read-only. No driver install, Driver Store mutation, registry mutation, AppX removal, Windows feature change, service change, BCD change, reboot scheduling, tweak, debloat, or runtime installation is permitted until the transaction/authority/rollback/verification contracts are implemented and reviewed.

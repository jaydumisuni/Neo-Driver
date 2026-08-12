# Neo Driver

Neo Driver is a model-free Windows setup, driver, runtime, gaming, technician, debloat, tweak, repair, and recovery suite.

> **Status:** Implementation started. Phase 1 is the shared Rust core/CLI contract plus a read-only Windows System X-Ray foundation. Machine mutation is intentionally disabled in this phase.

## Source of truth

The canonical product and architecture plan is [`docs/NEO_DRIVER_MASTER_PLAN.md`](docs/NEO_DRIVER_MASTER_PLAN.md).

Implementation must not drift from the master plan silently. Material scope, architecture, safety, UX-authority, package-policy, or execution-doctrine changes require an explicit recorded decision before implementation continues.

Current implementation status is tracked in [`docs/IMPLEMENTATION_STATUS.md`](docs/IMPLEMENTATION_STATUS.md).

## Current workspace

- `neo-core` — shared model-free evidence, mission, authority, risk, and verification contracts.
- `neo-probe` — read-only Windows evidence collection foundation.
- `neo-cli` — terminal surface backed by the same core contracts intended for the future GUI.
- `tools/phase1_static_review.py` — reproducible 20-lane Phase 1 engineering review.

## Current CLI surface

```text
neo scan [--json]
neo plan <intent> [--depth beginner|standard|expert] [--json]
neo status
```

`neo scan` is read-only. Driver installs, runtime installs, debloat, tweaks, BCD changes, and other machine mutation remain blocked until Neo's transaction, authority, rollback, and verification layers are implemented and reviewed.

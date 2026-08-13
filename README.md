# Neo Driver

Neo Driver is a model-free Windows setup, driver, runtime, gaming, technician, debloat, tweak, repair, and recovery suite.

> **Status:** Implementation active. Phases 1–5 are proven and merged. Phase 6 is building the deterministic Runtimes & Gaming assessment layer. The controlled Windows driver mutation backend remains internal pending live attached-device proof; runtime installation remains blocked at this gate.

## Source of truth

The canonical product and architecture plan is [`docs/NEO_DRIVER_MASTER_PLAN.md`](docs/NEO_DRIVER_MASTER_PLAN.md).

Implementation must not drift from the master plan silently. Material scope, architecture, safety, UX-authority, package-policy, or execution-doctrine changes require an explicit recorded decision before implementation continues.

Current implementation status is tracked in [`docs/IMPLEMENTATION_STATUS.md`](docs/IMPLEMENTATION_STATUS.md).

## Current workspace

- `neo-core` — shared model-free evidence, mission, authority, risk, and verification contracts.
- `neo-probe` — read-only Windows evidence collection foundation.
- `neo-device` — normalized ordered device/driver/USB-stack evidence contracts.
- `neo-catalogue` — package provenance, applicability, signature, dependency/conflict, security, and reboot contracts.
- `neo-match` — deterministic read-only driver candidate matching/ranking.
- `neo-transaction` — transaction, checkpoint, reboot/resume, verification, and rollback contracts.
- `neo-driverstore` — controlled Windows selected-driver installation backend, kept internal pending live-device proof.
- `neo-runtime` — deterministic runtime/gaming evidence assessment, profile readiness, package binding, and reviewable action planning.
- `neo-cli` — terminal surface backed by the same proven core contracts intended for the future GUI.
- `tools/phase1_static_review.py` through `tools/phase6_static_review.py` — reproducible 20-lane engineering reviews.

## Current CLI surface

```text
neo scan [--json]
neo plan <intent> [--depth beginner|standard|expert] [--json]
neo catalogue validate <file> [--json]
neo match --device <file> --catalogue <file> --architecture <arch> --build <n> [--json]
neo runtimes --evidence <file> --catalogue <file> --policy <file> [--profile fresh-windows|gaming|technician|developer] [--json]
neo gaming --evidence <file> --catalogue <file> --policy <file> [--json]
neo transaction validate-plan <file> [--json]
neo transaction checkpoint-template <file>
neo transaction validate-checkpoint <file> [--json]
neo status
```

The Phase 6 runtime/gaming commands are read-only. They may produce reviewable planned actions, including deselectable profile baselines, but cannot download/install runtimes, mutate Windows features, reboot, or advance a transaction. Driver mutation remains internal until live attached-device proof is complete.

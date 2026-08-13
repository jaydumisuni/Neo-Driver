# Decision 0009 — Phase 9 State Assessment Foundation

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Product domain:** first bounded child of the frozen Tweaks domain  
**Authority:** read-only assessment only

## Decision

Repository Phase 9 begins the master-plan Tweaks domain with a generic state-assessment primitive. Repository numbering differs from the original roadmap because the Runtime/Gaming domain required additional bounded child stages before work moved to Tweaks.

Phase 9 adds `neo-state-plan`. It validates curated intent metadata and supplied current-state evidence, then reports whether each explicitly selected item already matches its desired state.

The crate does not resolve its opaque state keys into operating-system actions. It creates no machine-changing authority.

## Model

Each definition records a stable ID, title/category, benefit/trade-off, risk, recommendation/verdict, default-selection metadata, administrator/reboot metadata, one opaque state key, one typed desired state, and warnings.

Supported values are text, unsigned 32-bit, and unsigned 64-bit values. Desired state is represented as a typed value or absence. Keys are case-insensitive identities within this phase.

Direct Serde construction re-runs validation. Duplicate IDs and duplicate keys fail closed. High-risk entries cannot be preselected. Preselected entries must be Certified and cannot carry unsafe recommendation states.

## Evidence

Evidence contains one supplied observation per state key: typed value present, absent, or unavailable with a reason.

Duplicate observation keys fail closed. Missing or unavailable evidence blocks assessment. Selection is explicit; duplicate, unknown, or Rejected selections fail closed.

The result exposes current state, desired state, and whether the item is already satisfied.

## Proof surface

`neo-state-assess` is an internal read-only proof binary in the existing CLI package. The product `neo` command remains unchanged in this phase.

All operating-system-specific resolution, transaction binding, and machine-changing execution remain outside Phase 9.

## Proof requirement

Phase 9 must preserve all Phase 1–8 gates and additionally prove workspace compiler/Clippy, the complete `neo-state-plan` regression suite, direct-Serde validation, duplicate rejection, explicit-selection enforcement, unsafe-preselection rejection, missing/unavailable/rejected-selection failure, satisfied-versus-different reporting, proof-binary compilation, and source review showing no execution backend.

No green run may be described as changing machine state. Phase 9 proves assessment only.

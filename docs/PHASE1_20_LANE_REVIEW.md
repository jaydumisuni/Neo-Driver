# Phase 1 — 20-Lane Review Record

Task: establish Neo's shared model-free Rust contracts, CLI boundary, and first read-only Windows evidence probes.

The executable review is `tools/phase1_static_review.py`. CI runs it on Windows and Linux before Rust build/test proof.

## Lanes

1. **Architecture** — GUI/CLI direction shares reusable core/probe crates.
2. **Model-free** — no LLM/cloud/model dependency in the Phase 1 runtime.
3. **Manual authority** — mutating actions require confirmation by invariant.
4. **High-risk defaults** — HIGH/EXPERT actions cannot be preselected.
5. **Mutation evidence** — mutating actions require evidence and rationale.
6. **Certification** — only CERTIFIED actions may be default-selected.
7. **Mission identity** — duplicate action IDs fail validation.
8. **Beginner** — Beginner depth exists without losing authority.
9. **Standard/Expert** — deeper views share the same safety contracts.
10. **Intent** — frozen first-launch intents are represented in core/CLI.
11. **Read-only surface** — Phase 1 contains no PnP/BCD/registry mutation operation.
12. **Windows identity** — OS identity uses bounded registry observation.
13. **Security state** — Test Signing, nointegritychecks, Secure Boot, and HVCI are separate evidence.
14. **Reboot state** — multiple read-only reboot indicators are observed.
15. **Connected devices** — connected-device evidence is collected.
16. **Problem devices** — problem-device evidence is collected separately.
17. **Driver Store** — third-party Driver Store evidence is enumeration-only.
18. **Failure honesty** — one command-start failure is preserved and does not cancel independent lanes.
19. **Platform boundary** — non-Windows scan returns an explicit unsupported-platform error.
20. **Proof/anti-drift** — fixtures, execution doctrine, and the canonical master-plan pointer are present.

## Findings corrected before push

### F1 — Registry path escaping

An early draft used an incorrect raw-string path form for a Windows registry query. Corrected before repository push.

### F2 — Probe cascade failure

Early probe calls propagated a command-start failure with `?`, which could prevent later independent evidence lanes from running. Corrected: command-start failure is now retained as `CommandEvidence` and the remaining read-only lanes continue.

### F3 — Provisional default selection

The first plan contract blocked `INVESTIGATE`/`REJECTED` default selection but still allowed `PROVISIONAL`. Corrected: only `CERTIFIED` may be preselected.

### F4 — Duplicate action IDs

The initial mission validator did not reject duplicate action IDs. Corrected with a deterministic uniqueness gate.

### F5 — Mutation without evidence/rationale

The initial contract required confirmation but did not require a mutation rationale and supporting evidence. Corrected: both are mandatory for mutating actions.

### F6 — Native architecture

The initial probe relied only on `PROCESSOR_ARCHITECTURE`, which can describe a 32-bit process on 64-bit Windows. Corrected: Neo first checks `PROCESSOR_ARCHITEW6432`, then falls back to `PROCESSOR_ARCHITECTURE`.

### F7 — TESTSIGNING default interpretation

A successful BCD enumeration may omit TESTSIGNING because the option is not set by default. Corrected: after a successful `bcdedit /enum {current}`, omitted `testsigning` and `nointegritychecks` are treated as persistent OFF; if BCD enumeration fails, the state stays unknown.

### F8 — Normal reboot-indicator absence looked like failure

Registry keys such as `RebootPending` are expected to be absent on healthy systems. Corrected: exit code 1 with a successfully started `reg.exe` query is represented as `absent`, not as a warning/failure for these presence probes.

### F9 — Unapproved licensing assumption

The initial local Cargo scaffold declared MIT even though no Neo license had been approved. Corrected before push: licensing metadata is omitted until explicitly decided.

## Reconciliation

After corrections, the deterministic Phase 1 static review reports **20/20 PASS**.

Compilation/runtime proof must still come from Rust-enabled CI; local runtime proof is not claimed because this workspace does not contain a Rust toolchain and is not Windows.

# Decision 0005 — Phase 5 Controlled Driver Installation

## Status

Accepted for Phase 5 implementation proof.

## Context

Phase 4 proved Neo's transaction, authority, checkpoint, reboot/resume, verification, and rollback contracts without attaching a machine-changing executor. Phase 5 may attach the first Windows driver mutator only if that mutator preserves those laws and does not manufacture compatibility or rollback certainty.

Windows driver installation introduces two independent machine-state surfaces:

1. the active driver binding of each impacted device; and
2. Driver Store package presence and Windows-published package identity.

Both are part of the transaction baseline and both must be restored when Neo introduced a change.

## Decision

### 1. Exact selected source before authority

Neo plans an installation only for a catalogue `InfDriverBundle` artifact whose catalogue signature state is `Verified`.

Before authority Neo must:

- canonicalize the selected INF under the approved package root;
- hash the exact source INF with SHA-256;
- ask Windows to verify the actual INF/catalogue signature;
- identify the exact present-device compatibility set for that INF;
- require that Windows' exact-INF impact set equals Neo's catalogue/matcher impact set;
- capture every impacted device's current binding/problem state; and
- resolve every active published INF to an exact baseline Driver Store package.

A missing or ambiguous baseline package blocks reversible authority.

### 2. Preflight repeats the evidence immediately before mutation

After user authority but before the first write, Neo re-proves:

- source INF bytes;
- Windows signature/catalogue evidence;
- present-device impact set;
- active bindings;
- captured baseline packages; and
- target package Driver Store baseline.

Any drift blocks apply before mutation.

### 3. Forward path preserves Windows best-match authority per authorized device

Neo stages the exact approved package and records its Windows-published OEM INF/package identity.

For each already-authorized device instance ID, the forward backend asks Windows to install that device's best preinstalled match without supplying a specific driver node. Neo does not expose or use a force-lower-ranked forward path.

This per-device route is deliberate: a device that appears after authority is not implicitly added to the transaction blast radius.

### 4. API success is not completion proof

A backend result and an observed machine change are separate facts.

After mutation Neo re-reads device/Driver Store state. Completion requires deterministic policy proof. A healthy Windows-selected no-op may complete with zero net mutation. If a newly staged target package is unused and was absent at baseline, Neo removes that package so the Driver Store returns to its captured state.

If post-write observation becomes unavailable, Neo records the outcome conservatively as changed and enters recovery rather than leaving the transaction in an unclassified `Applying` state.

### 5. Runtime reboot is evidence, not a planning guess

Install or rollback backends may discover at runtime that a reboot is required. That evidence escalates the transaction into the corresponding persistent reboot checkpoint. Post-reboot continuation requires re-probe and verification.

### 6. Rollback restores captured reality

Neo does not rely on Windows' optional single-driver rollback backup as the rollback guarantee.

For each changed device, rollback may use specific-device driver installation only to restore the exact captured baseline published INF/package. This specific-driver primitive is rollback-only; it is not a normal forward recommendation or force-install feature.

If Neo introduced the target package, it removes only that exact published OEM package, only after no device remains bound to it, and without force deletion. Rollback is not complete until captured binding and Driver Store baselines are re-proven.

### 7. Mutation surface remains internal in Phase 5

Phase 5 proves the Windows mutation engine and its transaction integration, but does not expose a CLI write command. The existing CLI remains inspection/read-only for transaction contracts.

A public/technician mutation surface requires live attached-device proof and a later authority decision. This keeps CI from being mistaken for real hardware mutation proof.

## Explicit exclusions

Phase 5 does not authorize:

- forced lower-ranked driver binding;
- force deletion of Driver Store packages;
- blanket USB/filter replacement;
- broad stale-package cleanup;
- BCD/security weakening;
- driver downloads or runtime installation;
- Apple/DFU Pro binding changes; or
- any mutation outside the exact transaction impact set.

## Proof requirement

Merge requires:

- Phase 1–5 static gates;
- tracked/current Cargo.lock;
- rustfmt;
- locked workspace build;
- Clippy with warnings denied;
- all unit/adversarial regressions on Ubuntu and Windows;
- Windows compilation of the real SetupAPI/NewDev backend;
- external review disposition with no unresolved correctness/security thread; and
- a final documentation-state CI run on the frozen PR head.

Live attached-device mutation proof remains explicitly unclaimed.

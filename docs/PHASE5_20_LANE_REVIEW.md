# Phase 5 — 20-Lane Engineering Review

Phase 5 is the first Neo Driver phase containing a real Windows mutation backend. The gate is therefore stricter than a compiler check: authority must bind to exact bytes, exact Windows compatibility evidence, exact captured pre-state, exact rollback packages, and deterministic post-write proof.

The executable gate is `tools/phase5_static_review.py`. It must pass on both Ubuntu and Windows in normal CI before Phase 5 can merge.

| Lane | Contract |
|---|---|
| 01 | `neo-driverstore` is a workspace member and Win32 bindings are Windows-only. |
| 02 | Driver plans and persisted sessions validate at root deserialization and remain fingerprint-bound. |
| 03 | Catalogue `Verified` state is re-proven against the actual INF via Windows signature/catalogue evidence. |
| 04 | Authority binds to canonical in-root source INF bytes and apply rechecks the SHA-256. |
| 05 | Windows compatibility is queried against the exact INF for present devices. |
| 06 | Windows exact-INF impact must exactly equal Neo catalogue/matcher impact. |
| 07 | Every impacted device must have an exact active binding and resolvable baseline Driver Store package before authority. |
| 08 | The driver session reconstructs and validates the generic transaction and baseline contracts. |
| 09 | Source, signature, impact set, bindings, baseline packages, and target-store baseline are re-proven immediately before mutation. |
| 10 | Staging captures and round-trips the exact Windows OEM published/package identity. |
| 11 | Forward mutation is per authorized device; Windows chooses that device's best preinstalled match and Neo supplies no driver node. |
| 12 | No force-install or force-delete primitive exists; target package removal is non-force. |
| 13 | Binding changes outside the authorized impact set fail through a typed blast-radius error. |
| 14 | Backend/API success is not completion proof; net mutation and deterministic postconditions are separate evidence. |
| 15 | A healthy best-match no-op restores an originally absent target package state instead of leaving an unused staged package. |
| 16 | Post-write observation uncertainty is conservatively recorded as changed and routes recovery. |
| 17 | Runtime reboot evidence is persisted and the post-reboot policy must be re-proven. |
| 18 | Specific-driver installation exists only in rollback and restores the exact captured baseline package. |
| 19 | Rollback reboot and Driver Store cleanup remain persistent and require exact baseline verification. |
| 20 | The mutation engine remains internal in Phase 5; no CLI write surface is exposed before live attached-device proof, and adversarial/Windows validation regressions remain present. |

## Deliberate boundary

Phase 5 proves the controlled selected-driver mutation engine, not broad Driver Store administration. It does not authorize forced lower-ranked binding, force package deletion, blanket USB/filter replacement, security/BCD weakening, runtime installs, downloads, or technician binding changes outside an explicit later authority path.

The Windows backend is compiler- and regression-proven in CI, but CI does not execute machine-changing calls. Live attached-device mutation proof is therefore not claimed by this phase.

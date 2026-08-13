# Neo Driver — Phase 6 20-Lane Review

**Phase:** Runtimes & Gaming assessment foundation  
**Status:** FROZEN FOR PROOF

This review is the deterministic Sergeant-style 20-lane gate for Phase 6. The executable mirror is `tools/phase6_static_review.py`.

1. **Workspace contract** — `neo-runtime` is a first-class workspace crate.
2. **Shared truth contract** — runtime planning reuses `neo-core` and `neo-catalogue`; it does not create a second package truth source.
3. **Model-free/read-only boundary** — no downloader, process executor, installer or shell execution exists in the runtime engine.
4. **Runtime coverage** — VC++ 2015+, DirectX June 2010, .NET, Python, WebView2, XNA, OpenAL, PhysX/Legacy and DirectPlay are typed components.
5. **Normalized state** — Installed/Missing/Broken/Partial/Unknown are distinct evidence states.
6. **Inventory validation** — invalid build, architecture, duplicate evidence and source-less evidence fail closed.
7. **Unknown fails closed** — unknown evidence never becomes install authority.
8. **Typed package binding** — component bindings must resolve to existing `PackageKind::Runtime` packages.
9. **OS applicability gates** — architecture/build limits are hard gates before candidate use.
10. **Ambiguity fails closed** — multiple compatible packages are not silently ranked or guessed.
11. **Manual authority** — every runtime recommendation is individually user-selectable.
12. **Baseline confirmation** — baseline preselection remains explicit-confirmation gated.
13. **Optional selection law** — optional components are never preselected.
14. **DirectX policy** — June 2010 legacy runtimes are a deselectable Fresh Windows/Gaming baseline recommendation.
15. **VC++ policy** — modern baseline begins at Visual C++ 2015+ x86/x64 rather than manufacturing yearly duplicates.
16. **Python policy** — Technician/Developer Python remains optional; partial/broken state is representable for repair-first logic.
17. **Gaming optionals** — XNA/OpenAL/PhysX/PhysX Legacy/.NET 3.5/DirectPlay remain explicit optional components absent dependency proof.
18. **No fake rollback** — assessment does not claim runtime rollback before a runtime executor contract exists.
19. **CLI boundary** — `neo runtimes` and `neo gaming` remain read-only and report `Machine changes: none`.
20. **Proof integration** — fixtures and the normal Ubuntu/Windows CI pipeline exercise the Phase 6 static gate and both CLI paths.

## Freeze disposition

The implementation is acceptable for proof only if all 20 lanes pass together with the inherited Phase 1–5 gates, compiler, Clippy, unit tests, lock integrity and CLI fixtures.

No runtime installation, Windows-feature mutation, download, reboot, transaction advancement or GUI write action is authorized by this freeze.

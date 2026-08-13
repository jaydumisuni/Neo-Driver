# Neo Driver — Phase 6 20-Lane Review

**Phase:** Runtimes & Gaming System X-Ray foundation  
**Status:** FROZEN FOR PROOF

This review is the deterministic Sergeant-style 20-lane gate for Phase 6. The executable mirror is `tools/phase6_static_review.py`.

1. **Workspace contract** — `neo-runtime`, `neo-directx-legacy`, and `neo-runtime-probe` are first-class workspace crates.
2. **Shared boundary contract** — assessment reuses `neo-core`/`neo-catalogue`; System X-Ray reuses `neo-probe` command evidence rather than creating another execution layer; DirectX evidence is consumed through the compiled detector crate.
3. **Model-free assessment** — the pure runtime engine contains no downloader, process executor, installer or shell mutation path.
4. **Bounded read-only scanner** — runtime probing is limited to registry queries, `dotnet --list-runtimes`, DISM `/Get-FeatureInfo`, `py -0p`, `where.exe`, and a read-only DirectX filesystem predicate; known mutation commands are rejected by the static gate.
5. **Runtime coverage** — VC++ 2015+, DirectX June 2010, .NET, Python, WebView2, XNA, OpenAL, PhysX/Legacy and DirectPlay are typed components.
6. **Normalized evidence validation** — Installed/Missing/Broken/Partial/Unknown are distinct states; invalid build, architecture, duplicate evidence and source-less evidence fail closed.
7. **Unknown fails closed** — unknown evidence never becomes install authority.
8. **Typed package binding** — component bindings must resolve to existing `PackageKind::Runtime` packages.
9. **Applicability and ambiguity** — architecture/build limits are hard gates and multiple compatible packages are not silently ranked or guessed.
10. **Manual authority** — every runtime recommendation is individually user-selectable; baseline preselection remains confirmation-gated and optionals remain off by default.
11. **Profile law** — VC++ 2015+ x86/x64 is the modern baseline, DirectX June 2010 is a deselectable Fresh Windows/Gaming baseline, and Python remains optional.
12. **Gaming optionals** — XNA/OpenAL/PhysX/PhysX Legacy/.NET 3.5/DirectPlay remain explicit optional components absent dependency proof.
13. **WebView2 documented predicate** — the Microsoft WebView2 GUID, architecture-aware EdgeUpdate HKLM path, HKCU path and non-zero `pv` test are frozen.
14. **Microsoft runtime predicates** — VC++ v14 registry evidence, .NET Framework 4 `Release`, modern .NET/Desktop runtime listing, NetFx3 and DirectPlay DISM feature queries are explicit.
15. **Python repair-first detection** — probing never launches a bare interpreter; PATH gaps are `Partial`, while lack of global command evidence stays `Unknown` rather than becoming a second-install trigger.
16. **DirectX legacy completeness / remaining unknowns** — the compiled DirectX detector uses trusted `GetWindowsDirectoryW` and Microsoft's documented D3DCompiler/D3DCSX/D3DX/X3DAudio/XACT/XAPOFX/XAudio/XInput filename ranges; XNA/OpenAL/PhysX predicates remain `Unknown` until independently proven.
17. **Raw evidence retention** — live System X-Ray preserves `CommandEvidence` and resolves host build/architecture through the base Neo machine probe; DirectX reports preserve expected/present/missing evidence separately.
18. **No fake rollback/runtime mutation** — assessment does not claim runtime rollback and the scanner contains no feature/runtime mutation command. DirectX completeness is presence evidence, not binary-health certification.
19. **CLI read-only surface** — `neo runtime-scan`, `neo runtimes`, and `neo gaming` report evidence/plans while preserving `Machine changes: none`.
20. **Live and fixture proof integration** — normal CI must execute Windows-only `neo runtime-scan --json` plus deterministic runtime and gaming fixtures, alongside all inherited Phase 1–5 gates.

## Freeze disposition

The implementation is acceptable for proof only if all 20 lanes pass together with the inherited Phase 1–5 gates, lock integrity, rustfmt, compiler/type proof, Clippy, unit tests, Windows live runtime scan, and CLI fixtures.

No runtime installation, Windows-feature mutation, download, reboot, transaction advancement or GUI write action is authorized by this freeze.

A green Phase 6 System X-Ray foundation does not by itself claim runtime installer execution or broader Gaming hardware/API readiness. XNA/OpenAL/PhysX detection remains explicitly unresolved rather than guessed.

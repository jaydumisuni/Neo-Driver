# Phase 13 — Debloat Assessment 20-Lane Review

**Status:** FROZEN REVIEW CONTRACT  
**Scope:** read-only AppX/debloat catalogue + evidence + selection/profile assessment  
**Mutation authority:** none

This review is the deterministic Phase 13 acceptance formation. A lane is PASS only when the production source and named regression/proof surface establish the condition. The final CI result is recorded only after the frozen implementation head runs.

Independent pre-proof source review closed two findings. First, manual selection can no longer turn higher-risk, non-certified, unknown, rejected, unsupported, conflicting, or do-not-touch catalogue state into a normal removal candidate, and protected/manual-only classification takes precedence over profile-preservation messaging. Second, every feature-dependent, dependency-sensitive, or protected/manual-only definition now requires at least one non-empty consequence/dependency note; a non-safe classification cannot exist without the evidence text that justifies review or protection.

1. **Workspace isolation** — `neo-debloat` is a first-class workspace member.
2. **Platform neutrality** — Phase 13 has no Windows dependency or Windows API authority.
3. **Transaction isolation** — Phase 13 does not depend on `neo-transaction`.
4. **Executor isolation** — Phase 13 does not depend on tweak, runtime, or driver mutation executors.
5. **No production command execution** — no `std::process::Command`, PowerShell, `cmd.exe`, DISM, AppX command, Winget, or package-manager invocation in production Phase 13 source.
6. **Frozen class/consequence model** — SAFE OPTIONAL, FEATURE DEPENDENT, DEPENDENCY SENSITIVE, and PROTECTED/MANUAL ONLY are distinct typed classes; every non-safe class requires at least one non-empty side-effect/dependency note.
7. **Installed/provisioned separation** — current-user installed state and provisioned-image state are captured independently.
8. **Restore-route model** — Store, provisioned-image, vendor-source, and none are distinct metadata states; declared restore is not claimed as proven rollback.
9. **Catalogue validation on deserialize** — Serde cannot bypass `DebloatCatalogue` validation.
10. **Evidence validation on deserialize** — Serde cannot bypass `DebloatEvidence` validation.
11. **Identity uniqueness** — duplicate Neo IDs and case-insensitive duplicate AppX package identities/observations fail closed.
12. **Default class/risk gate** — only SAFE OPTIONAL + LOW can be preselected.
13. **Default evidence/recommendation gate** — defaults require CERTIFIED evidence and an explicitly removal-appropriate recommendation.
14. **Default restore gate** — no default-selected item may lack a declared restore route.
15. **Profile preservation** — profile preservation removes defaults and blocks a selected present package; Custom has no hidden defaults.
16. **Explicit selection** — empty, duplicate, or unknown selection fails closed.
17. **Observation completeness** — selected packages require captured installed and provisioned evidence; missing/unavailable evidence fails closed.
18. **Candidate policy** — RemovalCandidate requires SAFE OPTIONAL + LOW + CERTIFIED + removal-appropriate recommendation + restore route and no profile block.
19. **Protected/policy block** — protected/manual-only and rejected/unsupported/conflicting/do-not-touch states cannot become normal candidates.
20. **Behavioral non-mutation proof** — the engineering binary reports `Machine changes: none`; the regression proves the fixture directory is unchanged byte-for-byte; normal CI runs this lane beside all inherited Phase 1–12 proof.

## Deliberate non-claims

Phase 13 does not claim real Windows AppX discovery, AppX/provisioning removal, restoration, transaction-bound rollback, public product debloat mutation, MCP/RPC debloat capability issuance, or real Microsoft/OEM package safety classification.

The synthetic `Contoso.*` fixtures prove the contract without turning donor metadata into unsupported safety knowledge.

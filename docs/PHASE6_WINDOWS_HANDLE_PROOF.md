# Phase 6 — Windows Handle-Lifetime and Final Implementation Proof

## P6-F11 — Retained capability handles blocked Windows staging cleanup

The capability-held filesystem rewrite passed Ubuntu and Windows type/build + Clippy gates, but the first authoritative Windows unit run exposed Win32 sharing error 32 during vault staging cleanup. The security model was correct; Windows refused deletion because retained handles were still open.

Three handle lifetimes were corrected explicitly:

1. the cleanup validation `session_dir` is dropped before `staging.remove_dir_all(...)`;
2. the staged payload reader is dropped before `payload.pack` can be removed;
3. the import's retained staging-session directory handle is dropped before `cleanup_staging(...)` opens/validates/removes the session directory.

A branch-only Windows proof applied exactly those three changes, then passed:

- Phase 6 structural review;
- Windows full-workspace `cargo check --locked`;
- Windows Clippy with warnings denied;
- all `neo-vault` unit and integration regressions.

Only after those gates passed did it commit the correction as `5cd91718eaace96cc373899748c62c0eff3e1e90`; the write-enabled proof helper deleted itself in that same commit.

This correction changes handle lifetime only. It does not weaken the retained no-follow directory-capability model from P6-F9, nor does it add any public write/download/install command.

## Phase 6 implementation-code proof

Normal user-authored CI run `31686242470` passed the complete configured release pipeline on both Ubuntu and Windows after P6-F11:

- Phase 1 through Phase 6 twenty-lane static reviews;
- Cargo lock integrity;
- rustfmt;
- locked full-workspace type/build proof;
- Clippy with warnings denied;
- all workspace unit/integration tests, including the capability/symlink/concurrent-import vault regressions;
- catalogue CLI fixture;
- matcher CLI fixture;
- transaction plan CLI fixture;
- transaction checkpoint CLI fixture;
- vault source-map CLI fixture.

## Review and surface freeze

PR #7 currently has zero unresolved CodeRabbit review threads. The major TOCTOU filesystem-security finding is resolved by the retained no-follow directory-capability implementation. The frozen PR surface contains 19 intended files and no temporary proof/helper workflow.

This file is the final documentation-only freeze record. No executable source is changed by this update. The resulting documentation-state head must pass the same Ubuntu + Windows CI pipeline before PR #7 may merge.

# Phase 6 — Windows Handle-Lifetime Proof

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

The complete Phase 6 branch must still pass the normal Ubuntu + Windows CI pipeline from a user-authored clean head before merge.

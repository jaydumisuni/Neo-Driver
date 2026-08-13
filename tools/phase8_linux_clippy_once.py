#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(path: str, old: str, new: str) -> None:
    p = ROOT / path
    text = p.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one anchor, found {count}: {old!r}")
    p.write_text(text.replace(old, new, 1), encoding="utf-8")


replace_once(
    "crates/neo-runtime-executor/src/executor.rs",
    "use crate::model::canonical_arch;\n",
    "#[cfg(any(windows, test))]\nuse crate::model::canonical_arch;\n",
)
replace_once(
    "crates/neo-runtime-executor/src/model.rs",
    "use neo_catalogue::{\n    PackageKind, RuntimeExecutionSpec, RuntimeInstallerKind, RuntimeVerificationRule,\n    SecurityRequirements,\n};\n",
    "use neo_catalogue::{\n    PackageKind, RuntimeExecutionSpec, RuntimeVerificationRule, SecurityRequirements,\n};\n#[cfg(any(windows, test))]\nuse neo_catalogue::RuntimeInstallerKind;\n",
)
replace_once(
    "crates/neo-runtime-executor/src/model.rs",
    "    pub(crate) fn started_without_exit(detail: impl Into<String>) -> Self {\n",
    "    #[cfg(windows)]\n    pub(crate) fn started_without_exit(detail: impl Into<String>) -> Self {\n",
)

print("Phase 8 Linux cfg hygiene corrections applied")

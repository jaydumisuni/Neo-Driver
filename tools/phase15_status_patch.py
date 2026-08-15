#!/usr/bin/env python3
from pathlib import Path

path = Path("docs/IMPLEMENTATION_STATUS.md")
text = path.read_text(encoding="utf-8")

bullet = "- **Phase 15:** merged and engineering-proven — read-only exact AppX identity, rollback-readiness, and Phase 4 transaction-preparation boundary for exactly one Phase 13 current-user removal candidate, using native Windows PackageManager current-user/provisioned inventories, exact package/dependency full/family identities, exact provisioned staged twins for the selected package and every direct dependency, constructor-owned externally read-only prepared state, a BaselineCaptured checkpoint with transaction-plan fingerprint continuity, deterministic Ubuntu/Windows fixture proof, and real non-empty Windows native inventory proof. AppX removal/re-registration execution, deprovision/provision execution, Store/network restore, batch/all-users mutation, public GUI/CLI writes, plugin dependency, and MCP/RPC debloat capability remain blocked."

if bullet not in text:
    lines = text.splitlines()
    marker = next(i for i, line in enumerate(lines) if line.startswith("- **Phase 14:**"))
    lines.insert(marker + 1, bullet)
    text = "\n".join(lines) + "\n"

baseline = "Phase 15 exact implementation head `0bc274218d6c2e86168930cd52a24d25be646070` passed complete independent push run `31908902022` and PR run `31908906703` on Ubuntu and Windows, including the strengthened non-empty real Windows native PackageManager exact-AppX inventory proof and the deterministic transaction-readiness fixture. CodeRabbit completed a full review and identified two valid findings: the engineering CLI's text-output mode was unreachable under its argument-count contract, and the Windows live inventory test could pass vacuously if both native inventories were empty. Both were corrected and their review threads were resolved; a second full CodeRabbit review on the final head could not start because the included-review limit was exhausted, so no second CodeRabbit PASS is claimed. GitGuardian scanned all 17 implementation commits with no secrets detected. Phase 15 merged through PR #30 as `c3489f3e8c043fc66b97810d35aee5a2e3445fc9`, and merged-main run `31909076612` then passed the complete Ubuntu and Windows Phase 1–15 pipeline, including the non-empty real Windows native exact-AppX identity proof."

if baseline not in text:
    marker = "\n## Phase 5 frozen implementation\n"
    if marker not in text:
        raise SystemExit("Phase 5 section marker not found")
    text = text.replace(marker, "\n" + baseline + "\n" + marker, 1)

section = r'''## Phase 15 frozen implementation

Phase 15 closes the evidence gap between Phase 14 logical AppX presence and any future controlled package executor while remaining read-only. It adds `neo-debloat-plan`, which composes the proven Phase 13 candidate law, Phase 14 current-user/provisioned presence evidence, native Windows `PackageManager` exact identity inventory, and the Phase 4 transaction/checkpoint model.

For exactly one selected Phase 13 `CurrentUser` `RemovalCandidate`, Phase 15 captures the exact package Name, FullName, FamilyName, classification flags, and direct dependency identities. It requires the selected package and every direct dependency to have an exact matching provisioned staged identity before deterministic local rollback is considered ready. `RestoreMethod::ProvisionedImage` is the only restore metadata promoted to this prepared rollback route; Store/vendor metadata remains descriptive and cannot manufacture rollback authority.

The prepared action is `ActionKind::Debloat`, uses exact `AppxPackage` state targets, requires explicit confirmation, captures serialized main/dependency identity baselines into a `BaselineCaptured` checkpoint, requires the main current-user package to become absent as the future postcondition, and records reversible rollback obligations for all captured targets. The transaction-plan fingerprint binds targets, postconditions, and rollback obligations; captured baseline payloads remain validated checkpoint state rather than being misrepresented as fingerprint inputs.

`DebloatPreparedTransaction` is constructor-owned and externally read-only. Phase 15 issues no authorization, apply record, capability, removal/re-registration call, deprovision/provision operation, Store/network recovery path, public write command, plugin dependency, or MCP/RPC debloat authority.

The Windows live proof executes native current-user and provisioned PackageManager enumeration, requires both inventories to be non-empty, validates every recovered exact identity, and proves the debloat fixture tree remains byte-for-byte unchanged. Deterministic synthetic fixtures prove transaction readiness on both Ubuntu and Windows without making machine changes.

## Phase 15 review findings closed before freeze

1. The first prepared object exposed replaceable public transaction/checkpoint/restore-route fields. Those fields are now crate-owned with immutable external getters so callers cannot rewrite prepared authority state while retaining the same object/fingerprint surface.
2. Early review text overstated the Phase 4 fingerprint by implying captured baseline payloads were hashed into it. The frozen contract now states precisely that the fingerprint binds the transaction plan, while baseline identities are separately validated checkpoint state.
3. The first Phase 15 static review script did not map one-for-one onto the frozen 20-lane review document. The permanent gate now proves the exact twenty frozen authority/proof lanes directly.
4. CodeRabbit found the engineering `neo-debloat-prepare` text-output path was unreachable because the argument-count guard effectively required the optional `--json` argument. The CLI now accepts exactly six required positional arguments for text output, optionally one exact `--json` flag, rejects other trailing arguments, and has unit regressions for all three cases.
5. CodeRabbit found the Windows live identity test could pass vacuously when current-user and provisioned inventories were empty because `all(...)` over an empty collection is true. The live proof now requires both inventories to be non-empty before identity validation, and Phase 15 static lane 19 binds those assertions explicitly.
'''

if "## Phase 15 frozen implementation" not in text:
    text = text.rstrip() + "\n\n" + section.strip() + "\n"

path.write_text(text.rstrip() + "\n", encoding="utf-8")

final = path.read_text(encoding="utf-8")
required = [
    bullet,
    baseline,
    "## Phase 15 frozen implementation",
    "## Phase 15 review findings closed before freeze",
    "31909076612",
    "c3489f3e8c043fc66b97810d35aee5a2e3445fc9",
]
missing = [value for value in required if value not in final]
if missing:
    raise SystemExit("status patch verification failed: " + repr(missing))

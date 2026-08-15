#!/usr/bin/env python3
from pathlib import Path

path = Path(__file__).resolve().parents[1] / "docs/NEO_DRIVER_MASTER_PLAN.md"
text = path.read_text(encoding="utf-8")
start_marker = "# 25. GUI and CLI are one engine\n"
end_marker = "\n---\n\n# 26. Rust / Python architecture"
start = text.find(start_marker)
end = text.find(end_marker, start)
if start < 0 or end < 0:
    raise SystemExit("canonical section 25 boundary was not found")

replacement = """# 25. MCP/RPC, GUI, and CLI are one engine

Do not implement independent GUI, CLI, Hunter, Oracle, or remote-control business logic.

Neo's machine-changing orchestration is **MCP/RPC-first**. Approved remote/intelligent callers use typed MCP tools which reach the workstation through authenticated typed RPC. The RPC service issues narrow internal capabilities only after Neo's permission, evidence, plan, transaction, confirmation, and risk gates pass.

Architecture:

```text
Hunter / approved MCP callers -> MCP -> authenticated workstation RPC --+
Neo GUI -----------------------------> typed Neo service contracts ------+-> Neo Core / transactions / executors
Neo CLI -----------------------------> diagnostic/manual contracts ------+
```

The GUI and CLI remain presentations over the same core truth. A GUI must not implement a second mutation backend. The CLI remains useful for inspection, planning, validation, fixtures, and explicit technician/manual operation, but it is not Neo's primary mutation control plane and must not bypass capability issuance by exposing raw executors.

Caller identity and permissions used by MCP/RPC are trusted transport/service context, not values a raw client request may self-assert. Every machine-changing RPC remains bound to the same transaction and verification laws as local operation.

No arguments to `NeoDriver.exe` should open the GUI.

Installed/path-accessible CLI should conceptually support commands such as:

- `neo scan`
- `neo status`
- `neo drivers`
- `neo drivers missing`
- `neo runtimes`
- `neo gaming`
- `neo technician`
- `neo repair`
- `neo tweaks`
- `neo debloat`
- `neo devices`
- `neo doctor`

Expert concepts:

- inspect device;
- list/rank driver candidates;
- install/rollback/export driver through approved authority;
- inspect package evidence;
- apply/undo tweak through approved authority;
- plan/apply profile through approved authority.

If destructive CLI operations are added later, they must remain explicit and route through the same typed service/capability/transaction authority rather than constructing privileged executor capabilities directly. Batch approval may exist for bounded automation, but high-risk actions require stronger explicit risk acknowledgement.
"""

path.write_text(text[:start] + replacement + text[end:], encoding="utf-8")

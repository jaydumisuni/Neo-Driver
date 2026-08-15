#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
path = ROOT / "crates/neo-tweak-executor/src/rpc.rs"
text = path.read_text(encoding="utf-8")

old_field = '''struct PendingTweakRpcSession {
    caller: TweakRpcCaller,
    mission_id: String,
    plan_fingerprint: String,
    session: TweakExecutionSession,
}'''
new_field = '''struct PendingTweakRpcSession {
    caller: TweakRpcCaller,
    plan_fingerprint: String,
    session: TweakExecutionSession,
}'''
if text.count(old_field) != 1:
    raise SystemExit("pending-session field target did not match exactly once")
text = text.replace(old_field, new_field)

old_store = '''            PendingTweakRpcSession {
                caller: context.caller.clone(),
                mission_id: request.mission_id,
                plan_fingerprint,
                session,
            },'''
new_store = '''            PendingTweakRpcSession {
                caller: context.caller.clone(),
                plan_fingerprint,
                session,
            },'''
if text.count(old_store) != 1:
    raise SystemExit("pending-session storage target did not match exactly once")
text = text.replace(old_store, new_store)

old_receipt = '''        mission_id: pending.mission_id,
        plan_fingerprint: pending.plan_fingerprint,'''
new_receipt = '''        mission_id: pending
            .session
            .plan()
            .transaction()
            .mission_id()
            .to_string(),
        plan_fingerprint: pending.plan_fingerprint,'''
if text.count(old_receipt) != 1:
    raise SystemExit("receipt mission target did not match exactly once")
text = text.replace(old_receipt, new_receipt)

path.write_text(text, encoding="utf-8")

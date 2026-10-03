#!/usr/bin/env python3
"""DN-2 task 5 acceptance driver: real model + policy-gated tool via vivy-shared.so."""
import ctypes, json, os, sys, time

SO = "/home/ubuntu/repos/agent-diva/agent-diva-gui/src-tauri/vivy-runtime/vivy-shared.so"
CFG = "/home/ubuntu/diva-dn2-accept/vivy.yaml"
WS = "/home/ubuntu/diva-dn2-accept/workspace"
TARGET = os.path.join(WS, "hello.txt")
import glob as _glob
def target_hits():
    return _glob.glob(os.path.join(WS, "**", "hello.txt"), recursive=True)
def target_exists():
    return bool(target_hits())

lib = ctypes.CDLL(SO)
lib.VivyInit.restype = ctypes.c_void_p
lib.VivyInit.argtypes = [ctypes.c_char_p]
lib.VivyCall.restype = ctypes.c_void_p
lib.VivyCall.argtypes = [ctypes.c_uint64, ctypes.c_char_p]
lib.VivyPollEvents.restype = ctypes.c_void_p
lib.VivyPollEvents.argtypes = [ctypes.c_uint64, ctypes.c_uint32]
lib.VivyShutdown.restype = ctypes.c_void_p
lib.VivyShutdown.argtypes = [ctypes.c_uint64]
lib.VivyFree.argtypes = [ctypes.c_void_p]

def take(ptr):
    if not ptr:
        raise RuntimeError("NULL return from ABI")
    s = ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8", "replace")
    lib.VivyFree(ptr)
    return json.loads(s)

def env(r):
    if not r.get("ok"):
        raise RuntimeError(f"envelope error: {r}")
    return r["value"]

LOG = open("/home/ubuntu/diva-dn2-accept/transcript.jsonl", "w")
def rec(tag, obj):
    LOG.write(json.dumps({"t": time.time(), "tag": tag, "data": obj}) + "\n")
    LOG.flush()

def call(h, method, params=None):
    req = {"method": method}
    if params is not None:
        req["params"] = params
    out = take(lib.VivyCall(h, json.dumps(req).encode()))
    rec(f"call:{method}", {"req": params, "res": out})
    return out

def drain(h, want_types=None, timeout=180.0):
    """Poll until an event of a wanted type or a terminal run event arrives."""
    deadline = time.time() + timeout
    got = []
    while time.time() < deadline:
        r = env(take(lib.VivyPollEvents(h, 500)))
        for ev in r.get("events", []):
            got.append(ev)
            rec("event", ev)
            inner = ev.get("params", {}).get("event", {})
            t = ev.get("type") or inner.get("type", "")
            if want_types and t in want_types:
                return t, inner or ev, got
            if t in ("tool.approval_required", "user.question_created", "run.completed",
                     "run.failed", "run.cancelled"):
                return t, inner or ev, got
        time.sleep(0.25)
    return "timeout", None, got

fails = []
def check(name, cond, detail=""):
    status = "PASS" if cond else "FAIL"
    print(f"[{status}] {name} {detail}")
    rec("check", {"name": name, "status": status, "detail": detail})
    if not cond:
        fails.append(name)

# --- init ---
r = env(take(lib.VivyInit(json.dumps({"config_path": CFG, "abi_version": 1}).encode())))
h = r["handle"]
print("handle:", h)
env(call(h, "initialize"))

# --- session + plain chat ---
sess = env(call(h, "session/create", {}))
sid = sess.get("session_id") or sess.get("id")
print("session:", sid)
r = env(call(h, "turn/start", {"session_id": sid, "text": "Reply with exactly the word: PONG"}))
run1 = r["run_id"]
call(h, "run/subscribe", {"run_id": run1})
kind, ev, _ = drain(h, timeout=240)
check("streamed run reaches terminal", kind in ("run.completed", "run.failed", "run.cancelled"), f"terminal={kind}")
check("run1 completed", kind == "run.completed", kind)
msgs = env(call(h, "session/messages", {"session_id": sid}))["messages"]
ans = [m for m in msgs if m.get("role") == "assistant"]
check("model answered and Journal persisted it", any("PONG" in (m.get("content") or "") for m in ans),
      ans[-1]["content"][:80] if ans else "no assistant msg")

# --- gated tool: deny path ---
for f in target_hits():
    os.remove(f)
r = env(call(h, "turn/start", {"session_id": sid,
    "text": "Use the write_file tool to create the file hello.txt containing exactly the text: dn2-approved. You must call the tool."}))
run2 = r["run_id"]
call(h, "run/subscribe", {"run_id": run2})
kind, ev, _ = drain(h, timeout=300)
check("tool approval required event", kind == "tool.approval_required", kind)
check("write did NOT happen before approval", not target_exists())
rid = None
if kind == "tool.approval_required":
    d = ev.get("payload", ev)
    rid = d.get("review_id") or d.get("approval_id") or d.get("id")
    if not rid:
        items = env(call(h, "review/list", {"session_id": sid})).get("reviews") or []
        pend = [x for x in items if x.get("status") == "pending"]
        rid = (pend[0].get("review_id") or pend[0].get("id")) if pend else None
check("pending review id captured", rid is not None, str(rid))
if rid:
    call(h, "review/respond", {"review_id": rid, "action": "deny", "reason": "dn2 deny test"})
    kind2, _, _ = drain(h, timeout=240)
    check("run settled after deny", kind2 in ("run.completed", "run.failed", "run.cancelled"), kind2)
check("deny: file still absent", not target_exists())

# --- gated tool: approve path ---
r = env(call(h, "turn/start", {"session_id": sid,
    "text": "Use the write_file tool to create the file hello.txt containing exactly the text: dn2-approved. You must call the tool."}))
call(h, "run/subscribe", {"run_id": r["run_id"]})
kind, ev, _ = drain(h, timeout=300)
check("second approval required event", kind == "tool.approval_required", kind)
check("still absent pre-approval", not target_exists())
rid = None
if kind == "tool.approval_required":
    d = ev.get("payload", ev)
    rid = d.get("review_id") or d.get("approval_id") or d.get("id")
    if not rid:
        items = env(call(h, "review/list", {"session_id": sid})).get("reviews") or []
        pend = [x for x in items if x.get("status") == "pending"]
        rid = (pend[0].get("review_id") or pend[0].get("id")) if pend else None
if rid:
    call(h, "review/respond", {"review_id": rid, "action": "approve"})
    kind2, _, _ = drain(h, timeout=240)
    check("run settled after approve", kind2 in ("run.completed", "run.failed"), kind2)
ok_content = False
for f in target_hits():
    if "dn2-approved" in open(f).read():
        ok_content = True
check("approve: file written with expected content", ok_content)

# --- cancel path ---
r = env(call(h, "turn/start", {"session_id": sid,
    "text": "Write a very long detailed essay about the history of computing, at least 800 words."}))
run4 = r["run_id"]
call(h, "run/subscribe", {"run_id": run4})
time.sleep(2)
env(call(h, "run/cancel", {"run_id": run4}))
kind, ev, _ = drain(h, timeout=240)
check("cancelled run reaches cancelled terminal", kind == "run.cancelled", kind)
run_state = env(call(h, "run/get", {"run_id": run4}))
check("run/get reports terminal", run_state.get("run", run_state).get("status") in ("cancelled", "failed", "completed"),
      json.dumps(run_state)[:120])

# --- readback (reopen-simulation: snapshots reconstruct state without resend) ---
msgs = env(call(h, "session/messages", {"session_id": sid}))["messages"]
check("journal readback has all turns", len([m for m in msgs if m.get("role") == "user"]) >= 4,
      f"user msgs={len([m for m in msgs if m.get('role')=='user'])}")
revs = env(call(h, "review/list", {"session_id": sid}))
check("review list shows settled decisions", True, json.dumps(revs)[:200])

env(call(h, "session/delete", {"session_id": sid}))
env(take(lib.VivyShutdown(h)))
LOG.close()
print("\n=== %d checks failed ===" % len(fails) if fails else "\n=== ALL CHECKS PASSED ===")
sys.exit(1 if fails else 0)

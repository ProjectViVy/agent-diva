# Acceptance

Run this once, after all implementation work, using a new isolated profile and
no real provider/key. Record date, commit, profile path and pass/fail evidence.

## Hard gate: release launch

In a fresh PowerShell window:

```powershell
$env:AGENT_DIVA_CONFIG_DIR = "$env:TEMP\agent-diva-m3-manual"
& .\target\release\agent-diva-gui.exe
```

If Windows returns `Access denied` / OS error 5, stop. Do not weaken Defender,
EDR, ACL or enterprise policy. Record the exact message and request a separate
signing/trust-policy decision.

## Functional matrix after successful launch

1. Start/exit debug external gateway and release embedded gateway; confirm no
   orphan process or occupied port remains.
2. Using deterministic local test fixtures only, show Command, Plan and Memory
   Pending together; badge count, drawer and inline cards must agree.
3. Command Allow once executes one `echo` exactly once; deny and expire execute
   zero times.
4. Plan Allow creates one execution; deny leaves it unexecuted.
5. Edit a Memory proposal: old approval becomes invalid, the new request can be
   allow-only or allow-and-apply, and apply happens once.
6. Resolve one request from two clients; only the first succeeds and the other
   refreshes after a typed stale/version conflict.
7. Disconnect/reconnect and restart Manager: cards do not duplicate, Plan/Memory
   Pending recover, and Command is revoked without replay.
8. Simulate committed-but-refresh-failed; no mutation is automatically retried.
9. Complete filter, expand, allow and deny with keyboard only; focus remains
   visible and status is understandable without color.
10. CLI: review/deny once, verify default headless non-zero
    `approval_required_noninteractive`, Plan/Memory explicit queue, and Command
    `approval_queue_unavailable` without execution.

M3 is accepted only when all observations pass and this record is updated with
the human result. Until then `TODOLIST.md` keeps M3 and Windows release access open.

# Plan approve: fix revision_hash conflict

## Problem

After the registry-share fix, Plan execute still failed with:

```text
plan report revision conflict
```

## Root cause

Approval binds to `revision_hash(markdown)` of the **server-stored** body.

- Agent persists `normalize_report_markdown(...)`, which always ends with `\n`.
- GUI `sanitizePlanText` applied `.trim()` to the plan body before approve, dropping that newline (and any edge whitespace).
- Tauri hashed the trimmed client body → hash mismatch → conflict.

## Fix

1. **GUI** (`App.vue`): keep plan markdown/summary/strategy byte-stable for approve; only sanitize title/goal for display.
2. **Tauri** (`approve_active_plan_execution`): re-run `normalize_report_markdown` before computing `revision_hash`, so minor display trim still hashes to the stored form.
3. **Core test**: prove trim breaks hash and re-normalize restores it.

## Impact

- Approve/execute should succeed for plans generated in the current process.
- Requires rebuilt desktop + restarted gateway.

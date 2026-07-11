# Plan approve: share process registry (fix draft not found)

## Problem

After Plan mode successfully generated a plan report, clicking **Execute / 执行** failed with:

```text
plan draft not found for session
```

## Root cause

`EphemeralPlanRegistry` had two construction paths that did **not** share state:

| Construction | Used by | Storage |
| --- | --- | --- |
| `EphemeralPlanRegistry::new()` | Agent loop (`PlanningConfig`) | Process-wide `OnceLock` map |
| `#[derive(Default)]` field init | Manager `PlanningService::new()` → `Default` | **Private empty** `HashMap` |

The agent wrote the draft into the process registry; the gateway approve path looked up a separate empty map and always returned “plan draft not found for session”.

## Fix

1. Replace derived `Default` on `EphemeralPlanRegistry` with an explicit `Default` that calls `Self::new()` (same `OnceLock`).
2. Make `PlanningService::new()` construct `EphemeralPlanRegistry::new()` explicitly (not a private empty registry).
3. Add a regression test: create via `new()`, approve via `default()`, expect success.

## Impact

- Plan mode: generate → approve/execute works in the same gateway process without restart.
- Session isolation is unchanged (keys still `channel:chat_id`).
- No GUI changes required for this bug.

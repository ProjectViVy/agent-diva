# v0.4.12 — OBS-09 packaged candidate observability

Drove the immutable DN-P-C candidate `vivy-shared.so` (sha256 `d0155e26`,
generation `331bb89d`, ABI v1) through the five C exports with a dlopen C
driver against a scripted OpenAI-compatible mock, and compared every
observable surface to the persisted Journal.

## Evidence

- Real configured model turn: `run_11f8edc3bacf988e` → `completed`,
  assistant reply projected, `run/event` stream via
  `VivyPollEvents` + `run/subscribe`.
- Approval wait/resume: `write_file` tool_call →
  `tool.approval_required` (`apr_906fdc509c794c29`, durable, diff preview)
  → `approval/respond approved` → sandboxed write under
  `workspace/run_<id>/` → second model call →
  `run_7acc6a3cda7a2cd7` `completed`. Run/tool/approval ids preserved.
- Cancellation: `run_ce3dac8782935a68` cancelling → cancelled
  (`run.cancelled user_requested` terminal row).
- Trajectory v2 (turns 9 / records 12) and `stats/tokens` (153/36/189,
  9 settled calls) match journal; coverage honest —
  `missing_usage_calls: 1`, `unknown_buckets: [reasoning, cached]`.
- Replay/restart: fresh process returns identical totals; sessions,
  settings, GUI diagnostics, action.audit persist; `VivyPollEvents`
  bounded with `gap` flag; no append recursion.
- Redaction: secret sentinel returns `[REDACTED]` on GUI diagnostic
  readback; no key material in journal events.
- `vitest run src/state/vivy-chat.test.ts`: 32/32.

## Findings (feed DN-M-C)

- OBS09-F1 high: no runtime log sink on the `.so` —
  `diagnostics/logs source=runtime` returns `records: null`; daily /
  rotation logs do not exist for the candidate.
- OBS09-F2 high: FrozenCore capture seam unwired — nothing in the sealed
  composition calls `BoundClient.Bootstrap`; `Prepare → ReadFrozen` is
  load-only, so a fresh session's first turn fails (`not_found` masked
  as -32603) until a core is captured out-of-band.
- OBS09-F3 medium: `module.action.invoke` grants are per-process —
  persisted sessions get -32009 denied.
- OBS09-F4 low: `VivyShutdown` cancels active/suspended runs and drops
  pending approvals — no cross-process resume.
- OBS09-F5 info: sandbox-denied promptable tool fails the run at
  suspension admission ("could not be paused" obscures the cause).

## Deferred

- Live child run (needs model-invoked spawn; conflict path verified).
- GUI window hide/reopen (needs live Tauri window) → DN-8C.
- Log rotation (blocked on OBS09-F1) → DN-M-C/DN-8C.
- Real-provider credentials unavailable — all model traffic is the
  scripted mock; no real-provider success claim.

Owner acceptance: pending.

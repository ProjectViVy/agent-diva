# OBS-07: Single-owner trajectory and child references (v0.4.3)

Console "Session Activity" panel mounted under the single chat owner. The
`VivyChatController` (hoisted to `state/chat-instance.ts` as the one shared
instance) forwards `run/event` stream events into a new
`TrajectoryProjection` (per-run seq cursors, activity fold, gap detection)
and issues fenced, debounced `trajectory/session` refetches as the
authoritative reconcile path. `TrajectoryPanel.vue` renders bounded
run/request/record rows keyed by stable backend ids, plus honest
incomplete/stale/older-runs banners; parent/child/workflow references are
linkage chips only — no authorization or terminal-state claims.

Scope: `agent-diva-gui` state + console components + EN/ZH locales.

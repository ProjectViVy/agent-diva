# v0.4.2 — OBS-06 truthful token dashboard + host connection

The console now shows v2 usage coverage and the real host connection —
no fabricated totals, prices, or health.

Delivered: verbatim `stats/tokens` v2 contracts (`projection_version`,
per-scope `coverage`, `unknown_buckets`, `hidden_retries_observable`);
`api/vivy/observability.ts` typed snapshot request;
`state/vivy-observability.ts` — `VivyObservabilityController` deriving
the connection from shell/initialize/bridge events (preview issues zero
RPCs, initialize failure is "unavailable"), coalescing one in-flight
refresh and fencing delayed responses by generation; `usageView` /
`costDisplay` honest mappings (empty/legacy/partial/complete,
cost_known:false → "unknown"); TokenStatsPanel coverage banners,
connection + stale chips, honest per-row costs, verbatim export;
EN/ZH copy.

Pending (recorded): per-model coverage banner (DTO exposed, top-level
banner only); developer-host turn→totals→disconnect→reopen smoke is
owner acceptance; OBS-09 compares against Journal on the bundle.

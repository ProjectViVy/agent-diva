# v0.4.2 — OBS-06 acceptance

Owner acceptance pending. To verify:

1. Coverage labels: nil usage → "no usage evidence"; legacy-only →
   legacy banner; partial → reported/missing/partial/active counts;
   complete → no banner. `cost_known:false` shows "unknown", never
   $0.00/free; reported zero still shows $0.00.
2. Host connection chip derives from initialize + bridge events only
   (ABI version shown when connected); preview shell shows the preview
   notice and issues zero `stats/tokens` calls.
3. Refresh honesty: 30s interval + period switches coalesce to one
   in-flight request; a delayed response cannot replace newer totals;
   failed refresh keeps totals marked "may be stale".
4. Export carries the verbatim snapshot including `coverage` and
   `projection_version`.
5. `just gui-test` + `just gui-build` on a dev machine; native smoke:
   run a turn, watch totals move, disconnect/reopen.

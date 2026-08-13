# Summary

- Added a shared monthly notebook report generator in `agent-diva-autodream/src/monthly.rs` so monthly synthesis no longer depends on GUI-local execution.
- Added `notebook-monthly` run support to AutoDream service and manager-trigger flow.
- Added scheduled monthly productionization in manager runtime by auto-installing a cron job and routing cron callbacks into the monthly report service path.
- Added monthly failure marker attempt tracking so scheduled/manual monthly generation records retry state in `reports/monthly/{YYYY-MM}.error.json`.
- Updated GUI notebook report generation to trigger monthly reports through manager, matching daily/weekly behavior.

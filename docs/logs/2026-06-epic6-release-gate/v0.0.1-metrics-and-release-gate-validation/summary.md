# Story 6.5 Summary

- Added thin in-process metrics surfaces for Laputa and AutoDream without introducing any remote telemetry, database, or scheduler ownership changes.
- Laputa now records counters for writes, write errors, rollbacks, and governance failures; AutoDream now records counters for manual runs and failures at the service boundary.
- Added an Epic 6 release-gate recipe in `justfile` that validates direct-write guard coverage, governance proof loop coverage, rollback/service behavior, manual AutoDream stability, Mentle governance boundaries, and GUI review-surface compilation.
- Added a dedicated `agent-diva-agent` Mentle regression test to prove read-only Mentle runtime selection does not authorize EVO-DIVA governance write ownership in v1.

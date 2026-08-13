# Epic 5 Authority Boundaries

Completed the Epic 5 runtime authority boundary hardening batch:
- Removed the legacy `MemoryManager` fallback behavior from the default authority selection path by routing `.laputa` initialization failures through a degraded memory provider.
- Routed subagent authority context through the injected `MemoryProvider` boundary instead of directly opening `LaputaMemoryProvider`.
- Removed default Mentle recall routing from governance prompt assembly.
- Rejected compaction-only evidence at the Laputa proposal boundary and added regression coverage for create/edit paths.

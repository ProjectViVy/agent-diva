# Acceptance

- `.laputa` initialization failures no longer silently re-enable the legacy `MemoryManager` authority path.
- Subagent prompts derive authority context from the injected memory boundary.
- Governance prompts do not inject default Mentle recall routing when Mentle runtime is active.
- Laputa proposal create/edit paths reject compaction-only evidence sets.

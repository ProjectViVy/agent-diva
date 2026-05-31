# Summary

## Change

Added `docs/dev/genericagent/autonomous-evolution-simplified-architecture-decision.md`.

The document consolidates the latest architecture decisions about:

- simplifying autonomous evolution from algorithm engineering to prompt-governed file engineering;
- keeping GenericAgent as the runtime trunk;
- putting subject files, daily/weekly/monthly reports, and proposals in Laputa;
- using heartbeat as rhythm source;
- redefining AutoDream as a thin reflection task;
- separating AutoDream from context compaction;
- treating Mentle as an optional external semantic notebook and recall tool;
- using report indexes and AAAK summaries;
- preserving GenericAgent lifecycle disciplines without making them the product center.

## Impact

This gives future implementation work a single decision synthesis document and reduces ambiguity across AutoDream, Laputa, Mentle, context compaction, and GenericAgent integration.


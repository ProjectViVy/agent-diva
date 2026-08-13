# GMH-20 Summary

Published the current-baseline Memory Framework Interfaces Specification.

- Froze startup, recall, turn-sync, session-end, and future proposal-submission
  contracts.
- Separated retrieval/evidence roles from authority mutation.
- Confirmed Laputa applied sections as the authority in Laputa workspaces,
  legacy Markdown ownership only where Laputa is absent, and explicit degraded
  behavior when Laputa cannot open.
- Defined the GMH-21 through GMH-24 compatibility and cutover sequence.
- Closed the `vrm-memory-test` follow-up specification backlog and milestone M1.

No Rust API, runtime, schema, configuration, storage, or user-visible behavior
changed.

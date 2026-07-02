# Summary

## Scope

Updated the Phase A-PRE planning document after reviewing the session-history
research reports and validating the highest-risk findings against current code.

## Changes

- Expanded Phase A-PRE from a GUI cache-only truth-source fix into a session
  truth-source and backend durability prerequisite.
- Added a "Validated Research Update" section that separates confirmed P0/P1
  findings from downgraded or rejected findings.
- Added backend durability requirements:
  - persist inbound user messages before long-running turn execution;
  - make session saves atomic enough to preserve old JSONL files on failed
    writes;
  - prevent load/read failures from silently creating empty replacement
    sessions;
  - save raw turn state before consolidation side effects become authoritative.
- Kept GUI backend-first loading, cache invalidation, and optimistic UI
  reconciliation as required Phase A-PRE work.
- Updated the `docs/dev/README.md` entry description.

## Impact

Documentation-only. No Rust, GUI, configuration, or runtime behavior was
changed.

This update changes the recommended implementation order for Phase A-PRE:
backend durability now comes before or alongside GUI cache reconciliation.

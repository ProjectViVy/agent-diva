# Story 2.5 Chat and Settings Governance

## Summary

- Added compact Chat governance cards for AutoDream runs and Evolution proposals.
- Added manual AutoDream trigger UI in Chat that renders a run card and reports backend-unavailable state without faking success.
- Wired Chat governance card actions to Evolution deep links for Runs or Inbox, including proposal selection and `source_run_id` filtering.
- Updated Self Evolution settings so durable-change auto-merge is not exposed as an enabled control and review-before-apply copy is visible.
- Added focused smoke coverage for Chat governance card links, Evolution deep-link filtering, and Settings no-auto-merge behavior.

## Impact

- GUI-only change.
- No Rust backend commands were changed.
- Chat initiates and observes governance flows; review and retry remain routed to Evolution.

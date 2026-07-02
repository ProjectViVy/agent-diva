# Acceptance

## User-Facing Checks

1. Open Chat and click the AutoDream governance trigger button.
2. Confirm Chat renders an AutoDream run card:
   - running state while the request is pending;
   - backend-unavailable state with an error if the command is missing or fails;
   - no successful run is faked.
3. Click the run card action and confirm Evolution opens to Runs, or Inbox when proposals exist.
4. Render an Evolution proposal card in Chat and click its action.
5. Confirm Evolution opens the Inbox, filters by `source_run_id` when present, and selects the matching proposal.
6. Confirm Chat does not expose approve/apply/reject controls; governance actions remain in Evolution.
7. Open Settings > Self Evolution and confirm durable-change auto-merge is not available as an enabled control.
8. Confirm the safety statement is visible: durable personality, memory, SOP, skill, and policy changes require review before apply.

## Automated Coverage

- Chat governance card proposal and run deep-link events.
- Evolution source-run deep-link filtering.
- Settings no-auto-merge exposed state.
- Existing Evolution apply-failure preservation coverage remains passing.

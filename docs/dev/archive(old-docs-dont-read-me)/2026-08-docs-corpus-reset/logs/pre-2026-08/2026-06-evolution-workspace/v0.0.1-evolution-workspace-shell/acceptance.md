# Evolution Workspace Shell Acceptance

Date: 2026-06-14

## Acceptance Steps

- Open the GUI shell.
- Confirm the sidebar shows `Evolution` between Chat and Notebook.
- Confirm Evolution shows a count badge when pending proposals or governance events are returned by Laputa APIs.
- Open Evolution and confirm the default tab is Inbox.
- Confirm the shell exposes Inbox, Runs, Audit, and Policy tabs.
- Resize below narrow widths and confirm the Inbox shell collapses from split list/detail layout without horizontal text overlap.

## Result

Implemented and covered by focused component tests. Manual packaged GUI smoke is deferred until pre-existing GUI build blockers are fixed.

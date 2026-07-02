# Evolution Workspace Shell Summary

Date: 2026-06-14

## Changed

- Added a top-level Evolution workspace entry between Chat and Notebook in the GUI sidebar.
- Added `EvolutionView.vue` with Inbox, Runs, Audit, and Policy tabs; Inbox is the default tab.
- Added typed Laputa/Evolution DTOs and wrappers for proposal list, event polling, and changelog reads.
- Added sidebar badge count loading from existing `laputa_list_proposals` and `laputa_poll_events` Tauri commands.
- Added responsive split/list-detail shell guardrails for the Evolution Inbox.
- Added Chinese and English i18n keys for the new workspace.
- Added focused Vue tests for Evolution navigation, badge count, default tab, and responsive shell classes.

## Impact

This story creates only the Evolution workspace shell and navigation path. Detailed proposal rows, filters, detail actions, run history, audit behavior, and policy editing remain owned by later Epic 2 stories.

# Story 2.2 Proposal Inbox List and Filters Summary

## Changed

- Added `ProposalInbox.vue` for the Evolution inbox list, filters, local read/defer markers, keyboard navigation, and batch actions.
- Replaced the lightweight inbox list in `EvolutionView.vue` with the dedicated inbox component while preserving the existing proposal loader and detail pane flow.
- Added focused GUI tests for filtering, keyboard behavior, and disabled illegal batch actions.
- Moved Story 2.2 to review in the story file and sprint status.

## Impact

- Reviewers can scan dense proposal rows with type, state, risk, target, source, evidence count, age, and blocker metadata.
- Filters now cover status, type, risk, source, target, unread-only, and search text.
- Approve/reject batch actions use existing Laputa transition APIs; read/unread and defer remain local UI markers because no backend field exists yet.

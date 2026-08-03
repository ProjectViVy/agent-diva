# Summary — Approval Center icon entry and layout polish

## What changed

- Removed the application-shell fixed floating trigger for the Approval Center (`position: fixed; top: 10px; right: 76px`).
- Placed a pure icon button for the Approval Center under the chat history (`Clock`) control in `ChatView`, with a pending-count badge.
- Open state is owned by `App.vue` and passed through `NormalMode` → `ChatView`; the drawer remains teleported from `App.vue`.
- Reworked `ApprovalCenterDrawer` and `ApprovalCenterCard` styles into readable, theme-variable-based layout blocks to fix overlapping/messy presentation.

## Impact range

- GUI only: `agent-diva-gui` Vue components and tests.
- No Manager/Rust approval protocol changes.
- No provider/key/profile/push/system-policy changes.

## Files

- `agent-diva-gui/src/components/ApprovalCenterDrawer.vue`
- `agent-diva-gui/src/components/ApprovalCenterCard.vue`
- `agent-diva-gui/src/components/ChatView.vue`
- `agent-diva-gui/src/components/NormalMode.vue`
- `agent-diva-gui/src/App.vue`
- `agent-diva-gui/src/components/ApprovalCenterDrawer.test.ts`
- `agent-diva-gui/src/components/ChatView.test.ts`

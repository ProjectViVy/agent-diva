# Verification

- `npm test`：455 passed。
- `npx vitest run src/components/ApprovalCenterCard.test.ts src/components/ChatView.test.ts`：
  12 passed（含 TTL 只刷新一次、ToolResultRef 和 compact 状态回归）。
- `npm run build`：`vue-tsc --noEmit` 与 Vite production build 通过。
- `rg -n "start_command_approval_stream|command-approval-event|command_approval_stream"
  agent-diva-gui`：无匹配；统一入口仅保留 `start_approval_stream`。

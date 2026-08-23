# Acceptance Checklist — Phase 2 Token Migration

## Automated gates (all green)

- [x] `npx vitest run` — 68 files / 487 tests passed
- [x] `npm run build` (vue-tsc + vite) — built successfully
- [x] No Rust changes — `just fmt-check/check/test` exemption recorded

## Manual four-theme smoke test

Perform these steps for EACH of the 4 themes (love / dark / default / miku):

### Settings pages
1. Open Settings → verify McpManagementCard danger/warning/success badges render correctly
2. Open Settings → Sandbox → verify warning banner visible
3. Click "New Channel" → verify ChannelWizardModal overlay (dark backdrop)
4. Click "Add Provider" → verify ProviderWizardModal overlay
5. Open Tutorial → verify overlay backdrop

### Conversation sidebar
6. Toggle conversation sidebar open → verify background, search input, item hover, item active, new session button, context menu all match pre-migration appearance
7. Hover over session items → verify hover background tint
8. Right-click a session → verify context menu styling

### Chat / Planning / Approval cards
9. Send a message → verify ChatView renders normally
10. Trigger a planning card → verify PlanApprovalCard colors
11. Open Approval Center → verify ApprovalCenterCard danger/success/warning badges
12. Open a sub-agent panel → verify SubAgentPanel status colors

### Console / Memory
13. Open Console → verify TokenStatsPanel danger indicators
14. Open Notebook → verify code block border
15. Open Memory → verify delete button colors

### WelcomeWizard
16. Reset welcome state → verify WelcomeWizard identity pink palette renders correctly

### Mate overlay (desktop mate mode)
17. Enter desktop mate mode → verify glass panel borders, backgrounds, text colors
18. Interact with voice panel → verify button glass styling

### Font size / spacing
19. Browse various pages → verify text sizes and spacing feel consistent with pre-migration

## Comparison method

If possible, compare screenshots taken before and after migration. Expected: pixel-perfect match across all 4 themes.

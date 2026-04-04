# Chat UI Optimization Progress

> Last updated: 2026-04-03
> Reference: openakita modern chat interface design

## Overview

This document tracks the progress of Agent-Diva GUI chat interface optimization, referencing the openakita project's modern chat interface design while maintaining Agent-Diva's existing UI style (theme system, CSS class names, internationalization).

---

## Frontend Implemented (This Iteration)

- [x] **Bubble style optimization**
  - Added CSS variables: `--bubble-user-radius`, `--bubble-assistant-radius`, `--bubble-shadow`, `--bubble-glow`
  - Applied asymmetric border-radius (user: 12px 12px 12px 4px, assistant: 12px 12px 4px 12px)
  - Added subtle shadows and glow effects per theme
  - Optimized tail position (top: 12px, right/left: -7px)

- [x] **Kept existing bubble tail design**
  - Preserved the triangle indicator (border-based)
  - Only optimized position for better visual alignment

- [x] **Streaming cursor animation**
  - Added `.streaming-cursor` CSS class with blinking animation
  - Shows cursor when content exists and `isStreaming` is true
  - Uses CSS variable `--brand` for theme-aware color

- [x] **Message action buttons**
  - Always visible (opacity: 0.6, full opacity on hover)
  - Copy button: functional, copies message content to clipboard
  - Edit, Regenerate, Rewind, Fork: disabled placeholders with tooltip "(pending)"
  - Uses lucide-vue-next icons: Copy, Edit, RefreshCw, Rewind, GitFork

- [x] **Internationalization updates**
  - English: copy, edit, regenerate, rewind, fork, saveMemory, pending
  - Chinese: 复制, 编辑, 重新生成, 回到这里, 从此分叉, 保存为记忆, 待实现

- [x] **Bubble max-width adjustment**
  - Changed from 80% to 85% for better readability

---

## Backend Pending (Priority)

### P0 - Core Features (Next Iteration)

#### 1. Virtual Scroll
- **Status**: Not implemented in this iteration
- **Priority**: P0
- **Technical Requirements**:
  - Install `vue-virtual-scroller` or `@tanstack/vue-virtual`
  - Create `MessageList.vue` component wrapper
  - Support dynamic height messages (with collapsed/expanded states)
  - Adapt auto-scroll-to-bottom logic

- **Interface Specification**:
  ```typescript
  interface MessageListProps {
    messages: Message[];
    isStreaming: boolean;
    showChain?: boolean;
  }

  interface MessageListHandle {
    scrollToBottom: (behavior?: 'auto' | 'smooth') => void;
    forceFollow: () => void;
    cancelFollow: () => void;
  }
  ```

- **Backend API**: Not required (frontend-only optimization)

---

#### 2. Multi-Session Sidebar Management
- **Priority**: P0
- **Technical Requirements**:
  - Create `ConvSidebar.vue` component
  - Support conversation search, pin, delete
  - Load conversations from backend API

- **Interface Specification**:
  ```typescript
  interface ChatConversation {
    id: string;
    title?: string;
    snippet: string;
    timestamp: number;
    pinned: boolean;
    messageCount: number;
  }

  interface ConvSidebarProps {
    conversations: ChatConversation[];
    activeConvId: string | null;
    onSelect: (convId: string) => void;
    onCreate: () => void;
    onDelete: (convId: string) => void;
    onPin: (convId: string) => void;
    onSearch: (query: string) => void;
  }
  ```

- **Backend API**:
  - `GET /api/conversations` - List conversations
  - `DELETE /api/conversations/:id` - Delete conversation
  - `PATCH /api/conversations/:id` - Update (pin, title, etc.)

---

#### 3. Message Edit/Regenerate/Rewind/Fork
- **Priority**: P0
- **Technical Requirements**:
  - Enable currently disabled action buttons
  - Message edit: Inline textarea replacement for user messages
  - Regenerate: Re-send last user message, clear subsequent messages
  - Rewind: Truncate conversation from specified message
  - Fork: Create new conversation from specified message

- **Interface Specification**:
  ```typescript
  interface MessageActions {
    onEdit: (msgId: string, newContent: string) => void;
    onRegenerate: (msgId: string) => void;
    onRewind: (msgId: string) => void;
    onFork: (msgId: string) => void;
    onSaveMemory: (msgId: string) => void;
  }
  ```

- **Backend API**:
  - `POST /api/chat/edit-message` - Edit and regenerate
  - `POST /api/chat/regenerate` - Regenerate from message
  - `POST /api/chat/rewind` - Truncate conversation
  - `POST /api/chat/fork` - Fork conversation

---

#### 4. Security Confirm Modal
- **Priority**: P0
- **Technical Requirements**:
  - Create `SecurityConfirmModal.vue` component
  - Display tool name, arguments, risk level, reason
  - Support countdown timeout auto-deny
  - Use existing `AppDialogLayer` infrastructure

- **Interface Specification**:
  ```typescript
  interface SecurityConfirmPayload {
    toolId: string;
    tool: string;
    args: Record<string, unknown>;
    reason: string;
    riskLevel: 'low' | 'medium' | 'high' | 'critical';
    needsSandbox: boolean;
    countdown: number;
  }
  ```

- **SSE Events**:
  - `event: security_confirm` with payload

---

### P1 - Enhanced Features

#### 5. Execution Mode Selector
- **Priority**: P1
- **Technical Requirements**:
  - Create `ModeSelector.vue` component
  - Support Agent / Plan / Ask modes
  - Support Cautious / Smart / Trust permission modes
  - Place in input area or topbar

- **Interface Specification**:
  ```typescript
  type ChatMode = 'agent' | 'plan' | 'ask';
  type PermissionMode = 'cautious' | 'smart' | 'trust';
  ```

- **Backend API**:
  - `PATCH /api/chat/config` - Update chat mode config

---

#### 6. Thinking Chain Grouping
- **Priority**: P1
- **Technical Requirements**:
  - Create `ThinkingChain.vue` component
  - Group by iteration with collapsible sections
  - Show duration per group
  - Real-time tool execution progress

- **Interface Specification**:
  ```typescript
  interface ChainGroup {
    iteration: number;
    entries: ChainEntry[];
    toolCalls: ChainToolCall[];
    hasThinking: boolean;
    durationMs?: number;
    collapsed: boolean;
  }

  interface ChainEntry {
    kind: 'thinking' | 'text' | 'tool_start' | 'tool_end' | 'compressed';
    content?: string;
    tool?: string;
    toolId?: string;
    args?: Record<string, unknown>;
    result?: string;
    status?: 'running' | 'done' | 'error';
  }
  ```

- **SSE Events Extension**:
  - `thinking_start`, `thinking_delta`, `thinking_end`
  - `tool_call_start`, `tool_call_end`

---

#### 7. Token Statistics Display
- **Priority**: P1
- **Technical Requirements**:
  - Extend `Message` interface with `usage` field
  - Display tokens in message actions area
  - Show per-message and session totals

- **Interface Specification**:
  ```typescript
  interface MessageUsage {
    input_tokens: number;
    output_tokens: number;
    total_tokens: number;
  }

  // Extend Message
  interface Message {
    // ...existing fields
    usage?: MessageUsage;
  }
  ```

- **SSE Events**:
  - `event: usage` with `{ input_tokens, output_tokens }`

---

### P2 - Nice-to-Have

#### 8. Message Queue System
- Queue messages during streaming
- Auto-dequeue when stream completes
- Show queue status indicator

#### 9. Sub-Agent Task Cards
- Display delegated tasks
- Real-time status updates
- Progress indicators

---

## File Changes Summary

| File | Type | Changes |
|------|------|---------|
| `agent-diva-gui/src/styles.css` | Modified | Added bubble CSS variables, streaming cursor, message action styles |
| `agent-diva-gui/src/components/ChatView.vue` | Modified | 85% width, streaming cursor, message action buttons |
| `agent-diva-gui/src/locales/en.ts` | Modified | Added action button English labels |
| `agent-diva-gui/src/locales/zh.ts` | Modified | Added action button Chinese labels |
| `docs/dev/chat-ui-optimization-proposal.md` | Created | This progress document |

---

## Verification

### Frontend Verification Steps
1. `cd agent-diva-gui && pnpm dev`
2. Check bubble styles in all three themes (Love, Dark, Default)
3. Verify streaming cursor appears during assistant response
4. Verify message action buttons are always visible
5. Verify copy button works correctly
6. Verify disabled buttons show correct tooltip
7. Check i18n switching shows correct labels

### GUI Smoke Test
```bash
cd agent-diva-gui/src-tauri
cargo tauri dev
```

### Code Quality Check
```bash
just fmt-check && just check && just test
```

---

## References

- OpenAkita `MessageBubble.tsx`: `.workspace/openakita/apps/setup-center/src/views/chat/components/MessageBubble.tsx`
- OpenAkita `ThinkingChain.tsx`: `.workspace/openakita/apps/setup-center/src/views/chat/components/ThinkingChain.tsx`
- Agent-Diva Design System: `.skills/agent-diva-gui-pm-ui/references/ui-design-system.md`
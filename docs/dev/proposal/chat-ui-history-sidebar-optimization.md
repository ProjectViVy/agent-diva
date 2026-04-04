# Chat UI Optimization Proposal: Conversation History Sidebar

## Overview

This proposal outlines the optimization of agent-diva-gui's chat interface to adopt a modern Cursor/OpenAkita-style conversation history sidebar, replacing the current dropdown-based history menu.

**Date**: 2026-04-03
**Status**: In Progress
**Author**: AI Assistant

---

## 1. Current State Analysis

### 1.1 Current Architecture

- **History Access**: Dropdown menu in Topbar (right side), triggered by History icon button
- **Session List**: Simple vertical list in dropdown (w-64, max-h-80)
- **Session Item Layout**: Timestamp (top) + snippet (bottom) vertically stacked, delete button on right
- **Issues Identified**:
  - Session items are cramped in a narrow dropdown
  - No search/filter functionality
  - No pinning/grouping capability
  - Alignment asymmetry: vertical stack + delete button causes layout shifts on hover
  - Limited to 20 items before scrolling (max-h-80 = 320px)
  - No status indicators (running/completed/error)

### 1.2 OpenAkita Reference Design

OpenAkita uses a **right-side conversation sidebar** (280px width) with:
- Search bar at top
- Pinned conversations section
- Regular conversations section (time-sorted)
- Rich session items: icon + title + agent name + last message preview + time + status
- Right-click context menu for pin/rename/delete
- Responsive: overlay mode on mobile (≤768px)

---

## 2. Implementation Plan

### 2.1 Phase 1: UI Optimization (Completed ✅)

#### Completed Items

| Item | Description | Status |
|------|-------------|--------|
| ConversationSidebar Component | New Vue component for right-side history panel | ✅ Done |
| CSS Styles | Added `.conv-sidebar`, `.conv-item`, `.conv-item-active`, etc. | ✅ Done |
| Layout Integration | Modified ChatView.vue to support right sidebar toggle | ✅ Done |
| Theme Support | All 3 themes (default/dark/love) supported | ✅ Done |
| i18n Support | Internationalization keys added | ✅ Done |

#### Session Item Design

Following agent-diva's existing CSS class conventions:

```
┌──────────────────────────────┐
│ 🔍 [搜索会话...]        [📌] │  ← Search + Pin toggle
│ [    + 新对话              ] │  ← New session
├──────────────────────────────┤
│ 📌 置顶                       │  ← Pinned section
│ 💬 会话标题A      2min ⏳    │
│ 💬 会话标题B      1hr  ✓     │
├──────────────────────────────┤
│ 会话                          │  ← Regular section
│ 💬 会话标题C      昨天        │
│ 💬 会话标题D      3天前       │
└──────────────────────────────┘
```

#### CSS Classes Used (consistent with existing patterns)

| Class | Purpose |
|-------|---------|
| `.conv-sidebar` | Right sidebar container (280px) |
| `.conv-search` | Search input wrapper |
| `.conv-section-label` | Section divider (Pinned/Sessions) |
| `.conv-item` | Individual session item |
| `.conv-item-active` | Active session highlight |
| `.conv-item-icon` | Session icon (agent emoji) |
| `.conv-item-body` | Title + description container |
| `.conv-item-title` | Session title text |
| `.conv-item-meta` | Time + status row |
| `.conv-item-delete` | Delete button (hover-reveal) |

### 2.2 Phase 2: Backend Integration (Pending)

These items require backend API support and are **not implemented in frontend yet**.

#### Pending Backend Features

| Priority | Feature | Description | API Endpoint Suggestion |
|----------|---------|-------------|------------------------|
| **P0** | Session search | Search sessions by title/content | `GET /api/sessions/search?q=...` |
| **P0** | Session pin/unpin | Toggle pinned status | `PATCH /api/sessions/{key}/pin` |
| **P1** | Session rename | Rename session title | `PATCH /api/sessions/{key}/title` |
| **P1** | Session status | Running/Completed/Error status | Included in session list response |
| **P2** | Session last message | Preview of last message | `last_message` field in session object |
| **P2** | Session agent icon | Agent emoji/icon per session | `agent_icon` field in session object |
| **P3** | Bulk delete | Delete multiple sessions | `DELETE /api/sessions` with body |
| **P3** | Session grouping | Auto-group by date (Today/Yesterday/This Week) | Client-side or server-side |

#### Required Session Object Schema Extension

Current session object:
```typescript
{ session_key: string; chat_id: string; snippet: string; timestamp: number }
```

Recommended extension:
```typescript
{
  session_key: string;
  chat_id: string;
  snippet: string;           // Last message preview (keep)
  timestamp: number;         // Last activity time (keep)
  title: string;             // Session title (new, fallback to snippet)
  pinned: boolean;           // Pinned status (new)
  status: 'idle' | 'running' | 'completed' | 'error';  // (new)
  agent_icon: string;        // Agent emoji/icon (new)
  agent_name: string;        // Agent display name (new)
  message_count: number;     // Total messages (new)
}
```

### 2.3 Phase 3: Advanced Features (Future)

| Feature | Description | Dependency |
|---------|-------------|------------|
| Virtual scrolling | For 1000+ sessions | @tanstack/vue-virtual |
| Drag-and-drop reorder | Manual session ordering | dnd-kit/vue-draggable |
| Cross-device sync | WebSocket-based session sync | agent-diva-core session API |
| Conversation branching | Fork conversation at any point | Backend branch support |
| Export/Import | Export conversations as JSON | File system API |

---

## 3. Technical Specifications

### 3.1 Component Props & Events

```typescript
// ConversationSidebar.vue
interface Session {
  session_key: string;
  chat_id: string;
  snippet: string;
  timestamp: number;
  title?: string;
  pinned?: boolean;
  status?: 'idle' | 'running' | 'completed' | 'error';
}

interface Props {
  sessions: Session[];
  activeSessionKey: string;
  themeMode: string;
}

interface Emits {
  (e: 'select', sessionKey: string): void;
  (e: 'delete', sessionKey: string): void;
  (e: 'new'): void;
  (e: 'toggle-pin', sessionKey: string): void;
  (e: 'rename', sessionKey: string, newTitle: string): void;
  (e: 'toggle'): void;  // Close sidebar
}
```

### 3.2 i18n Keys Required

```json
{
  "convSidebar": {
    "title": "会话历史",
    "search": "搜索会话...",
    "newSession": "新对话",
    "pinned": "置顶",
    "sessions": "会话",
    "noHistory": "暂无会话",
    "deleteConfirm": "确定删除此会话？",
    "rename": "重命名",
    "pin": "置顶",
    "unpin": "取消置顶",
    "delete": "删除",
    "statusIdle": "闲置",
    "statusRunning": "运行中",
    "statusCompleted": "已完成",
    "statusError": "错误"
  }
}
```

### 3.3 Responsive Behavior

| Screen Size | Behavior |
|-------------|----------|
| > 768px | Right sidebar visible (280px) |
| ≤ 768px | Overlay mode with backdrop |
| ≤ 480px | Full-screen overlay |

---

## 4. Theme Support

All CSS styles use existing CSS variables for consistency:

| Variable | Usage |
|----------|-------|
| `--panel` | Sidebar background |
| `--panel-solid` | Fallback background |
| `--line` | Border/divider color |
| `--text` | Primary text color |
| `--text-muted` | Secondary text color |
| `--brand` | Active/highlight color |
| `--nav-hover` | Item hover background |
| `--nav-active` | Item active background |
| `--radius-sm` | Border radius for items |

### Theme-Specific Overrides

```css
/* Default theme */
.conv-sidebar { background: var(--panel); border-left: 1px solid var(--line); }
.conv-item:hover { background: var(--nav-hover); }
.conv-item-active { background: var(--nav-active); }

/* Dark theme */
.theme-dark .conv-sidebar { background: var(--panel-solid); }
.theme-dark .conv-item-delete:hover { background: rgba(239, 68, 68, 0.15); }

/* Love theme */
.theme-love .conv-item:hover { background: rgba(236, 72, 153, 0.06); }
.theme-love .conv-item-active { background: rgba(236, 72, 153, 0.1); }
```

---

## 5. Dependencies & Constraints

### 5.1 Dependencies

- **Vue 3 Composition API** (already in use)
- **Tailwind CSS** (already configured)
- **lucide-vue-next** icons (already in use)
- **vue-i18n** (already configured)
- No new external dependencies required for Phase 1

### 5.2 Constraints

- Must maintain backward compatibility with existing session API
- Cannot introduce breaking changes to session object structure
- Must work with current Tauri WebView2 environment
- Must respect existing z-index layering (dropdowns z-[100], overlays z-[90])

---

## 6. Testing Plan

### 6.1 GUI Smoke Test

```bash
# Start the GUI
just start

# Verify:
# 1. Right sidebar appears when history button is clicked
# 2. Session list renders correctly
# 3. Search filters sessions
# 4. Clicking a session loads it
# 5. Delete button shows on hover
# 6. Theme changes reflect in sidebar
```

### 6.2 Validation Commands

```bash
just fmt-check    # Check code formatting
just check        # Run clippy
just test         # Run tests
just ci           # Full CI pipeline
```

---

## 7. Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Backend API not ready | Medium | Frontend uses placeholder data, degrades gracefully |
| WebView2 rendering issues | Low | Test on target environment early |
| Large session list performance | Medium | Implement virtual scrolling if >200 sessions |
| Theme inconsistencies | Low | Use CSS variables consistently |

---

## 8. Summary of Changes

### Files Modified

| File | Change Type | Description |
|------|-------------|-------------|
| `agent-diva-gui/src/components/ChatView.vue` | Modified | Added sidebar toggle, layout adjustment |
| `agent-diva-gui/src/components/NormalMode.vue` | Modified | Integrated sidebar, state management |
| `agent-diva-gui/src/styles.css` | Modified | Added conversation sidebar CSS |
| `agent-diva-gui/src/i18n/zh.json` | Modified | Added convSidebar keys |
| `agent-diva-gui/src/i18n/en.json` | Modified | Added convSidebar keys |

### Files Added

| File | Description |
|------|-------------|
| `agent-diva-gui/src/components/ConversationSidebar.vue` | New sidebar component |

---

## 9. Backend Requirements Summary

For the frontend to be fully functional, the backend needs to provide:

1. **Session list with extended metadata** (title, pinned, status, agent_icon)
2. **Search endpoint** for filtering sessions
3. **Pin/unpin endpoint** for session management
4. **Rename endpoint** for session title editing
5. **Status updates** via WebSocket or polling

Until these are implemented, the frontend will:
- Use existing session data (snippet, timestamp)
- Show placeholder icons
- Disable pin/rename actions with tooltips
- Display "功能待实现" for unimplemented features

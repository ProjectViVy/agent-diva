<script setup lang="ts">
import { ref, computed } from 'vue';
import {
  Search,
  Plus,
  Pin,
  PinOff,
  Trash2,
  Edit3,
  CheckCircle2,
  XCircle,
  Loader2,
  MessageSquare,
  X,
  RefreshCw,
  ChevronDown,
  ChevronRight,
  FolderOpen,
} from '@lucide/vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

interface Session {
  session_key: string;
  chat_id: string;
  snippet: string;
  timestamp: number;
  title?: string;
  last_message?: string;
  message_count: number;
  title_generated: boolean;
  title_manually_set: boolean;
  pinned?: boolean;
  status?: 'idle' | 'running' | 'completed' | 'error';
  agent_icon?: string;
  agent_name?: string;
  workspace_id?: string;
  channel?: string;
  kind?: 'root' | 'branch' | 'subagent' | 'ephemeral';
  root_session_key?: string | null;
  parent_session_key?: string | null;
  branch_label?: string | null;
  legacy?: boolean;
}

interface SessionTreeRow {
  type: 'workspace' | 'channel' | 'session';
  key: string;
  label?: string;
  depth?: number;
  hasChildren?: boolean;
  session?: Session;
}

const props = defineProps<{
  sessions: Session[];
  activeSessionKey: string;
  themeMode: string;
  workspaceRoot?: string;
}>();

const emit = defineEmits<{
  (e: 'select', sessionKey: string): void;
  (e: 'delete', sessionKey: string): void;
  (e: 'new'): void;
  (e: 'toggle-pin', sessionKey: string): void;
  (e: 'rename', sessionKey: string, newTitle: string): void;
  (e: 'refresh'): void;
  (e: 'close'): void;
}>();

const searchQuery = ref('');
const collapsedKeys = ref<string[]>([]);
const renamingId = ref<string | null>(null);
const renameInput = ref('');
const contextMenu = ref<{ visible: boolean; x: number; y: number; sessionId: string }>({
  visible: false,
  x: 0,
  y: 0,
  sessionId: '',
});

const workspaceName = computed(() => {
  const root = props.workspaceRoot?.trim();
  if (!root) return '当前工作区';
  const parts = root.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] || root;
});

const isCollapsed = (key: string) => collapsedKeys.value.includes(key);

const toggleCollapsed = (key: string) => {
  collapsedKeys.value = isCollapsed(key)
    ? collapsedKeys.value.filter((item) => item !== key)
    : [...collapsedKeys.value, key];
};

const sessionChannel = (session: Session) => {
  if (session.channel?.trim()) return session.channel.trim();
  const separator = session.session_key.indexOf(':');
  return separator > 0 ? session.session_key.slice(0, separator) : 'unknown';
};

const kindLabel = (session: Session) => {
  switch (session.kind) {
    case 'branch':
      return '分支';
    case 'subagent':
      return '子代理';
    case 'ephemeral':
      return '临时';
    default:
      return session.legacy ? '历史 root' : 'root';
  }
};

const sessionPathText = (session: Session, byKey: Map<string, Session>) => {
  const path: string[] = [];
  let current: Session | undefined = session;
  const seen = new Set<string>();
  while (current && !seen.has(current.session_key)) {
    seen.add(current.session_key);
    path.unshift(current.title || current.branch_label || current.session_key);
    current = current.parent_session_key ? byKey.get(current.parent_session_key) : undefined;
  }
  return path.join(' / ');
};

const filteredSessions = computed(() => {
  const all = props.sessions;
  const q = searchQuery.value.trim().toLowerCase();
  if (!q) return all;

  const byKey = new Map(all.map((session) => [session.session_key, session]));
  const visibleKeys = new Set<string>();
  for (const session of all) {
    const haystack = [
      session.title,
      session.last_message,
      session.snippet,
      session.branch_label,
      sessionChannel(session),
      kindLabel(session),
      session.legacy ? 'legacy 历史 root' : '',
      sessionPathText(session, byKey),
    ]
      .filter(Boolean)
      .join(' ')
      .toLowerCase();
    if (!haystack.includes(q)) continue;

    let current: Session | undefined = session;
    const seen = new Set<string>();
    while (current && !seen.has(current.session_key)) {
      seen.add(current.session_key);
      visibleKeys.add(current.session_key);
      current = current.parent_session_key ? byKey.get(current.parent_session_key) : undefined;
    }
  }
  return all.filter((session) => visibleKeys.has(session.session_key));
});

const treeRows = computed<SessionTreeRow[]>(() => {
  const groups = new Map<string, { workspaceKey: string; channel: string; sessions: Session[] }>();
  for (const session of filteredSessions.value) {
    const workspaceKey = session.workspace_id || '__active_workspace__';
    const channel = sessionChannel(session);
    const key = `${workspaceKey}::${channel}`;
    const group = groups.get(key) || { workspaceKey, channel, sessions: [] };
    group.sessions.push(session);
    groups.set(key, group);
  }

  const sortedGroups = [...groups.values()].sort((a, b) => {
    const aTime = Math.max(...a.sessions.map((session) => session.timestamp), 0);
    const bTime = Math.max(...b.sessions.map((session) => session.timestamp), 0);
    return bTime - aTime || a.channel.localeCompare(b.channel);
  });
  const rows: SessionTreeRow[] = [];
  let previousWorkspaceKey: string | null = null;

  if (sortedGroups.length === 0) {
    rows.push({
      type: 'workspace',
      key: 'workspace:__active_workspace__',
      label: workspaceName.value,
    });
  }

  for (const group of sortedGroups) {
    const workspaceCollapseKey = `workspace:${group.workspaceKey}`;
    if (previousWorkspaceKey !== group.workspaceKey) {
      rows.push({
        type: 'workspace',
        key: workspaceCollapseKey,
        label: workspaceName.value,
      });
      previousWorkspaceKey = group.workspaceKey;
    }
    if (isCollapsed(workspaceCollapseKey)) continue;

    const channelCollapseKey = `channel:${group.workspaceKey}:${group.channel}`;
    rows.push({ type: 'channel', key: channelCollapseKey, label: group.channel });
    if (isCollapsed(channelCollapseKey)) continue;

    const byKey = new Map(group.sessions.map((session) => [session.session_key, session]));
    const children = new Map<string, Session[]>();
    const roots: Session[] = [];
    for (const session of group.sessions) {
      const parent = session.parent_session_key ? byKey.get(session.parent_session_key) : undefined;
      if (parent) {
        const siblings = children.get(parent.session_key) || [];
        siblings.push(session);
        children.set(parent.session_key, siblings);
      } else {
        roots.push(session);
      }
    }
    const sortSessions = (sessions: Session[]) =>
      sessions.sort((a, b) => b.timestamp - a.timestamp || a.session_key.localeCompare(b.session_key));
    sortSessions(roots);

    const appendSession = (session: Session, depth: number) => {
      const childSessions = sortSessions(children.get(session.session_key) || []);
      const sessionKey = `session:${session.session_key}`;
      rows.push({
        type: 'session',
        key: sessionKey,
        depth,
        hasChildren: childSessions.length > 0,
        session,
      });
      if (!isCollapsed(sessionKey)) {
        for (const child of childSessions) appendSession(child, depth + 1);
      }
    };
    for (const root of roots) appendSession(root, 0);
  }
  return rows;
});

// Format time relative to now
const formatTimeAgo = (timestamp: number): string => {
  if (!Number.isFinite(timestamp) || timestamp <= 0) {
    return t('convSidebar.unknownTime');
  }
  const now = Date.now();
  const diff = now - timestamp;
  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (seconds < 60) return t('convSidebar.justNow');
  if (minutes < 60) return t('convSidebar.minutesAgo', { count: minutes });
  if (hours < 24) return t('convSidebar.hoursAgo', { count: hours });
  if (days < 7) return t('convSidebar.daysAgo', { count: days });

  return new Date(timestamp).toLocaleDateString();
};

// Get status icon component
const getStatusIcon = (status?: string) => {
  switch (status) {
    case 'running':
      return { component: Loader2, class: 'animate-spin text-amber-500' };
    case 'completed':
      return { component: CheckCircle2, class: 'text-green-500' };
    case 'error':
      return { component: XCircle, class: 'text-red-500' };
    default:
      return { component: MessageSquare, class: 'text-gray-400' };
  }
};

// Handle session click
const handleSessionClick = (sessionKey: string) => {
  if (renamingId.value === sessionKey) return;
  emit('select', sessionKey);
  closeContextMenu();
};

// Handle right-click context menu
const handleContextMenu = (e: MouseEvent, sessionKey: string) => {
  e.preventDefault();
  contextMenu.value = {
    visible: true,
    x: e.clientX,
    y: e.clientY,
    sessionId: sessionKey,
  };
};

// Close context menu
const closeContextMenu = () => {
  contextMenu.value.visible = false;
};

// Start renaming
const startRename = (session: Session) => {
  renamingId.value = session.session_key;
  renameInput.value = session.title || session.last_message || session.snippet || '';
  closeContextMenu();
};

// Confirm rename
const confirmRename = () => {
  if (renamingId.value && renameInput.value.trim()) {
    emit('rename', renamingId.value, renameInput.value.trim());
  }
  renamingId.value = null;
  renameInput.value = '';
};

// Cancel rename
const cancelRename = () => {
  renamingId.value = null;
  renameInput.value = '';
};

// Handle keydown in rename input
const handleRenameKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter') {
    e.preventDefault();
    confirmRename();
  } else if (e.key === 'Escape') {
    cancelRename();
  }
};

// Close context menu on outside click
const handleOutsideClick = () => {
  closeContextMenu();
};

// Expose close method for parent
defineExpose({ closeContextMenu });
</script>

<template>
  <aside
    class="conv-sidebar"
    @click="handleOutsideClick"
  >
    <!-- Header: Search + Actions -->
    <div class="conv-header">
      <div class="conv-search">
        <Search :size="14" class="conv-search-icon" />
        <input
          v-model="searchQuery"
          type="text"
          :placeholder="t('convSidebar.search')"
          class="conv-search-input"
        />
        <button
          v-if="searchQuery"
          @click="searchQuery = ''"
          class="conv-search-clear"
        >
          <X :size="12" />
        </button>
      </div>
      <div class="conv-header-actions">
        <!-- TODO: 仅显示置顶按钮目前意义不明确，暂时注释掉 -->
        <!-- <button
          @click="showPinnedOnly = !showPinnedOnly"
          class="conv-action-btn"
          :class="{ active: showPinnedOnly }"
          :title="showPinnedOnly ? t('convSidebar.showAll') : t('convSidebar.showPinned')"
        >
          <Pin v-if="showPinnedOnly" :size="14" />
          <PinOff v-else :size="14" />
        </button> -->
      </div>
    </div>

    <!-- New Session Button -->
    <button @click="emit('new')" class="conv-new-session">
      <Plus :size="14" />
      <span>{{ t('convSidebar.newSession') }}</span>
    </button>

    <!-- Session hierarchy: workspace -> channel -> root/child lineage -->
    <div class="conv-list">
      <template v-for="row in treeRows" :key="row.key">
        <button
          v-if="row.type === 'workspace'"
          type="button"
          class="conv-tree-workspace"
          @click="toggleCollapsed(row.key)"
        >
          <FolderOpen :size="13" />
          <span class="conv-tree-heading-label">工作区</span>
          <strong>{{ row.label }}</strong>
          <ChevronRight v-if="isCollapsed(row.key)" :size="13" />
          <ChevronDown v-else :size="13" />
        </button>

        <button
          v-else-if="row.type === 'channel'"
          type="button"
          class="conv-tree-channel"
          @click="toggleCollapsed(row.key)"
        >
          <MessageSquare :size="12" />
          <span>{{ row.label }}</span>
          <ChevronRight v-if="isCollapsed(row.key)" :size="12" />
          <ChevronDown v-else :size="12" />
        </button>

        <div
          v-else-if="row.session"
          class="conv-item conv-tree-session"
          :class="{ 'conv-item-active': row.session.session_key === activeSessionKey }"
          :style="{ '--conv-depth': row.depth || 0 }"
          @click="handleSessionClick(row.session.session_key)"
          @contextmenu="handleContextMenu($event, row.session.session_key)"
        >
          <div class="conv-tree-indent" aria-hidden="true" />
          <button
            v-if="row.hasChildren"
            type="button"
            class="conv-tree-toggle"
            :aria-label="isCollapsed(row.key) ? '展开子会话' : '折叠子会话'"
            @click.stop="toggleCollapsed(row.key)"
          >
            <ChevronRight v-if="isCollapsed(row.key)" :size="12" />
            <ChevronDown v-else :size="12" />
          </button>
          <span v-else class="conv-tree-toggle-spacer" aria-hidden="true" />

          <div class="conv-item-icon">
            <span>{{ row.session.agent_icon || (row.session.kind === 'subagent' ? '⚙️' : '💬') }}</span>
          </div>
          <div class="conv-item-body">
            <template v-if="renamingId === row.session.session_key">
              <input
                v-model="renameInput"
                @keydown="handleRenameKeydown"
                @blur="confirmRename"
                class="conv-rename-input"
                autofocus
              />
            </template>
            <template v-else>
              <div class="conv-item-title">
                <span>{{ row.session.title || t('convSidebar.untitled') }}</span>
                <span v-if="(row.session.kind && row.session.kind !== 'root') || row.session.legacy" class="conv-kind-badge">
                  {{ kindLabel(row.session) }}
                </span>
                <Pin v-if="row.session.pinned" :size="11" class="conv-pinned-indicator" />
              </div>
              <div class="conv-item-meta">
                <span class="conv-item-channel">{{ sessionChannel(row.session) }}</span>
                <span class="conv-item-preview">{{ row.session.last_message || row.session.snippet || t('convSidebar.untitled') }}</span>
                <span class="conv-item-time">{{ formatTimeAgo(row.session.timestamp) }}</span>
                <span class="conv-item-count">{{ row.session.message_count }}</span>
                <component
                  :is="getStatusIcon(row.session.status).component"
                  :size="12"
                  :class="getStatusIcon(row.session.status).class"
                />
              </div>
            </template>
          </div>
          <button
            @click.stop="emit('delete', row.session.session_key)"
            class="conv-item-delete"
            :title="t('convSidebar.delete')"
          >
            <Trash2 :size="12" />
          </button>
        </div>
      </template>

      <!-- Empty State -->
      <div v-if="filteredSessions.length === 0" class="conv-empty">
        <MessageSquare :size="32" class="conv-empty-icon" />
        <p>{{ searchQuery ? t('convSidebar.noResults') : t('convSidebar.noHistory') }}</p>
        <button
          v-if="!searchQuery"
          type="button"
          class="conv-empty-refresh"
          @click="emit('refresh')"
        >
          <RefreshCw :size="14" />
          <span>{{ t('convSidebar.refresh') }}</span>
        </button>
      </div>
    </div>

    <!-- Context Menu -->
    <Teleport to="body">
      <div
        v-if="contextMenu.visible"
        class="conv-context-menu"
        :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
        @click.stop
      >
        <button
          @click="startRename(sessions.find(s => s.session_key === contextMenu.sessionId)!)"
          class="conv-context-item"
        >
          <Edit3 :size="14" />
          <span>{{ t('convSidebar.rename') }}</span>
        </button>
        <button
          @click="emit('toggle-pin', contextMenu.sessionId)"
          class="conv-context-item"
        >
          <Pin v-if="!sessions.find(s => s.session_key === contextMenu.sessionId)?.pinned" :size="14" />
          <PinOff v-else :size="14" />
          <span>{{
            sessions.find(s => s.session_key === contextMenu.sessionId)?.pinned
              ? t('convSidebar.unpin')
              : t('convSidebar.pin')
          }}</span>
        </button>
        <div class="conv-context-divider"></div>
        <button
          @click="emit('delete', contextMenu.sessionId)"
          class="conv-context-item conv-context-danger"
        >
          <Trash2 :size="14" />
          <span>{{ t('convSidebar.delete') }}</span>
        </button>
      </div>
    </Teleport>
  </aside>
</template>

<style scoped>
/* Conversation Sidebar Container */
.conv-sidebar {
  width: 280px;
  border-left: 1px solid var(--conv-sidebar-border, #e5e7eb);
  background: var(--conv-sidebar-bg, #ffffff);
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

/* Header */
.conv-header {
  padding: 10px var(--space-3, 12px);
  border-bottom: 1px solid var(--line, #e5e7eb);
  display: flex;
  flex-direction: column;
  gap: var(--space-2, 8px);
}

/* Search */
.conv-search {
  position: relative;
  display: flex;
  align-items: center;
}

.conv-search-icon {
  position: absolute;
  left: 10px;
  color: var(--text-muted, #9ca3af);
  pointer-events: none;
}

.conv-search-input {
  width: 100%;
  padding: var(--space-2, 8px) var(--space-6, 32px) var(--space-2, 8px) var(--space-6, 32px);
  border-radius: var(--radius-sm, 8px);
  border: 1px solid var(--conv-search-border, #e5e7eb);
  background: var(--conv-search-bg, #ffffff);
  color: var(--conv-search-text, #111827);
  font-size: var(--font-size-sm, 13px);
  outline: none;
  transition: border-color 0.15s ease;
}

.conv-search-input:focus {
  border-color: var(--brand, #ec4899);
}

.conv-search-input::placeholder {
  color: var(--text-muted, #9ca3af);
}

.conv-search-clear {
  position: absolute;
  right: 8px;
  padding: var(--space-1, 4px);
  border: none;
  background: transparent;
  color: var(--text-muted, #9ca3af);
  cursor: pointer;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.conv-search-clear:hover {
  background: var(--nav-hover, rgba(0, 0, 0, 0.04));
}

/* Header Actions */
.conv-header-actions {
  display: flex;
  justify-content: flex-end;
}

.conv-action-btn {
  padding: 6px var(--space-2, 8px);
  border-radius: var(--radius-sm, 8px);
  border: none;
  background: transparent;
  color: var(--text-muted, #9ca3af);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.conv-action-btn:hover {
  background: var(--nav-hover, rgba(0, 0, 0, 0.04));
  color: var(--text, #111827);
}

.conv-action-btn.active {
  background: var(--nav-active, rgba(0, 0, 0, 0.06));
  color: var(--brand, #ec4899);
}

/* New Session Button */
.conv-new-session {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2, 8px);
  padding: 10px var(--space-3, 12px);
  margin: var(--space-2, 8px) var(--space-3, 12px);
  border-radius: var(--radius-sm, 8px);
  border: 1px dashed var(--conv-new-border, #e5e7eb);
  background: transparent;
  color: var(--conv-new-text, #9ca3af);
  font-size: var(--font-size-sm, 13px);
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.conv-new-session:hover {
  border-color: var(--brand, #ec4899);
  color: var(--brand, #ec4899);
  background: var(--conv-item-hover-bg, rgba(0, 0, 0, 0.04));
}

/* Session List */
.conv-list {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: var(--space-1, 4px) var(--space-2, 8px);
}

/* Section */
.conv-section {
  margin-bottom: 8px;
}

.conv-section-label {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: var(--space-2, 8px) 10px var(--space-1, 4px);
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-muted, #9ca3af);
}

/* Workspace/channel lineage headers */
.conv-tree-workspace,
.conv-tree-channel {
  display: flex;
  align-items: center;
  width: 100%;
  border: 0;
  background: transparent;
  color: var(--text-muted, #64748b);
  text-align: left;
  cursor: pointer;
}

.conv-tree-workspace {
  gap: 6px;
  padding: 10px 10px 7px;
  color: var(--text, #334155);
  font-size: 11px;
}

.conv-tree-workspace strong {
  min-width: 0;
  overflow: hidden;
  color: var(--accent, #db2777);
  font-size: 11px;
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.conv-tree-heading-label {
  color: var(--text-muted, #94a3b8);
  font-size: 10px;
}

.conv-tree-workspace svg:last-child,
.conv-tree-channel svg:last-child {
  margin-left: auto;
  flex-shrink: 0;
}

.conv-tree-channel {
  gap: 6px;
  padding: 6px 10px 4px 24px;
  color: var(--text-muted, #64748b);
  font-size: 10px;
  font-weight: 650;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.conv-tree-channel:hover,
.conv-tree-workspace:hover {
  color: var(--accent, #db2777);
}

.conv-tree-session {
  padding-left: calc(10px + (var(--conv-depth, 0) * 14px));
}

.conv-tree-indent {
  width: 0;
  flex: 0 0 auto;
}

.conv-tree-toggle,
.conv-tree-toggle-spacer {
  display: grid;
  place-items: center;
  width: 14px;
  height: 20px;
  flex: 0 0 14px;
  color: var(--text-muted, #94a3b8);
}

.conv-tree-toggle {
  padding: 0;
  border: 0;
  background: transparent;
  cursor: pointer;
}

.conv-tree-toggle:hover {
  color: var(--accent, #db2777);
}

.conv-item-channel {
  flex: 0 0 auto;
  color: var(--text-muted, #94a3b8);
  font-size: 10px;
  text-transform: uppercase;
}

.conv-kind-badge {
  display: inline-flex;
  align-items: center;
  margin-left: 5px;
  padding: 1px 4px;
  border-radius: 4px;
  background: var(--nav-hover, rgba(148, 163, 184, 0.12));
  color: var(--text-muted, #64748b);
  font-size: 9px;
  font-weight: 500;
  vertical-align: middle;
}

.conv-pinned-indicator {
  margin-left: 4px;
  color: #f59e0b;
  vertical-align: middle;
}

/* Session Item */
.conv-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: var(--space-2, 8px) 10px;
  border-radius: var(--radius-sm, 8px);
  cursor: pointer;
  transition: background 0.12s ease;
  position: relative;
}

.conv-item:hover {
  background: var(--conv-item-hover-bg, rgba(0, 0, 0, 0.04));
}

.conv-item-active {
  background: var(--conv-item-active-bg, rgba(0, 0, 0, 0.06)) !important;
}

/* Item Icon */
.conv-item-icon {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  border: 1px solid var(--conv-icon-border, #e5e7eb);
  background: var(--conv-icon-bg, #ffffff);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  font-size: var(--font-size-base, 14px);
}

/* Item Body */
.conv-item-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.conv-item-title {
  font-size: var(--font-size-sm, 13px);
  font-weight: 500;
  color: var(--text, #111827);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.conv-item-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--text-muted, #9ca3af);
}

.conv-item-preview {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.conv-item-time {
  flex-shrink: 0;
}

.conv-item-count {
  opacity: 0.6;
  font-size: 10px;
  flex-shrink: 0;
}

/* Rename Input */
.conv-rename-input {
  width: 100%;
  padding: var(--space-1, 4px) 6px;
  border-radius: 4px;
  border: 1px solid var(--brand, #ec4899);
  background: var(--panel-solid, #ffffff);
  color: var(--text, #111827);
  font-size: var(--font-size-sm, 13px);
  outline: none;
}

/* Delete Button */
.conv-item-delete {
  padding: var(--space-1, 4px);
  border: none;
  background: transparent;
  color: var(--text-muted, #9ca3af);
  cursor: pointer;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.conv-item:hover .conv-item-delete {
  opacity: 0.6;
}

.conv-item-delete:hover {
  opacity: 1 !important;
  background: var(--danger-bg, rgba(239, 68, 68, 0.1));
  color: var(--danger, #ef4444);
}

/* Empty State */
.conv-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  color: var(--text-muted, #9ca3af);
  text-align: center;
}

.conv-empty-icon {
  margin-bottom: 12px;
  opacity: 0.4;
}

.conv-empty p {
  font-size: var(--font-size-sm, 13px);
  margin: 0;
}

.conv-empty-refresh {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-top: 12px;
  padding: 6px 10px;
  border: 1px solid var(--line, #e5e7eb);
  border-radius: var(--radius-sm, 8px);
  color: var(--text, #374151);
  background: var(--panel-solid, #ffffff);
  cursor: pointer;
}

.conv-empty-refresh:hover {
  border-color: var(--brand, #ec4899);
  color: var(--brand, #ec4899);
}

/* Context Menu */
.conv-context-menu {
  position: fixed;
  min-width: 160px;
  background: var(--conv-menu-bg, #ffffff);
  border: 1px solid var(--conv-menu-border, #e5e7eb);
  border-radius: var(--radius-sm, 8px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
  padding: 6px;
  z-index: 1000;
}

.conv-context-item {
  display: flex;
  align-items: center;
  gap: var(--space-2, 8px);
  width: 100%;
  padding: var(--space-2, 8px) 10px;
  border: none;
  background: transparent;
  color: var(--text, #111827);
  font-size: var(--font-size-sm, 13px);
  cursor: pointer;
  border-radius: 6px;
  transition: background 0.12s ease;
  text-align: left;
}

.conv-context-item:hover {
  background: var(--nav-hover, rgba(0, 0, 0, 0.04));
}

.conv-context-danger {
  color: var(--danger, #ef4444);
}

.conv-context-danger:hover {
  background: var(--danger-bg, rgba(239, 68, 68, 0.08));
}

.conv-context-divider {
  height: 1px;
  background: var(--line, #e5e7eb);
  margin: var(--space-1, 4px) 0;
}
</style>

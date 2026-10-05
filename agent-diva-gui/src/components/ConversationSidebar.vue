<script setup lang="ts">
import { ref, computed } from 'vue';
import { Search, Plus, Pin, PinOff, Trash2, Edit3, CheckCircle2, XCircle, Loader2, MessageSquare, X, RefreshCw } from '@lucide/vue';
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
}

const props = defineProps<{
  sessions: Session[];
  activeSessionKey: string;
  themeMode: string;
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
const showPinnedOnly = ref(false);
const renamingId = ref<string | null>(null);
const renameInput = ref('');
const contextMenu = ref<{ visible: boolean; x: number; y: number; sessionId: string }>({
  visible: false,
  x: 0,
  y: 0,
  sessionId: '',
});

// Filter sessions by search query
const filteredSessions = computed(() => {
  const q = searchQuery.value.trim().toLowerCase();
  if (!q) return props.sessions;
  return props.sessions.filter((s) => {
    const haystack = `${s.title || ''} ${s.last_message || s.snippet || ''}`.toLowerCase();
    return haystack.includes(q);
  });
});
// Separate pinned and regular sessions
const pinnedSessions = computed(() =>
  filteredSessions.value
    .filter((s) => s.pinned)
    .sort((a, b) => b.timestamp - a.timestamp)
);

const regularSessions = computed(() =>
  filteredSessions.value
    .filter((s) => !s.pinned)
    .sort((a, b) => b.timestamp - a.timestamp)
);

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
      return { component: Loader2, class: 'animate-spin text-warning' };
    case 'completed':
      return { component: CheckCircle2, class: 'text-success' };
    case 'error':
      return { component: XCircle, class: 'text-destructive' };
    default:
      return { component: MessageSquare, class: 'text-muted-foreground' };
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
    :class="`theme-${themeMode}`"
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
        class="ui-input ui-input--leading-icon ui-input--trailing-icon conv-search-input"
        />
        <button
          v-if="searchQuery"
          @click="searchQuery = ''"
          class="ui-button ui-button--ghost ui-button--compact conv-search-clear"
        >
          <X :size="12" />
        </button>
      </div>
      <div class="conv-header-actions">
        <!-- TODO: 仅显示置顶按钮目前意义不明确，暂时注释掉 -->
        <!-- <button
          @click="showPinnedOnly = !showPinnedOnly"
          class="ui-button ui-button--ghost ui-button--compact conv-action-btn"
          :class="{ active: showPinnedOnly }"
          :title="showPinnedOnly ? t('convSidebar.showAll') : t('convSidebar.showPinned')"
        >
          <Pin v-if="showPinnedOnly" :size="14" />
          <PinOff v-else :size="14" />
        </button> -->
      </div>
    </div>

    <!-- New Session Button -->
    <button @click="emit('new')" class="ui-button ui-button--ghost ui-button--compact conv-new-session">
      <Plus :size="14" />
      <span>{{ t('convSidebar.newSession') }}</span>
    </button>

    <!-- Session List -->
    <div class="conv-list">
      <!-- Pinned Section -->
      <div v-if="!showPinnedOnly || pinnedSessions.length > 0" class="conv-section">
        <div v-if="pinnedSessions.length > 0" class="conv-section-label">
          <Pin :size="12" class="text-warning" />
          <span>{{ t('convSidebar.pinned') }}</span>
        </div>
        <div
          v-for="session in pinnedSessions"
          :key="session.session_key"
          class="conv-item"
          :class="{ 'conv-item-active': session.session_key === activeSessionKey }"
          @click="handleSessionClick(session.session_key)"
          @contextmenu="handleContextMenu($event, session.session_key)"
        >
          <!-- Icon -->
          <div class="conv-item-icon">
            <span>{{ session.agent_icon || '💬' }}</span>
          </div>
          <!-- Body -->
          <div class="conv-item-body">
            <!-- Renaming Mode -->
            <template v-if="renamingId === session.session_key">
              <input
                v-model="renameInput"
                @keydown="handleRenameKeydown"
                @blur="confirmRename"
                class="ui-input conv-rename-input"
                autofocus
              />
            </template>
            <!-- Normal Mode -->
            <template v-else>
              <div class="conv-item-title">{{ session.title || t('convSidebar.untitled') }}</div>
              <div class="conv-item-meta">
                <span class="conv-item-preview">{{ session.last_message || session.snippet || t('convSidebar.untitled') }}</span>
                <span class="conv-item-time">{{ formatTimeAgo(session.timestamp) }}</span>
                <span class="conv-item-count">{{ session.message_count }}</span>
                <component
                  :is="getStatusIcon(session.status).component"
                  :size="12"
                  :class="getStatusIcon(session.status).class"
                />
              </div>
            </template>
          </div>
          <!-- Delete Button (hover reveal) -->
          <button
            @click.stop="emit('delete', session.session_key)"
            class="ui-button ui-button--ghost ui-button--compact conv-item-delete"
            :title="t('convSidebar.delete')"
          >
            <Trash2 :size="12" />
          </button>
        </div>
      </div>

      <!-- Regular Section -->
      <div v-if="!showPinnedOnly" class="conv-section">
        <div v-if="regularSessions.length > 0 || pinnedSessions.length > 0" class="conv-section-label">
          <MessageSquare :size="12" />
          <span>{{ t('convSidebar.sessions') }}</span>
        </div>
        <div
          v-for="session in regularSessions"
          :key="session.session_key"
          class="conv-item"
          :class="{ 'conv-item-active': session.session_key === activeSessionKey }"
          @click="handleSessionClick(session.session_key)"
          @contextmenu="handleContextMenu($event, session.session_key)"
        >
          <!-- Icon -->
          <div class="conv-item-icon">
            <span>{{ session.agent_icon || '💬' }}</span>
          </div>
          <!-- Body -->
          <div class="conv-item-body">
            <!-- Renaming Mode -->
            <template v-if="renamingId === session.session_key">
              <input
                v-model="renameInput"
                @keydown="handleRenameKeydown"
                @blur="confirmRename"
                class="ui-input conv-rename-input"
                autofocus
              />
            </template>
            <!-- Normal Mode -->
            <template v-else>
              <div class="conv-item-title">{{ session.title || t('convSidebar.untitled') }}</div>
              <div class="conv-item-meta">
                <span class="conv-item-preview">{{ session.last_message || session.snippet || t('convSidebar.untitled') }}</span>
                <span class="conv-item-time">{{ formatTimeAgo(session.timestamp) }}</span>
                <span class="conv-item-count">{{ session.message_count }}</span>
                <component
                  :is="getStatusIcon(session.status).component"
                  :size="12"
                  :class="getStatusIcon(session.status).class"
                />
              </div>
            </template>
          </div>
          <!-- Delete Button (hover reveal) -->
          <button
            @click.stop="emit('delete', session.session_key)"
            class="ui-button ui-button--ghost ui-button--compact conv-item-delete"
            :title="t('convSidebar.delete')"
          >
            <Trash2 :size="12" />
          </button>
        </div>
      </div>

      <!-- Empty State -->
      <div v-if="filteredSessions.length === 0" class="conv-empty">
        <MessageSquare :size="32" class="conv-empty-icon" />
        <p>{{ searchQuery ? t('convSidebar.noResults') : t('convSidebar.noHistory') }}</p>
        <button
          v-if="!searchQuery"
          type="button"
          class="ui-button ui-button--ghost ui-button--compact conv-empty-refresh"
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
          class="ui-button ui-button--ghost ui-button--compact conv-context-item"
        >
          <Edit3 :size="14" />
          <span>{{ t('convSidebar.rename') }}</span>
        </button>
        <button
          @click="emit('toggle-pin', contextMenu.sessionId)"
          class="ui-button ui-button--ghost ui-button--compact conv-context-item"
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
          class="ui-button ui-button--ghost ui-button--compact conv-context-item conv-context-danger"
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
  border-left: 1px solid var(--border);
  background: var(--card);
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

/* Header */
.conv-header {
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 8px;
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
  color: var(--muted-foreground);
  pointer-events: none;
}

.conv-search-input {
  width: 100%;
  transition: border-color 0.15s ease;
}

.conv-search-clear {
  position: absolute;
  right: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* Header Actions */
.conv-header-actions {
  display: flex;
  justify-content: flex-end;
}

.conv-action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.conv-action-btn.active {
  background: var(--sidebar-accent);
  color: var(--primary);
}

/* New Session Button */
.conv-new-session {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin: 8px 12px;
  transition: all 0.15s ease;
}

/* Session List */
.conv-list {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 4px 8px;
}

/* Section */
.conv-section {
  margin-bottom: 8px;
}

.conv-section-label {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 10px 4px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--muted-foreground);
}

/* Session Item */
.conv-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background 0.12s ease;
  position: relative;
}

.conv-item:hover {
  background: var(--accent);
}

.conv-item-active {
  background: var(--sidebar-accent) !important;
}

/* Item Icon */
.conv-item-icon {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--card);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  font-size: 14px;
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
  font-size: 13px;
  font-weight: 500;
  color: var(--foreground);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.conv-item-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--muted-foreground);
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
}

/* Delete Button */
.conv-item-delete {
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
}

/* Empty State */
.conv-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  color: var(--muted-foreground);
  text-align: center;
}

.conv-empty-icon {
  margin-bottom: 12px;
  opacity: 0.4;
}

.conv-empty p {
  font-size: 13px;
  margin: 0;
}

.conv-empty-refresh {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-top: 12px;
}

/* Context Menu */
.conv-context-menu {
  position: fixed;
  min-width: 160px;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-lg);
  padding: 6px;
  z-index: 1000;
}

.conv-context-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  transition: background 0.12s ease;
  text-align: left;
}

.conv-context-divider {
  height: 1px;
  background: var(--border);
  margin: 4px 0;
}

/* ========================================
   Theme Overrides
   ======================================== */

/* Dark Theme */
:root[data-theme="dark"] .conv-sidebar,
.theme-dark .conv-sidebar {
  border-left-color: var(--border);
  background: var(--card);
}

.theme-dark .conv-item-icon {
  border-color: var(--border);
  background: var(--card);
}

.theme-dark .conv-context-menu {
  background: var(--card);
  border-color: var(--border);
}

/* Love Theme */
.theme-love .conv-sidebar {
  border-left-color: var(--border-strong);
  background: var(--card);
}

.theme-love .conv-item:hover {
  background: var(--destructive-soft);
}

.theme-love .conv-item-active {
  background: var(--destructive-soft);
}

.theme-love .conv-item-icon {
  border-color: var(--border-strong);
  background: var(--card);
}

.theme-love .conv-context-menu {
  background: var(--card);
  border-color: var(--border-strong);
}

/* Default Theme */
.theme-default .conv-sidebar {
  border-left-color: var(--border);
  background: var(--card);
}

/* Miku Theme */
.theme-miku .conv-sidebar {
  border-left-color: var(--border-strong);
  background: var(--card);
}

.theme-miku .conv-item:hover {
  background: var(--accent);
}

.theme-miku .conv-item-active {
  background: var(--accent);
}

.theme-miku .conv-item-icon {
  border-color: var(--border-strong);
  background: var(--card);
}

.theme-miku .conv-context-menu {
  background: var(--card);
  border-color: var(--border-strong);
}
</style>

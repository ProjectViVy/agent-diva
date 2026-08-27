<script setup lang="ts">
import { computed, ref } from 'vue';
import { ChevronDown, CircleAlert, FileText, FolderOpen, RefreshCw, Settings2 } from '@lucide/vue';
import type { WorkspaceContextState } from '../composables/useWorkspaceContext';
import type { WorkspaceStatus } from '../api/desktop';

const props = withDefaults(defineProps<{
  workspace: WorkspaceStatus | null;
  state: WorkspaceContextState;
  error?: string | null;
  placement?: 'above' | 'below';
}>(), {
  placement: 'below',
});

const emit = defineEmits<{
  (event: 'open-settings'): void;
  (event: 'refresh'): void;
}>();

const open = ref(false);

const workspaceName = computed(() => {
  if (!props.workspace?.root) return '工作区加载中';
  if (props.workspace.source === 'process-cwd' || props.workspace.source === 'legacy-default') {
    return '默认工作区';
  }
  const parts = props.workspace.root.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] || props.workspace.root;
});

const agentsLabel = computed(() => {
  if (props.workspace?.agents_md?.present) return 'AGENTS.md 已加载';
  if (props.workspace) return '未发现 AGENTS.md';
  return 'AGENTS 状态待加载';
});

function close() {
  open.value = false;
}

function openSettings() {
  close();
  emit('open-settings');
}
</script>

<template>
  <div class="relative z-40">
    <button
      type="button"
      class="workspace-chip no-drag"
      :class="{ 'workspace-chip-open': open }"
      data-testid="workspace-chip"
      :title="workspace?.root || '工作区状态加载中'"
      aria-haspopup="dialog"
      :aria-expanded="open"
      @click="open = !open"
    >
      <FolderOpen :size="13" />
      <span class="workspace-chip-name">{{ workspaceName }}</span>
      <span
        class="workspace-chip-dot"
        :class="{
          'workspace-chip-dot-ready': state === 'ready',
          'workspace-chip-dot-error': state === 'error',
        }"
      />
      <ChevronDown :size="12" class="workspace-chip-chevron" :class="{ 'rotate-180': open }" />
    </button>

    <div v-if="open" class="fixed inset-0 z-30" aria-hidden="true" @click="close" />
    <section
      v-if="open"
      class="workspace-popover absolute left-0 z-40 w-[min(360px,calc(100vw-32px))]"
      :class="placement === 'above' ? 'workspace-popover-above' : 'workspace-popover-below'"
      role="dialog"
      aria-label="当前工作区"
      data-testid="workspace-popover"
    >
      <div class="workspace-popover-heading">
        <div>
          <p class="workspace-popover-eyebrow">当前工作区</p>
          <p class="workspace-popover-title">{{ workspaceName }}</p>
        </div>
        <CircleAlert v-if="state === 'error'" :size="16" class="text-amber-500" />
      </div>

      <div class="workspace-popover-path">{{ workspace?.root || error || '正在读取运行时工作区…' }}</div>

      <div class="workspace-popover-status">
        <FileText :size="14" />
        <span>{{ agentsLabel }}</span>
        <span v-if="workspace?.agents_md?.truncated" class="workspace-status-note">已截断</span>
      </div>
      <div v-if="workspace" class="workspace-popover-meta">
        来源：{{ workspace.source }}
      </div>

      <div class="workspace-popover-actions">
        <button type="button" class="workspace-popover-action" @click="emit('refresh')">
          <RefreshCw :size="13" :class="{ 'animate-spin': state === 'loading' || state === 'refreshing' }" />
          刷新状态
        </button>
        <button type="button" class="workspace-popover-action workspace-popover-primary" @click="openSettings">
          <Settings2 :size="13" />
          切换工作区
        </button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.workspace-chip {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  max-width: 190px;
  height: 30px;
  padding: 0 0.6rem;
  border: 1px solid var(--line, rgba(148, 163, 184, 0.28));
  border-radius: 0.65rem;
  background: var(--surface, rgba(255, 255, 255, 0.7));
  color: var(--text-muted, #64748b);
  font-size: 0.7rem;
  transition: border-color 0.15s ease, background-color 0.15s ease, color 0.15s ease;
}

.workspace-chip:hover,
.workspace-chip-open {
  border-color: var(--accent, #ec4899);
  background: var(--surface-raised, rgba(255, 255, 255, 0.92));
  color: var(--text, #334155);
}

.workspace-chip-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.workspace-chip-dot {
  width: 6px;
  height: 6px;
  border-radius: 999px;
  background: #cbd5e1;
}

.workspace-chip-dot-ready { background: #22c55e; }
.workspace-chip-dot-error { background: #f59e0b; }
.workspace-chip-chevron { flex: 0 0 auto; transition: transform 0.15s ease; }

.workspace-popover {
  padding: 0.85rem;
  border: 1px solid var(--line, rgba(148, 163, 184, 0.28));
  border-radius: 0.85rem;
  background: var(--surface-raised, rgba(255, 255, 255, 0.97));
  box-shadow: 0 16px 38px rgba(15, 23, 42, 0.16);
}

.workspace-popover-above { bottom: calc(100% + 8px); }
.workspace-popover-below { top: calc(100% + 8px); }

.workspace-popover-heading,
.workspace-popover-status,
.workspace-popover-actions {
  display: flex;
  align-items: center;
}

.workspace-popover-heading { justify-content: space-between; gap: 0.75rem; }
.workspace-popover-eyebrow { margin: 0; color: var(--text-muted, #64748b); font-size: 0.65rem; }
.workspace-popover-title { margin: 0.15rem 0 0; color: var(--text, #1e293b); font-size: 0.8rem; font-weight: 600; }
.workspace-popover-path { margin-top: 0.75rem; color: var(--text-muted, #64748b); font-family: ui-monospace, SFMono-Regular, monospace; font-size: 0.65rem; line-height: 1.35; overflow-wrap: anywhere; }
.workspace-popover-status { gap: 0.4rem; margin-top: 0.75rem; color: var(--text, #334155); font-size: 0.7rem; }
.workspace-status-note { margin-left: auto; color: #b45309; font-size: 0.62rem; }
.workspace-popover-meta { margin-top: 0.35rem; color: var(--text-muted, #64748b); font-size: 0.62rem; }
.workspace-popover-actions { gap: 0.45rem; margin-top: 0.85rem; }
.workspace-popover-action { display: inline-flex; align-items: center; gap: 0.35rem; padding: 0.42rem 0.55rem; border-radius: 0.5rem; color: var(--text-muted, #64748b); font-size: 0.65rem; }
.workspace-popover-action:hover { background: var(--nav-hover, rgba(148, 163, 184, 0.12)); color: var(--text, #334155); }
.workspace-popover-primary { margin-left: auto; color: var(--accent, #db2777); }
</style>

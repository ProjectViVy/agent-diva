<script setup lang="ts">
/**
 * SubAgentPanel — displays child agent lifecycle events.
 *
 * Shows a list of active/completed sub-agents with status icons,
 * summaries, and elapsed time. Uses polling via useSubAgents composable.
 */
import { onMounted } from 'vue';
import { Bot } from 'lucide-vue-next';
import { useSubAgents } from '../composables/useSubAgents';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const {
  children,
  completedChildren,
  totalCount,
  polling,
  startPolling,
  formatElapsed,
  statusIcon,
} = useSubAgents(3000);

onMounted(() => {
  startPolling();
});
</script>

<template>
  <div class="subagent-panel">
    <!-- Header -->
    <div class="subagent-panel-header">
      <div class="subagent-panel-title">
        <Bot :size="14" />
        <span>{{ t('subagent.title', '子代理') }}</span>
        <span v-if="totalCount > 0" class="subagent-badge">{{ totalCount }}</span>
      </div>
      <span v-if="polling" class="subagent-polling-dot" :title="t('subagent.polling', '轮询中')" />
    </div>

    <!-- Empty state -->
    <div v-if="children.length === 0" class="subagent-empty">
      <div class="subagent-empty-icon">🤖</div>
      <span class="subagent-empty-text">{{ t('subagent.empty', '暂无子代理任务') }}</span>
    </div>

    <!-- Child list -->
    <div v-else class="subagent-list">
      <div
        v-for="child in completedChildren"
        :key="child.task_id"
        class="subagent-item"
        :class="`subagent-status-${child.status}`"
      >
        <div class="subagent-item-row">
          <span class="subagent-status-icon">{{ statusIcon(child.status) }}</span>
          <span class="subagent-task-id">{{ child.task_id }}</span>
          <span class="subagent-elapsed">{{ formatElapsed(child.elapsed_ms) }}</span>
        </div>
        <div v-if="child.summary" class="subagent-summary">{{ child.summary }}</div>
        <div class="subagent-meta">
          <span>{{ child.tool_call_count }} {{ t('subagent.toolCalls', '工具调用') }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.subagent-panel {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  background: var(--panel-solid, #fff);
  color: var(--text, #1a1a1a);
  overflow: hidden;
}

/* ── Header ── */
.subagent-panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
  border-bottom: 1px solid var(--line, #e5e7eb);
  flex-shrink: 0;
}

.subagent-panel-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text, #1a1a1a);
}

.subagent-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  border-radius: 9px;
  background: var(--accent-bg-light, #fce7f3);
  color: var(--accent, #ec4899);
  font-size: 10px;
  font-weight: 700;
  line-height: 1;
}

.subagent-polling-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #22c55e;
  animation: pulse-dot 1.5s ease-in-out infinite;
}

@keyframes pulse-dot {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

/* ── Empty state ── */
.subagent-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 40px 16px;
  flex: 1;
}

.subagent-empty-icon {
  font-size: 32px;
  opacity: 0.4;
}

.subagent-empty-text {
  font-size: 12px;
  color: var(--text-muted, #9ca3af);
}

/* ── List ── */
.subagent-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px;
}

/* ── Item ── */
.subagent-item {
  padding: 10px 12px;
  border-radius: var(--radius-sm, 6px);
  margin-bottom: 2px;
  transition: background 0.15s ease;
}

.subagent-item:hover {
  background: var(--accent-bg-light, #fdf2f8);
}

.subagent-item-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.subagent-status-icon {
  font-size: 14px;
  line-height: 1;
  flex-shrink: 0;
}

.subagent-task-id {
  font-size: 12px;
  font-weight: 600;
  font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
  color: var(--text, #1a1a1a);
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.subagent-elapsed {
  font-size: 11px;
  color: var(--text-muted, #9ca3af);
  font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
  flex-shrink: 0;
}

.subagent-summary {
  font-size: 12px;
  color: var(--text-muted, #6b7280);
  margin-top: 4px;
  padding-left: 22px;
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.subagent-meta {
  font-size: 10px;
  color: var(--text-muted, #9ca3af);
  margin-top: 4px;
  padding-left: 22px;
}

/* ── Status accent (left border) ── */
.subagent-status-ok {
  border-left: 3px solid #22c55e;
}

.subagent-status-error {
  border-left: 3px solid #ef4444;
}

.subagent-status-timeout {
  border-left: 3px solid #f59e0b;
}

.subagent-status-cancelled {
  border-left: 3px solid #6b7280;
}
</style>

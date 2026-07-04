<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { Copy, Activity } from 'lucide-vue-next';
import { showAppToast } from '../../../utils/appToast';

const { t } = useI18n();

const props = defineProps<{
  lines: string[];
  loading: boolean;
}>();

const autoRefresh = ref(false);
let pollTimer: ReturnType<typeof setInterval> | null = null;

const emit = defineEmits<{
  (e: 'refresh'): void;
}>();

function toggleAutoRefresh() {
  autoRefresh.value = !autoRefresh.value;
}

watch(autoRefresh, (val: boolean) => {
  if (val) {
    pollTimer = setInterval(() => {
      emit('refresh');
    }, 5000);
  } else if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
});

onUnmounted(() => {
  if (pollTimer) {
    clearInterval(pollTimer);
  }
});

function isAuditLine(line: string): boolean {
  return line.includes('"target":"audit"') || line.includes('"target": "audit"');
}

async function copyLine(content: string) {
  try {
    await navigator.clipboard.writeText(content);
    showAppToast(t('auditPage.raw.copySuccess'), 'success');
  } catch {
    showAppToast(t('auditPage.raw.copyFailed'), 'error');
  }
}

function formatLineNumber(n: number): string {
  return String(n).padStart(4, ' ');
}
</script>

<template>
  <div
    class="raw-log-tab"
    role="tabpanel"
    :aria-label="t('auditPage.raw.panelLabel')"
    :aria-busy="loading"
  >
    <div class="log-toolbar">
      <button
        class="log-toolbar-btn"
        :class="{ active: autoRefresh }"
        :aria-pressed="autoRefresh"
        @click="toggleAutoRefresh"
        @keydown.enter.prevent="toggleAutoRefresh"
        @keydown.space.prevent="toggleAutoRefresh"
      >
        <Activity :size="14" aria-hidden="true" />
        <span>{{ autoRefresh ? t('auditPage.raw.autoRefreshOn') : t('auditPage.raw.autoRefreshOff') }}</span>
      </button>
      <span v-if="autoRefresh" class="log-live-badge" role="status">{{ t('auditPage.raw.live') }}</span>
    </div>

    <div v-if="loading && lines.length === 0" class="skeleton-container" aria-hidden="true">
      <div v-for="i in 8" :key="i" class="skeleton-log-row">
        <div class="skeleton-line-num" />
        <div class="skeleton-line-content" />
      </div>
    </div>

    <div
      v-else-if="!loading && lines.length === 0"
      class="empty-state"
      role="status"
    >
      <span class="empty-icon" aria-hidden="true">馃搫</span>
      <p class="empty-title">{{ t('auditPage.raw.emptyTitle') }}</p>
      <p class="empty-hint">{{ t('auditPage.raw.emptyHint') }}</p>
    </div>

    <div v-else class="log-lines-container">
      <div
        v-for="(line, idx) in lines"
        :key="idx"
        class="log-line"
        :class="{ 'log-line-audit': isAuditLine(line) }"
        role="row"
      >
        <span class="log-line-num" aria-hidden="true">{{ formatLineNumber(idx + 1) }}</span>
        <code class="log-line-content">{{ line }}</code>
        <button
          class="log-line-copy"
          :aria-label="t('auditPage.raw.copyLineLabel', { line: idx + 1 })"
          :title="t('auditPage.raw.copyLineLabel', { line: idx + 1 })"
          @click="copyLine(line)"
          @keydown.enter.prevent="copyLine(line)"
          @keydown.space.prevent="copyLine(line)"
        >
          <Copy :size="12" aria-hidden="true" />
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.raw-log-tab {
  min-height: 200px;
}

.log-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.log-toolbar-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: 1px solid var(--line, #e5e7eb);
  border-radius: var(--radius-sm, 8px);
  background: var(--panel, #ffffff);
  color: var(--text-muted, #6b7280);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  outline: none;
}

.log-toolbar-btn:hover {
  border-color: var(--accent, #3b82f6);
  color: var(--text, #111827);
}

.log-toolbar-btn:focus-visible {
  outline: 2px solid var(--accent, #3b82f6);
  outline-offset: 2px;
}

.log-toolbar-btn.active {
  background: var(--accent-bg-light, rgba(59,130,246,0.08));
  border-color: var(--accent, #3b82f6);
  color: var(--accent, #3b82f6);
}

.log-live-badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: 10px;
  background: var(--danger-bg, rgba(239,68,68,0.1));
  color: var(--danger, #ef4444);
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  animation: live-pulse 2s ease-in-out infinite;
}

@keyframes live-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}

.skeleton-container {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.skeleton-log-row {
  display: grid;
  grid-template-columns: 40px 1fr;
  gap: 8px;
  align-items: center;
  padding: 6px 8px;
}

.skeleton-line-num {
  height: 12px;
  width: 30px;
  border-radius: 3px;
  background: var(--line, #e5e7eb);
  animation: skeleton-pulse 1.5s ease-in-out infinite;
}

.skeleton-line-content {
  height: 12px;
  border-radius: 3px;
  background: var(--line, #e5e7eb);
  animation: skeleton-pulse 1.5s ease-in-out infinite;
}

@keyframes skeleton-pulse {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 0.8; }
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 64px 24px;
  text-align: center;
}

.empty-icon {
  font-size: 40px;
  margin-bottom: 16px;
  opacity: 0.5;
}

.empty-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-muted, #6b7280);
  margin: 0 0 4px;
}

.empty-hint {
  font-size: 12px;
  color: var(--text-muted, #9ca3af);
  margin: 0;
}

.log-lines-container {
  border: 1px solid var(--line, #e5e7eb);
  border-radius: var(--radius-sm, 8px);
  overflow: hidden;
  max-height: 500px;
  overflow-y: auto;
}

.log-line {
  display: grid;
  grid-template-columns: 40px 1fr 28px;
  gap: 4px;
  align-items: start;
  padding: 4px 4px 4px 8px;
  border-bottom: 1px solid var(--line, #e5e7eb);
  font-size: 11px;
  line-height: 1.5;
  transition: background 0.1s ease;
  font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
}

.log-line:last-child {
  border-bottom: none;
}

.log-line:hover {
  background: var(--accent-bg-light, rgba(0,0,0,0.02));
}

.log-line-audit {
  background: var(--accent-bg-light, rgba(59,130,246,0.04));
  border-left: 3px solid var(--accent, #3b82f6);
}

.log-line-num {
  color: var(--text-muted, #9ca3af);
  font-size: 10px;
  user-select: none;
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.log-line-content {
  color: var(--text, #111827);
  white-space: pre;
  overflow-x: auto;
  word-break: break-all;
}

.log-line-copy {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border-radius: 4px;
  border: none;
  background: transparent;
  color: var(--text-muted, #9ca3af);
  cursor: pointer;
  opacity: 0;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.log-line:hover .log-line-copy {
  opacity: 0.6;
}

.log-line-copy:hover {
  opacity: 1 !important;
  background: var(--accent-bg-light, rgba(0,0,0,0.05));
  color: var(--text, #111827);
}

.log-line-copy:focus-visible {
  opacity: 1;
  outline: 2px solid var(--accent, #3b82f6);
  outline-offset: -2px;
}
</style>

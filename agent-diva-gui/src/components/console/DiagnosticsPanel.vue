<script setup lang="ts">
/**
 * OBS-08 bounded diagnostics view. Reads `diagnostics/logs` through
 * DiagnosticsReader (source/date/level/query verbatim, server cursor
 * paging, gap/has_more/truncated shown as-is) and shows the GUI
 * recorder's honest persistence accounting: queued / persisted /
 * dropped / unconfirmed — loss is always visible, never retried away.
 */
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { vivyClient } from '../../api/vivy/instance';
import {
  DiagnosticsReader,
  GuiDiagnosticRecorder,
  guiDiagnostics,
  type GuiQueueStats,
} from '../../state/gui-diagnostics';
import type { DiagnosticRecord } from '../../api/vivy/contracts';

const props = defineProps<{
  reader?: DiagnosticsReader;
  recorder?: GuiDiagnosticRecorder;
}>();

const { t } = useI18n();

const reader = props.reader ?? new DiagnosticsReader(vivyClient);
const recorder = props.recorder ?? guiDiagnostics;

const source = ref('runtime');
const date = ref('');
const level = ref('');
const query = ref('');
const tick = ref(0);

const page = computed(() => (tick.value, reader.page));
const lastError = computed(() => (tick.value, reader.lastError));
const loading = computed(() => (tick.value, reader.loading));
const stats = ref<GuiQueueStats>(recorder.stats());

function bump(): void {
  tick.value++;
}

async function refresh(): Promise<void> {
  await reader.refresh({
    source: source.value,
    date: date.value || undefined,
    level: level.value || undefined,
    query: query.value || undefined,
  });
  stats.value = recorder.stats();
  bump();
}

async function nextPage(): Promise<void> {
  await reader.nextPage();
  bump();
}

function timeText(rec: DiagnosticRecord): string {
  return rec.at ? new Date(rec.at).toLocaleTimeString() : '';
}

let poll: ReturnType<typeof setInterval> | null = null;
onMounted(() => {
  void refresh();
  // Keep the recorder accounting live; bounded (stats only, no RPCs).
  poll = setInterval(() => {
    stats.value = recorder.stats();
  }, 2000);
});
onBeforeUnmount(() => {
  if (poll) clearInterval(poll);
});
</script>

<template>
  <div class="diag-panel">
    <div class="diag-filters">
      <select v-model="source" class="ui-input diag-input" @change="refresh">
        <option value="runtime">{{ t('diagnostics.sourceRuntime') }}</option>
        <option value="gui">{{ t('diagnostics.sourceGui') }}</option>
      </select>
      <input v-model="date" type="date" class="ui-input diag-input" @change="refresh" />
      <select v-model="level" class="ui-input diag-input" @change="refresh">
        <option value="">{{ t('diagnostics.allLevels') }}</option>
        <option value="debug">debug</option>
        <option value="info">info</option>
        <option value="warn">warn</option>
        <option value="error">error</option>
      </select>
      <input
        v-model="query"
        class="ui-input diag-input diag-query"
        :placeholder="t('diagnostics.queryPlaceholder')"
        @keyup.enter="refresh"
      />
      <button class="ui-button ui-button--ghost diag-btn" :disabled="loading" @click="refresh">
        {{ t('diagnostics.refresh') }}
      </button>
      <button
        class="ui-button ui-button--ghost diag-btn"
        :disabled="loading || !page?.has_more || !page?.next_cursor"
        @click="nextPage"
      >
        {{ t('diagnostics.nextPage') }}
      </button>
    </div>

    <div v-if="page?.gap" class="diag-banner diag-banner--gap">
      {{ t('diagnostics.gap') }}
    </div>
    <div v-else-if="lastError" class="diag-banner diag-banner--error">{{ lastError }}</div>
    <div v-if="page?.has_more" class="diag-banner diag-banner--notice">
      {{ t('diagnostics.hasMore') }}
    </div>

    <div class="diag-stats">
      <span>{{ t('diagnostics.queued') }}: {{ stats.queued }}</span>
      <span>{{ t('diagnostics.persisted') }}: {{ stats.persisted }}</span>
      <span v-if="stats.dropped" class="diag-stat-warn">
        {{ t('diagnostics.dropped') }}: {{ stats.dropped }}
      </span>
      <span v-if="stats.unconfirmed" class="diag-stat-warn">
        {{ t('diagnostics.unconfirmed') }}: {{ stats.unconfirmed }}
      </span>
      <span v-if="stats.lastError" class="diag-stat-warn" :title="stats.lastError">
        {{ t('diagnostics.appendError') }}
      </span>
    </div>

    <div v-if="!page?.records?.length && !lastError" class="diag-empty">
      {{ loading ? t('diagnostics.loading') : t('diagnostics.empty') }}
    </div>
    <div v-else-if="page?.records?.length" class="diag-records">
      <div
        v-for="rec in page.records"
        :key="rec.id"
        class="diag-row"
        :class="{ 'diag-row--truncated': rec.truncated }"
      >
        <span class="diag-time">{{ timeText(rec) }}</span>
        <span class="diag-level" :class="`diag-level--${rec.level}`">{{ rec.level }}</span>
        <span class="diag-component">{{ rec.component }}</span>
        <span class="diag-message">{{ rec.message }}</span>
        <span v-if="rec.truncated" class="diag-trunc">{{ t('diagnostics.truncated') }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.diag-panel {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.diag-filters {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  align-items: center;
}

.diag-query {
  flex: 1;
  min-width: 8rem;
}

.diag-banner {
  padding: 0.5rem 0.75rem;
  border-radius: var(--radius-md);
  font-size: 0.8125rem;
}

.diag-banner--gap {
  background: var(--warning-soft);
  border: 1px solid var(--warning);
  color: var(--warning);
}

.diag-banner--error {
  background: var(--destructive-soft);
  border: 1px solid var(--destructive);
  color: var(--destructive);
}

.diag-banner--notice {
  background: var(--accent);
  border: 1px solid var(--border);
  color: var(--muted-foreground);
}

.diag-stats {
  display: flex;
  gap: 0.75rem;
  font-size: 0.75rem;
  color: var(--muted-foreground);
  flex-wrap: wrap;
}

.diag-stat-warn {
  color: var(--warning);
}

.diag-empty {
  padding: 1rem;
  text-align: center;
  color: var(--muted-foreground);
  font-size: 0.8125rem;
}

.diag-records {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.diag-row {
  display: grid;
  grid-template-columns: 5rem 3.5rem 6rem 1fr auto;
  gap: 0.5rem;
  padding: 0.375rem 0.625rem;
  font-size: 0.75rem;
  border-bottom: 1px solid var(--border);
  align-items: baseline;
}

.diag-row:last-child {
  border-bottom: none;
}

.diag-row--truncated {
  background: var(--warning-soft);
}

.diag-time {
  color: var(--muted-foreground);
  font-family: var(--font-mono, monospace);
}

.diag-level {
  text-transform: uppercase;
  font-size: 0.625rem;
  letter-spacing: 0.05em;
  color: var(--muted-foreground);
}

.diag-level--warn {
  color: var(--warning);
}

.diag-level--error {
  color: var(--destructive);
}

.diag-component {
  color: var(--muted-foreground);
}

.diag-message {
  color: var(--foreground);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.diag-trunc {
  color: var(--warning);
  font-size: 0.6875rem;
}
</style>

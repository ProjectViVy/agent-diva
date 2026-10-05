<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  Activity,
  ArrowRight,
  ChevronLeft,
  Coins,
  Download,
  TrendingUp,
  Zap
} from '@lucide/vue';
import { formatTokenCount, formatCost, type TimeRangePeriod } from '../../api/tokenStats';
import { vivyClient } from '../../api/vivy/instance';
import {
  costDisplay,
  usageView,
  vivyObservability,
  type HostConnectionState,
} from '../../state/vivy-observability';
import type { VivyTokenTimelinePoint } from '../../api/vivy/contracts';

const { t } = useI18n();

// View state
const showDetail = ref(false);

// State
const period = ref<TimeRangePeriod>('1d');
const loading = ref(false);

// Controller notifications — the observability lane owns snapshot,
// error and stale state; this tick just re-reads it.
const obsTick = ref(0);
const obs = vivyObservability;
const snap = computed(() => (obsTick.value, obs.snapshot()));
const conn = computed(() => (obsTick.value, obs.connection()));
const error = computed(() => (obsTick.value, obs.error()));
const stale = computed(() => (obsTick.value, obs.stale()));
const view = computed(() => (snap.value ? usageView(snap.value) : null));

const total = computed(() => snap.value?.total ?? null);
const providers = computed(() => snap.value?.providers ?? []);
const timeline = computed(() => snap.value?.timeline ?? []);
const sessions = computed(() => snap.value?.sessions ?? []);
const modelDistribution = computed(() => snap.value?.models ?? []);

const isPreview = computed(() => conn.value.state === 'preview');
const HOST_STATE_KEYS: Record<HostConnectionState, string> = {
  preview: 'tokenStats.hostPreview',
  connecting: 'tokenStats.hostConnecting',
  connected: 'tokenStats.hostConnected',
  gap: 'tokenStats.hostGap',
  lost: 'tokenStats.hostLost',
  unavailable: 'tokenStats.hostUnavailable',
  disconnected: 'tokenStats.hostDisconnected',
};
const connLabel = computed(() => t(HOST_STATE_KEYS[conn.value.state]));

// Auto-refresh interval
let refreshInterval: number | null = null;
let detachObs: (() => void) | null = null;

// Computed
const maxTimelineValue = computed(() => {
  if (timeline.value.length === 0) return 100;
  return Math.max(...timeline.value.map(p => p.total_tokens));
});

// Methods
async function fetchAllStats() {
  loading.value = true;
  try {
    // Coalesced + fenced inside the controller: a delayed response to a
    // superseded request resolves null and never replaces totals.
    await obs.refresh({
      period: period.value,
      tz_offset_minutes: new Date().getTimezoneOffset(),
      session_limit: 10,
    });
  } finally {
    loading.value = false;
  }
}

function costText(usd: number, costKnown: boolean): string {
  const c = costDisplay(usd, costKnown);
  return c.kind === 'known' ? formatCost(c.usd) : t('tokenStats.costUnknown');
}

function changePeriod(newPeriod: TimeRangePeriod) {
  period.value = newPeriod;
  fetchAllStats();
}

function getTimelineBarHeight(point: VivyTokenTimelinePoint): string {
  const max = maxTimelineValue.value;
  if (max === 0) return '10%';
  return `${Math.max(10, (point.total_tokens / max) * 100)}%`;
}


function getModelColor(index: number): string {
  const colors = ['#6366f1', '#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6'];
  return colors[index % colors.length];
}

function formatTimeBucket(isoString: string): string {
  if (!isoString) return '';
  const date = new Date(isoString);
  if (period.value === '1d') {
    return date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
  } else {
    return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
  }
}

function formatTimeBucketTooltip(isoString: string): string {
  if (!isoString) return '';
  const date = new Date(isoString);
  return date.toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
}

function exportData() {
  // Export the verbatim authoritative snapshot — coverage included.
  const data = { ...snap.value, exportedAt: new Date().toISOString() };

  const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `token-stats-${period.value}-${Date.now()}.json`;
  a.click();
  URL.revokeObjectURL(url);
}

function viewDetails() {
  showDetail.value = true;
}

function backToOverview() {
  showDetail.value = false;
}

// Lifecycle
onMounted(() => {
  detachObs = obs.subscribe(() => obsTick.value++);
  void obs.attach(vivyClient);
  fetchAllStats();
  // Refresh every 30 seconds; refresh is coalesced so the interval never
  // duplicates an in-flight request.
  refreshInterval = window.setInterval(fetchAllStats, 30000);
});

onUnmounted(() => {
  if (refreshInterval) {
    window.clearInterval(refreshInterval);
  }
  detachObs?.();
  // Listener teardown fences any in-flight response so it cannot land
  // after the panel is gone.
  obs.detach();
});
</script>

<template>
  <div class="token-stats-panel space-y-6">
    <!-- Detail View -->
    <template v-if="showDetail">
      <div class="detail-header">
        <button class="ui-button ui-button--ghost back-btn" @click="backToOverview">
          <ChevronLeft :size="16" />
          {{ t('tokenStats.viewDetails') }}
        </button>
      </div>

      <!-- Extended Stats for Detail View -->
      <div v-if="total" class="extended-stats">
        <!-- Cache Stats -->
        <div class="cache-stats">
          <h4 class="section-title">{{ t('tokenStats.cacheTokens') }}</h4>
          <div class="cache-grid">
            <div class="cache-item">
              <span class="cache-label">{{ t('tokenStats.cacheTokens') }}</span>
              <span class="cache-value">{{ formatTokenCount(total.total_cached) }}</span>
            </div>
            <div class="cache-item">
              <span class="cache-label">{{ t('tokenStats.reasoningTokens') }}</span>
              <span class="cache-value">{{ formatTokenCount(total.total_reasoning) }}</span>
            </div>
          </div>
          <p v-if="view && !view.coverage.hidden_retries_observable" class="coverage-note">
            {{ t('tokenStats.hiddenRetries') }}
          </p>
        </div>

        <!-- Endpoint Breakdown -->
        <div v-if="providers.length > 0" class="breakdown-section">
          <h4 class="section-title">{{ t('tokenStats.provider') }}</h4>
          <div class="breakdown-list">
            <div v-for="item in providers" :key="item.key" class="breakdown-item">
              <span class="breakdown-label">{{ item.key }}</span>
              <div class="breakdown-bar-container">
                <div
                  class="breakdown-bar"
                  :style="{ width: `${(item.total_tokens / (total?.total_tokens || 1)) * 100}%` }"
                ></div>
              </div>
              <span class="breakdown-value">{{ formatTokenCount(item.total_tokens) }}</span>
            </div>
          </div>
        </div>

        <!-- Extended Session List -->
        <div v-if="sessions.length > 0" class="extended-sessions">
          <h4 class="section-title">{{ t('tokenStats.sessionBreakdown') }}</h4>
          <div class="sessions-table">
            <div class="sessions-table-header">
              <span>{{ t('tokenStats.session') }}</span>
              <span>{{ t('tokenStats.inputTokens') }}</span>
              <span>{{ t('tokenStats.outputTokens') }}</span>
              <span>{{ t('tokenStats.estimatedCost') }}</span>
            </div>
            <div v-for="session in sessions" :key="session.id" class="sessions-table-row">
              <span class="session-name">{{ session.title || session.id }}</span>
              <span>{{ formatTokenCount(session.total_input) }}</span>
              <span>{{ formatTokenCount(session.total_output) }}</span>
              <span>{{ costText(session.cost_usd, session.cost_known) }}</span>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- Overview -->
    <template v-else>
      <!-- Period Selector -->
      <div class="flex items-center justify-between">
        <div class="flex gap-2">
          <button
            v-for="p in ['1d', '3d', '1w', '1m', '6m', '1y'] as TimeRangePeriod[]"
            :key="p"
            class="ui-button ui-button--ghost period-btn"
            :class="{ 'period-btn--active': period === p }"
            @click="changePeriod(p)"
          >
            {{ t(`tokenStats.period.${p}`, p) }}
          </button>
        </div>
        <button class="ui-button ui-button--ghost export-btn" @click="exportData">
          <Download :size="14" />
          {{ t('tokenStats.export') }}
        </button>
      </div>

      <!-- Host connection chip — initialize/transport truth, not a health RPC -->
      <div class="conn-row">
        <span class="conn-chip" :class="`conn-chip--${conn.state}`">{{ connLabel }}</span>
        <span v-if="conn.state === 'connected' && conn.protocolVersion" class="conn-meta">
          ABI v{{ conn.protocolVersion }}
        </span>
        <span v-if="stale" class="stale-chip">{{ t('tokenStats.staleTotals') }}</span>
      </div>

      <!-- Preview shell: no live host metadata or totals -->
      <div v-if="isPreview" class="coverage-banner coverage-banner--notice">
        {{ t('tokenStats.previewNotice') }}
      </div>

      <!-- Error Display -->
      <div v-else-if="error && !snap" class="error-banner">
        {{ error }}
      </div>

      <!-- Coverage banners: nil / legacy / partial are distinct labels -->
      <div v-else-if="view && view.state === 'empty'" class="coverage-banner coverage-banner--notice">
        {{ t('tokenStats.coverageEmpty') }}
      </div>
      <div v-else-if="view && view.state === 'legacy'" class="coverage-banner coverage-banner--notice">
        {{ t('tokenStats.coverageLegacy') }}
      </div>
      <div v-else-if="view && view.state === 'partial'" class="coverage-banner coverage-banner--partial">
        {{ t('tokenStats.coveragePartial') }} —
        {{
          t('tokenStats.coveragePartialDetail', {
            reported: view.coverage.reported_calls,
            missing: view.coverage.missing_usage_calls,
            partial: view.coverage.partial_usage_calls,
            active: view.coverage.active_calls,
          })
        }}
        <span v-if="view.unknownBuckets.length > 0">
          · {{ t('tokenStats.unknownBuckets', { buckets: view.unknownBuckets.join(', ') }) }}
        </span>
      </div>

      <!-- Stats Cards -->
      <div v-if="total" class="stats-grid">
        <div class="stat-card">
          <div class="stat-icon stat-icon--primary">
            <Zap :size="16" />
          </div>
          <div class="stat-content">
            <div class="stat-value">{{ formatTokenCount(total.total_tokens) }}</div>
            <div class="stat-label">{{ t('tokenStats.totalTokens') }}</div>
          </div>
        </div>

        <div class="stat-card">
          <div class="stat-icon stat-icon--blue">
            <TrendingUp :size="16" />
          </div>
          <div class="stat-content">
            <div class="stat-value text-info">{{ formatTokenCount(total.total_input) }}</div>
            <div class="stat-label">{{ t('tokenStats.inputTokens') }}</div>
          </div>
        </div>

        <div class="stat-card">
          <div class="stat-icon stat-icon--green">
            <Activity :size="16" />
          </div>
          <div class="stat-content">
            <div class="stat-value text-success">{{ formatTokenCount(total.total_output) }}</div>
            <div class="stat-label">{{ t('tokenStats.outputTokens') }}</div>
          </div>
        </div>

        <div class="stat-card">
          <div class="stat-icon stat-icon--amber">
            <Coins :size="16" />
          </div>
          <div class="stat-content">
            <div class="stat-value text-warning">{{ costText(total.total_cost_usd, total.cost_known) }}</div>
            <div class="stat-label">{{ t('tokenStats.estimatedCost') }}</div>
          </div>
        </div>
      </div>


      <!-- Model Distribution -->
      <div v-if="modelDistribution.length > 0" class="model-distribution">
        <h4 class="section-title">{{ t('tokenStats.modelDistribution') }}</h4>
        <div class="model-badges">
          <span
            v-for="(model, index) in modelDistribution.slice(0, 5)"
            :key="model.model"
            class="model-badge"
          >
            <span class="model-dot" :style="{ background: getModelColor(index) }"></span>
            {{ model.model.split('/').pop() || model.model }}: {{ model.percentage.toFixed(1) }}%
          </span>
        </div>
      </div>

      <!-- Timeline Chart -->
      <div v-if="timeline.length > 0" class="timeline-section">
        <h4 class="section-title">{{ t('tokenStats.usageTrend') }}</h4>
        <div class="timeline-chart">
          <div
            v-for="(point, index) in timeline"
            :key="index"
            class="timeline-bar"
            :style="{ height: getTimelineBarHeight(point) }"
            :title="`${formatTimeBucketTooltip(point.time_bucket)}\n${t('tokenStats.totalTokens')}: ${formatTokenCount(point.total_tokens)}\n${t('tokenStats.inputTokens')}: ${formatTokenCount(point.total_input)}\n${t('tokenStats.outputTokens')}: ${formatTokenCount(point.total_output)}`"
          >
            <div
              class="timeline-bar-output"
              :style="{ height: point.total_tokens > 0 ? (point.total_output / point.total_tokens * 100) + '%' : '0%' }"
            ></div>
            <div
              class="timeline-bar-input"
              :style="{ height: point.total_tokens > 0 ? (point.total_input / point.total_tokens * 100) + '%' : '0%' }"
            ></div>
          </div>
        </div>
        <div class="timeline-labels">
          <span>{{ formatTimeBucket(timeline[0]?.time_bucket) }}</span>
          <span>{{ formatTimeBucket(timeline[timeline.length - 1]?.time_bucket) }}</span>
        </div>
      </div>

      <!-- Sessions List -->
      <div v-if="sessions.length > 0" class="sessions-section">
        <h4 class="section-title">{{ t('tokenStats.sessionBreakdown') }}</h4>
        <div class="sessions-list">
          <div
            v-for="session in sessions.slice(0, 5)"
            :key="session.id"
            class="session-item"
          >
            <div class="session-info">
              <span class="session-id">{{ session.title || session.id.split(':').pop() }}</span>
              <span class="session-meta">
                {{ session.request_count }} {{ t('tokenStats.requests') }} · {{ session.model.split('/').pop() }}
              </span>
            </div>
            <div class="session-stats">
              <span class="session-tokens">{{ formatTokenCount(session.total_tokens) }}</span>
              <span class="session-cost">{{ costText(session.cost_usd, session.cost_known) }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- View Details Link -->
      <div class="view-details">
        <button class="ui-button ui-button--ghost ui-button--compact details-link" @click="viewDetails">
          {{ t('tokenStats.viewDetails') }}
          <ArrowRight :size="14" />
        </button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.token-stats-panel {
  padding: 1rem;
}

.period-btn {
  transition: all 0.2s;
}

.period-btn--active {
  background: var(--accent);
  color: var(--primary);
  border-color: var(--primary);
}

.export-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  transition: all 0.2s;
}

.error-banner {
  padding: 0.75rem 1rem;
  background: var(--destructive-soft);
  border: 1px solid var(--destructive);
  border-radius: var(--radius-md);
  color: var(--destructive);
  font-size: 0.875rem;
}

.conn-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.75rem;
}

.conn-chip {
  padding: 0.125rem 0.625rem;
  border-radius: 9999px;
  font-weight: 500;
  background: var(--accent);
  color: var(--muted-foreground);
}

.conn-chip--connected {
  background: var(--success-soft);
  color: var(--success);
}

.conn-chip--gap {
  background: var(--warning-soft);
  color: var(--warning);
}

.conn-chip--lost,
.conn-chip--unavailable {
  background: var(--destructive-soft);
  color: var(--destructive);
}

.conn-meta {
  color: var(--muted-foreground);
}

.stale-chip {
  padding: 0.125rem 0.625rem;
  border-radius: 9999px;
  background: var(--warning-soft);
  color: var(--warning);
  font-weight: 500;
}

.coverage-banner {
  padding: 0.625rem 1rem;
  border-radius: var(--radius-md);
  font-size: 0.8125rem;
}

.coverage-banner--notice {
  background: var(--accent);
  border: 1px solid var(--border);
  color: var(--muted-foreground);
}

.coverage-banner--partial {
  background: var(--warning-soft);
  border: 1px solid var(--warning);
  color: var(--warning);
}

.coverage-note {
  margin-top: 0.75rem;
  font-size: 0.75rem;
  color: var(--muted-foreground);
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 0.75rem;
}

@media (min-width: 768px) {
  .stats-grid {
    grid-template-columns: repeat(4, 1fr);
  }
}

.stat-card {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 1rem;
  background: var(--accent);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  transition: all 0.2s;
}

.stat-card:hover {
  border-color: var(--primary);
  transform: translateY(-1px);
}

.stat-icon {
  width: 2.25rem;
  height: 2.25rem;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
}

.stat-icon--primary {
  background: var(--info-soft);
  color: var(--info);
}

.stat-icon--blue {
  background: var(--info-soft);
  color: var(--info);
}

.stat-icon--green {
  background: var(--success-soft);
  color: var(--success);
}

.stat-icon--amber {
  background: var(--warning-soft);
  color: var(--warning);
}

.stat-content {
  flex: 1;
}

.stat-value {
  font-size: 1.125rem;
  font-weight: 700;
  color: var(--foreground);
}

.stat-label {
  font-size: 0.75rem;
  color: var(--muted-foreground);
  margin-top: 0.125rem;
}

.model-distribution {
  padding: 1rem;
  background: var(--accent);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
}

.section-title {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--foreground);
  margin-bottom: 0.75rem;
}

.model-badges {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.model-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.25rem 0.75rem;
  font-size: 0.75rem;
  background: var(--card);
  border-radius: 9999px;
  color: var(--foreground);
}

.model-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.timeline-section {
  padding: 1rem;
  background: var(--accent);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
}

.timeline-chart {
  display: flex;
  align-items: flex-end;
  gap: 2px;
  height: 6rem;
  padding: 0 0.25rem;
}

.timeline-bar {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  border-radius: 2px 2px 0 0;
  min-height: 4px;
  overflow: hidden;
  transition: height 0.3s ease;
  background: transparent;
}

.timeline-bar-output {
  width: 100%;
  background: var(--card);
  transition: background 0.3s ease;
}

.timeline-bar-input {
  width: 100%;
  background: var(--card);
  transition: background 0.3s ease;
}

.timeline-bar:hover .timeline-bar-output {
  background: var(--card);
}

.timeline-bar:hover .timeline-bar-input {
  background: var(--card);
}

.timeline-labels {
  display: flex;
  justify-content: space-between;
  margin-top: 0.5rem;
  font-size: 0.625rem;
  color: var(--muted-foreground);
}

.sessions-section {
  padding: 1rem;
  background: var(--accent);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
}

.sessions-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.session-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem;
  background: var(--card);
  border-radius: var(--radius-md);
  transition: background 0.2s;
}

.session-item:hover {
  background: var(--accent);
}

.session-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.session-id {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--foreground);
}

.session-meta {
  font-size: 0.75rem;
  color: var(--muted-foreground);
}

.session-stats {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 0.125rem;
}

.session-tokens {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--foreground);
}

.session-cost {
  font-size: 0.75rem;
  color: var(--muted-foreground);
}

.view-details {
  display: flex;
  justify-content: center;
}

.details-link {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  transition: all 0.2s;
}

/* Detail View Styles */
.detail-header {
  margin-bottom: 1rem;
}

.back-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  transition: all 0.2s;
}

.extended-stats {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.cache-stats {
  padding: 1rem;
  background: var(--accent);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
}

.cache-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 1rem;
}

.cache-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.cache-label {
  font-size: 0.75rem;
  color: var(--muted-foreground);
}

.cache-value {
  font-size: 1rem;
  font-weight: 600;
  color: var(--foreground);
}

.breakdown-section {
  padding: 1rem;
  background: var(--accent);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
}

.breakdown-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.breakdown-item {
  display: grid;
  grid-template-columns: 1fr 2fr auto;
  gap: 0.75rem;
  align-items: center;
}

.breakdown-label {
  font-size: 0.875rem;
  color: var(--foreground);
}

.breakdown-bar-container {
  height: 8px;
  background: var(--border);
  border-radius: 4px;
  overflow: hidden;
}

.breakdown-bar {
  height: 100%;
  background: var(--card);
  border-radius: 4px;
}

.breakdown-value {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--foreground);
  min-width: 80px;
  text-align: right;
}

.extended-sessions {
  padding: 1rem;
  background: var(--accent);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
}

.sessions-table {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.sessions-table-header {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr 1fr;
  gap: 0.75rem;
  padding: 0.5rem 0.75rem;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--muted-foreground);
  border-bottom: 1px solid var(--border);
}

.sessions-table-row {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr 1fr;
  gap: 0.75rem;
  padding: 0.75rem;
  font-size: 0.875rem;
  background: var(--card);
  border-radius: var(--radius-md);
}

.sessions-table-row:hover {
  background: var(--accent);
}

.session-name {
  color: var(--foreground);
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
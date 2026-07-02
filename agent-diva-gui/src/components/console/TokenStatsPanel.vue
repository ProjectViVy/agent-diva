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
} from 'lucide-vue-next';
import {
  getTokenUsageTotal,
  getTokenUsageSummary,
  getTokenUsageTimeline,
  getTokenUsageSessions,
  getTokenUsageModels,
  getTokenUsageRealtime,
  formatTokenCount,
  formatCost,
  type UsageTotal,
  type UsageSummary,
  type TimelinePoint,
  type SessionUsage,
  type ModelDistribution,
  type InMemoryStats,
  type TimeRangePeriod
} from '../../api/tokenStats';

const { t } = useI18n();

// View state
const showDetail = ref(false);

// State
const period = ref<TimeRangePeriod>('1d');
const loading = ref(false);
const error = ref<string | null>(null);

// Data
const total = ref<UsageTotal | null>(null);
const byEndpoint = ref<UsageSummary[]>([]);
const byModel = ref<UsageSummary[]>([]);
const timeline = ref<TimelinePoint[]>([]);
const sessions = ref<SessionUsage[]>([]);
const modelDistribution = ref<ModelDistribution[]>([]);
const realtimeStats = ref<InMemoryStats | null>(null);

// Auto-refresh interval
let refreshInterval: number | null = null;

// Computed
const maxTimelineValue = computed(() => {
  if (timeline.value.length === 0) return 100;
  return Math.max(...timeline.value.map(p => p.total_tokens));
});

// Methods
async function fetchAllStats() {
  loading.value = true;
  error.value = null;

  try {
    const [totalRes, endpointRes, modelRes, timelineRes, sessionsRes, modelsRes, realtimeRes] = await Promise.all([
      getTokenUsageTotal(period.value).catch(() => null),
      getTokenUsageSummary(period.value, 'endpoint').catch(() => []),
      getTokenUsageSummary(period.value, 'model').catch(() => []),
      getTokenUsageTimeline(period.value).catch(() => []),
      getTokenUsageSessions(period.value, 10).catch(() => []),
      getTokenUsageModels(period.value).catch(() => []),
      getTokenUsageRealtime().catch(() => null),
    ]);

    total.value = totalRes;
    byEndpoint.value = endpointRes;
    byModel.value = modelRes;
    timeline.value = timelineRes;
    sessions.value = sessionsRes;
    modelDistribution.value = modelsRes;
    realtimeStats.value = realtimeRes;
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function changePeriod(newPeriod: TimeRangePeriod) {
  period.value = newPeriod;
  fetchAllStats();
}

function getTimelineBarHeight(point: TimelinePoint): string {
  const max = maxTimelineValue.value;
  if (max === 0) return '10%';
  return `${Math.max(10, (point.total_tokens / max) * 100)}%`;
}

function getUsagePercentage(): number {
  if (!total.value || total.value.total_tokens === 0) return 0;
  // Assume 200k default budget
  const budget = 200_000;
  return Math.min(100, (total.value.total_tokens / budget) * 100);
}

function getModelColor(index: number): string {
  const colors = ['#6366f1', '#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6'];
  return colors[index % colors.length];
}

function exportData() {
  // Export stats as JSON
  const data = {
    period: period.value,
    total: total.value,
    byEndpoint: byEndpoint.value,
    byModel: byModel.value,
    timeline: timeline.value,
    sessions: sessions.value,
    exportedAt: new Date().toISOString()
  };

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
  fetchAllStats();
  // Refresh every 30 seconds
  refreshInterval = window.setInterval(fetchAllStats, 30000);
});

onUnmounted(() => {
  if (refreshInterval) {
    window.clearInterval(refreshInterval);
  }
});
</script>

<template>
  <div class="token-stats-panel space-y-6">
    <!-- Detail View -->
    <template v-if="showDetail">
      <div class="detail-header">
        <button class="back-btn" @click="backToOverview">
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
              <span class="cache-label">{{ t('tokenStats.cacheCreation') }}</span>
              <span class="cache-value">{{ formatTokenCount(total.total_cache_creation) }}</span>
            </div>
            <div class="cache-item">
              <span class="cache-label">{{ t('tokenStats.cacheRead') }}</span>
              <span class="cache-value">{{ formatTokenCount(total.total_cache_read) }}</span>
            </div>
          </div>
        </div>

        <!-- Endpoint Breakdown -->
        <div v-if="byEndpoint.length > 0" class="breakdown-section">
          <h4 class="section-title">{{ t('tokenStats.endpoint') }}</h4>
          <div class="breakdown-list">
            <div v-for="item in byEndpoint" :key="item.group_key" class="breakdown-item">
              <span class="breakdown-label">{{ item.group_key }}</span>
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
            <div v-for="session in sessions" :key="session.session_id" class="sessions-table-row">
              <span class="session-name">{{ session.session_id }}</span>
              <span>{{ formatTokenCount(session.total_input) }}</span>
              <span>{{ formatTokenCount(session.total_output) }}</span>
              <span>{{ formatCost(session.total_cost) }}</span>
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
            class="period-btn"
            :class="{ 'period-btn--active': period === p }"
            @click="changePeriod(p)"
          >
            {{ t(`tokenStats.period.${p}`, p) }}
          </button>
        </div>
        <button class="export-btn" @click="exportData">
          <Download :size="14" />
          {{ t('tokenStats.export') }}
        </button>
      </div>

      <!-- Error Display -->
      <div v-if="error" class="error-banner">
        {{ error }}
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
            <div class="stat-value text-blue-400">{{ formatTokenCount(total.total_input) }}</div>
            <div class="stat-label">{{ t('tokenStats.inputTokens') }}</div>
          </div>
        </div>

        <div class="stat-card">
          <div class="stat-icon stat-icon--green">
            <Activity :size="16" />
          </div>
          <div class="stat-content">
            <div class="stat-value text-emerald-400">{{ formatTokenCount(total.total_output) }}</div>
            <div class="stat-label">{{ t('tokenStats.outputTokens') }}</div>
          </div>
        </div>

        <div class="stat-card">
          <div class="stat-icon stat-icon--amber">
            <Coins :size="16" />
          </div>
          <div class="stat-content">
            <div class="stat-value text-amber-400">{{ formatCost(total.total_cost) }}</div>
            <div class="stat-label">{{ t('tokenStats.estimatedCost') }}</div>
          </div>
        </div>
      </div>

      <!-- Usage Progress -->
      <div v-if="total" class="usage-progress">
        <div class="usage-progress-header">
          <span class="usage-progress-label">{{ t('tokenStats.usageRate') }}</span>
          <span class="usage-progress-value">{{ getUsagePercentage().toFixed(1) }}%</span>
        </div>
        <div class="usage-progress-bar">
          <div
            class="usage-progress-fill"
            :style="{ width: `${getUsagePercentage()}%` }"
          ></div>
        </div>
        <div class="usage-progress-footer">
          <span>{{ formatTokenCount(total.total_tokens) }} {{ t('tokenStats.used') }}</span>
          <span>200K {{ t('tokenStats.budget') }}</span>
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
            :title="`${point.time_bucket}: ${formatTokenCount(point.total_tokens)}`"
          ></div>
        </div>
        <div class="timeline-labels">
          <span>{{ timeline[0]?.time_bucket || '' }}</span>
          <span>{{ timeline[timeline.length - 1]?.time_bucket || '' }}</span>
        </div>
      </div>

      <!-- Sessions List -->
      <div v-if="sessions.length > 0" class="sessions-section">
        <h4 class="section-title">{{ t('tokenStats.sessionBreakdown') }}</h4>
        <div class="sessions-list">
          <div
            v-for="session in sessions.slice(0, 5)"
            :key="session.session_id"
            class="session-item"
          >
            <div class="session-info">
              <span class="session-id">{{ session.session_id.split(':').pop() }}</span>
              <span class="session-meta">
                {{ session.request_count }} {{ t('tokenStats.requests') }} · {{ session.primary_model.split('/').pop() }}
              </span>
            </div>
            <div class="session-stats">
              <span class="session-tokens">{{ formatTokenCount(session.total_tokens) }}</span>
              <span class="session-cost">{{ formatCost(session.total_cost) }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- View Details Link -->
      <div class="view-details">
        <button class="details-link" @click="viewDetails">
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
  padding: 0.375rem 0.75rem;
  font-size: 0.75rem;
  font-weight: 500;
  border-radius: 9999px;
  background: var(--accent-bg-light);
  color: var(--text-muted);
  border: 1px solid transparent;
  transition: all 0.2s;
}

.period-btn:hover {
  background: var(--nav-hover);
}

.period-btn--active {
  background: var(--accent-bg-light);
  color: var(--accent);
  border-color: var(--accent);
}

.export-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.375rem 0.75rem;
  font-size: 0.75rem;
  color: var(--text-muted);
  background: transparent;
  border-radius: var(--radius-sm);
  transition: all 0.2s;
}

.export-btn:hover {
  background: var(--accent-bg-light);
  color: var(--text);
}

.error-banner {
  padding: 0.75rem 1rem;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.3);
  border-radius: var(--radius-sm);
  color: #ef4444;
  font-size: 0.875rem;
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
  background: var(--accent-bg-light);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  transition: all 0.2s;
}

.stat-card:hover {
  border-color: var(--accent);
  transform: translateY(-1px);
}

.stat-icon {
  width: 2.25rem;
  height: 2.25rem;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
}

.stat-icon--primary {
  background: rgba(99, 102, 241, 0.15);
  color: #6366f1;
}

.stat-icon--blue {
  background: rgba(59, 130, 246, 0.15);
  color: #3b82f6;
}

.stat-icon--green {
  background: rgba(16, 185, 129, 0.15);
  color: #10b981;
}

.stat-icon--amber {
  background: rgba(245, 158, 11, 0.15);
  color: #f59e0b;
}

.stat-content {
  flex: 1;
}

.stat-value {
  font-size: 1.125rem;
  font-weight: 700;
  color: var(--text);
}

.stat-label {
  font-size: 0.75rem;
  color: var(--text-muted);
  margin-top: 0.125rem;
}

.usage-progress {
  padding: 1rem;
  background: var(--accent-bg-light);
  border: 1px solid var(--line);
  border-radius: var(--radius);
}

.usage-progress-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 0.5rem;
}

.usage-progress-label {
  font-size: 0.75rem;
  color: var(--text-muted);
}

.usage-progress-value {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text);
}

.usage-progress-bar {
  height: 6px;
  background: var(--line);
  border-radius: 3px;
  overflow: hidden;
}

.usage-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #6366f1, #8b5cf6);
  border-radius: 3px;
  transition: width 0.5s ease;
}

.usage-progress-footer {
  display: flex;
  justify-content: space-between;
  margin-top: 0.375rem;
  font-size: 0.625rem;
  color: var(--text-muted);
}

.model-distribution {
  padding: 1rem;
  background: var(--accent-bg-light);
  border: 1px solid var(--line);
  border-radius: var(--radius);
}

.section-title {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text);
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
  background: var(--panel-solid);
  border-radius: 9999px;
  color: var(--text);
}

.model-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.timeline-section {
  padding: 1rem;
  background: var(--accent-bg-light);
  border: 1px solid var(--line);
  border-radius: var(--radius);
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
  background: linear-gradient(180deg, #6366f1 0%, rgba(99, 102, 241, 0.3) 100%);
  border-radius: 2px 2px 0 0;
  min-height: 4px;
  transition: height 0.3s ease;
}

.timeline-bar:hover {
  background: linear-gradient(180deg, #8b5cf6 0%, rgba(139, 92, 246, 0.4) 100%);
}

.timeline-labels {
  display: flex;
  justify-content: space-between;
  margin-top: 0.5rem;
  font-size: 0.625rem;
  color: var(--text-muted);
}

.sessions-section {
  padding: 1rem;
  background: var(--accent-bg-light);
  border: 1px solid var(--line);
  border-radius: var(--radius);
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
  background: var(--panel-solid);
  border-radius: var(--radius-sm);
  transition: background 0.2s;
}

.session-item:hover {
  background: var(--nav-hover);
}

.session-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.session-id {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text);
}

.session-meta {
  font-size: 0.75rem;
  color: var(--text-muted);
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
  color: var(--text);
}

.session-cost {
  font-size: 0.75rem;
  color: var(--text-muted);
}

.view-details {
  display: flex;
  justify-content: center;
}

.details-link {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.5rem 1rem;
  font-size: 0.875rem;
  color: var(--accent);
  background: transparent;
  border-radius: var(--radius-sm);
  transition: all 0.2s;
}

.details-link:hover {
  background: var(--accent-bg-light);
}

/* Detail View Styles */
.detail-header {
  margin-bottom: 1rem;
}

.back-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.5rem 0.75rem;
  font-size: 0.875rem;
  color: var(--text-muted);
  background: var(--accent-bg-light);
  border-radius: var(--radius-sm);
  transition: all 0.2s;
}

.back-btn:hover {
  color: var(--text);
  background: var(--nav-hover);
}

.extended-stats {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.cache-stats {
  padding: 1rem;
  background: var(--accent-bg-light);
  border: 1px solid var(--line);
  border-radius: var(--radius);
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
  color: var(--text-muted);
}

.cache-value {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text);
}

.breakdown-section {
  padding: 1rem;
  background: var(--accent-bg-light);
  border: 1px solid var(--line);
  border-radius: var(--radius);
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
  color: var(--text);
}

.breakdown-bar-container {
  height: 8px;
  background: var(--line);
  border-radius: 4px;
  overflow: hidden;
}

.breakdown-bar {
  height: 100%;
  background: linear-gradient(90deg, #6366f1, #8b5cf6);
  border-radius: 4px;
}

.breakdown-value {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text);
  min-width: 80px;
  text-align: right;
}

.extended-sessions {
  padding: 1rem;
  background: var(--accent-bg-light);
  border: 1px solid var(--line);
  border-radius: var(--radius);
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
  color: var(--text-muted);
  border-bottom: 1px solid var(--line);
}

.sessions-table-row {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr 1fr;
  gap: 0.75rem;
  padding: 0.75rem;
  font-size: 0.875rem;
  background: var(--panel-solid);
  border-radius: var(--radius-sm);
}

.sessions-table-row:hover {
  background: var(--nav-hover);
}

.session-name {
  color: var(--text);
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
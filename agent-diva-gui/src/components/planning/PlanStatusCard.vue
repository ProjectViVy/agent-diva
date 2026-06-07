<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { PlanDetail, TodoDetail } from '../../api/planning';

const { t } = useI18n();

const props = defineProps<{
  plan: PlanDetail;
}>();

// --- Computed stats ---
const totalTodos = computed(() => props.plan.todos.length);

const completedCount = computed(
  () => props.plan.todos.filter((t: TodoDetail) => t.status === 'completed').length,
);

const inProgressCount = computed(
  () => props.plan.todos.filter((t: TodoDetail) => t.status === 'in_progress').length,
);

const progressPercent = computed(() => {
  if (totalTodos.value === 0) return 0;
  return Math.round((completedCount.value / totalTodos.value) * 100);
});

const statusColor = computed(() => {
  switch (props.plan.status) {
    case 'active':
      return 'var(--accent, #ec407a)';
    case 'completed':
      return 'var(--success, #66bb6a)';
    case 'failed':
      return 'var(--danger, #ef5350)';
    default:
      return 'var(--text-muted, rgba(240, 230, 239, 0.55))';
  }
});
</script>

<template>
  <div class="plan-status-card">
    <!-- Header -->
    <div class="plan-header">
      <h3 class="plan-title">{{ plan.title }}</h3>
      <span class="plan-status-badge" :style="{ color: statusColor }">
        {{ plan.status }}
      </span>
    </div>

    <!-- Goal -->
    <p class="plan-goal">{{ plan.goal }}</p>

    <!-- 4-grid metrics -->
    <div class="plan-metrics">
      <div class="metric-item">
        <span class="metric-value" style="color: var(--accent, #ec407a)">{{ plan.phase }}</span>
        <span class="metric-label">{{ t('planning.phase') }}</span>
      </div>
      <div class="metric-item">
        <span class="metric-value" style="color: var(--text, #f0e6ef)">{{ totalTodos }}</span>
        <span class="metric-label">{{ t('planning.totalTodos') }}</span>
      </div>
      <div class="metric-item">
        <span class="metric-value" style="color: var(--success, #66bb6a)">{{ completedCount }}</span>
        <span class="metric-label">{{ t('planning.completed') }}</span>
      </div>
      <div class="metric-item">
        <span class="metric-value" style="color: var(--warning, #ffa726)">{{ inProgressCount }}</span>
        <span class="metric-label">{{ t('planning.inProgress') }}</span>
      </div>
    </div>

    <!-- Progress bar -->
    <div class="plan-progress-section">
      <div class="plan-progress-header">
        <span class="plan-progress-label">{{ t('planning.progress') }}</span>
        <span class="plan-progress-value">{{ progressPercent }}%</span>
      </div>
      <div class="plan-progress-bar">
        <div
          class="plan-progress-fill"
          :style="{ width: `${progressPercent}%` }"
        />
      </div>
    </div>

    <!-- Strategy (collapsible via details) -->
    <details v-if="plan.strategy" class="plan-details">
      <summary class="plan-details-summary">{{ t('planning.strategy') }}</summary>
      <p class="plan-details-content">{{ plan.strategy }}</p>
    </details>

    <!-- Assumptions -->
    <details v-if="plan.assumptions.length > 0" class="plan-details">
      <summary class="plan-details-summary">{{ t('planning.assumptions') }} ({{ plan.assumptions.length }})</summary>
      <ul class="plan-details-list">
        <li v-for="(item, i) in plan.assumptions" :key="i">{{ item }}</li>
      </ul>
    </details>

    <!-- Risks -->
    <details v-if="plan.risks.length > 0" class="plan-details">
      <summary class="plan-details-summary">{{ t('planning.risks') }} ({{ plan.risks.length }})</summary>
      <ul class="plan-details-list plan-details-list--risk">
        <li v-for="(item, i) in plan.risks" :key="i">{{ item }}</li>
      </ul>
    </details>

    <!-- Open Questions -->
    <details v-if="plan.open_questions.length > 0" class="plan-details">
      <summary class="plan-details-summary">{{ t('planning.openQuestions') }} ({{ plan.open_questions.length }})</summary>
      <ul class="plan-details-list">
        <li v-for="(item, i) in plan.open_questions" :key="i">{{ item }}</li>
      </ul>
    </details>
  </div>
</template>

<style scoped>
.plan-status-card {
  background: var(--bg-panel, rgba(30, 20, 28, 0.95));
  border: 1px solid var(--line, rgba(255, 255, 255, 0.08));
  border-radius: 12px;
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 0.875rem;
  transition: border-color 0.2s ease;
}

.plan-status-card:hover {
  border-color: var(--accent-border, rgba(236, 64, 122, 0.3));
}

/* Header */
.plan-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.plan-title {
  font-size: 1.15rem;
  font-weight: 700;
  color: var(--text, #f0e6ef);
  margin: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plan-status-badge {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  background: rgba(255, 255, 255, 0.05);
  padding: 0.2rem 0.6rem;
  border-radius: 9999px;
  flex-shrink: 0;
}

/* Goal */
.plan-goal {
  font-size: 0.875rem;
  color: var(--text-muted, rgba(240, 230, 239, 0.55));
  margin: 0;
  line-height: 1.5;
}

/* 4-grid metrics */
.plan-metrics {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 0.75rem;
}

.metric-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.25rem;
  padding: 0.625rem 0.375rem;
  background: rgba(255, 255, 255, 0.03);
  border-radius: 8px;
  border: 1px solid var(--line, rgba(255, 255, 255, 0.05));
}

.metric-value {
  font-size: 1.1rem;
  font-weight: 700;
}

.metric-label {
  font-size: 0.7rem;
  color: var(--text-muted, rgba(240, 230, 239, 0.55));
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

/* Progress bar */
.plan-progress-section {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.plan-progress-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.plan-progress-label {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-muted, rgba(240, 230, 239, 0.55));
}

.plan-progress-value {
  font-size: 0.8rem;
  font-weight: 700;
  color: var(--accent, #ec407a);
}

.plan-progress-bar {
  height: 6px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 3px;
  overflow: hidden;
}

.plan-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent, #ec407a), var(--accent-light, #f48fb1));
  border-radius: 3px;
  transition: width 0.4s ease;
}

/* Collapsible sections */
.plan-details {
  border: 1px solid var(--line, rgba(255, 255, 255, 0.06));
  border-radius: 8px;
  overflow: hidden;
}

.plan-details-summary {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text, #f0e6ef);
  padding: 0.5rem 0.75rem;
  cursor: pointer;
  background: rgba(255, 255, 255, 0.02);
  transition: background 0.15s ease;
  user-select: none;
}

.plan-details-summary:hover {
  background: rgba(236, 64, 122, 0.06);
}

.plan-details-content {
  font-size: 0.85rem;
  color: var(--text-muted, rgba(240, 230, 239, 0.65));
  padding: 0.5rem 0.75rem;
  margin: 0;
  line-height: 1.6;
}

.plan-details-list {
  font-size: 0.85rem;
  color: var(--text-muted, rgba(240, 230, 239, 0.65));
  padding: 0.375rem 0.75rem 0.5rem 1.75rem;
  margin: 0;
  line-height: 1.6;
}

.plan-details-list--risk {
  color: var(--warning, #ffa726);
}
</style>

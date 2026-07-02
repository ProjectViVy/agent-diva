<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { Loader2, AlertCircle, Inbox } from 'lucide-vue-next';
import PlanStatusCard from './PlanStatusCard.vue';
import TodoListPanel from './TodoListPanel.vue';
import type { PlanSummary, PlanDetail } from '../../api/planning';

const { t } = useI18n();

// --- State ---
const plans = ref<PlanSummary[]>([]);
const selectedPlanId = ref<string | null>(null);
const selectedPlan = ref<PlanDetail | null>(null);
const loading = ref(false);
const detailLoading = ref(false);
const error = ref('');

let pollHandle: ReturnType<typeof setInterval> | null = null;

// --- Computed ---
const sortedPlans = computed(() => {
  return [...plans.value].sort((a, b) => {
    // Active plan first
    if (a.is_active && !b.is_active) return -1;
    if (!a.is_active && b.is_active) return 1;
    return 0;
  });
});

// --- Data loading ---
async function loadPlans() {
  try {
    loading.value = true;
    error.value = '';
    plans.value = await invoke<PlanSummary[]>('get_plans');
  } catch (err) {
    console.error('[PlanningView] Failed to load plans:', err);
    error.value = String(err);
  } finally {
    loading.value = false;
  }
}

async function loadPlanDetail(planId: string) {
  try {
    detailLoading.value = true;
    selectedPlan.value = await invoke<PlanDetail>('get_plan', { planId });
  } catch (err) {
    console.error('[PlanningView] Failed to load plan detail:', err);
    selectedPlan.value = null;
  } finally {
    detailLoading.value = false;
  }
}

async function loadActivePlan() {
  try {
    const active = await invoke<PlanDetail | null>('get_active_plan');
    if (active) {
      selectedPlanId.value = active.id;
      selectedPlan.value = active;
    }
  } catch {
    // No active plan — not an error
  }
}

function selectPlan(planId: string) {
  selectedPlanId.value = planId;
  loadPlanDetail(planId);
}

// --- Polling ---
function startPolling() {
  pollHandle = setInterval(() => {
    loadPlans();
    if (selectedPlanId.value) {
      loadPlanDetail(selectedPlanId.value);
    }
  }, 5000);
}

function stopPolling() {
  if (pollHandle) {
    clearInterval(pollHandle);
    pollHandle = null;
  }
}

// --- Lifecycle ---
onMounted(async () => {
  await loadPlans();
  await loadActivePlan();
  startPolling();
});

onUnmounted(() => {
  stopPolling();
});
</script>

<template>
  <div class="planning-view">
    <!-- Left pane: plan list -->
    <div class="plan-list-pane">
      <div class="plan-list-header">
        <h2 class="plan-list-title">{{ t('planning.title') }}</h2>
      </div>

      <!-- Loading state -->
      <div v-if="loading && plans.length === 0" class="plan-list-empty">
        <Loader2 :size="20" class="animate-spin text-pink-400" />
      </div>

      <!-- Error state -->
      <div v-else-if="error && plans.length === 0" class="plan-list-empty">
        <AlertCircle :size="20" class="text-red-400" />
        <span class="text-sm text-red-400">{{ error }}</span>
      </div>

      <!-- Empty state -->
      <div v-else-if="plans.length === 0" class="plan-list-empty">
        <Inbox :size="24" class="text-gray-500" />
        <span class="text-sm text-gray-500">{{ t('planning.noActivePlan') }}</span>
      </div>

      <!-- Plan list -->
      <div v-else class="plan-list-items">
        <button
          v-for="plan in sortedPlans"
          :key="plan.id"
          class="plan-list-item"
          :class="{
            'plan-list-item--active': plan.is_active,
            'plan-list-item--selected': plan.id === selectedPlanId,
          }"
          @click="selectPlan(plan.id)"
        >
          <div class="plan-item-header">
            <span class="plan-item-title">{{ plan.title }}</span>
            <span v-if="plan.is_active" class="plan-item-badge">●</span>
          </div>
          <div class="plan-item-meta">
            <span class="plan-item-phase">{{ plan.phase }}</span>
            <span class="plan-item-progress">{{ plan.todo_completed }}/{{ plan.todo_count }}</span>
          </div>
        </button>
      </div>
    </div>

    <!-- Right pane: plan detail -->
    <div class="plan-detail-pane">
      <!-- No selection -->
      <div v-if="!selectedPlanId" class="plan-detail-empty">
        <Inbox :size="32" class="text-gray-600" />
        <p class="text-gray-500 mt-2">{{ t('planning.selectPlan') }}</p>
      </div>

      <!-- Loading detail -->
      <div v-else-if="detailLoading && !selectedPlan" class="plan-detail-empty">
        <Loader2 :size="24" class="animate-spin text-pink-400" />
      </div>

      <!-- Plan detail content -->
      <template v-else-if="selectedPlan">
        <PlanStatusCard :plan="selectedPlan" />
        <TodoListPanel :todos="selectedPlan.todos" />
      </template>
    </div>
  </div>
</template>

<style scoped>
.planning-view {
  display: flex;
  height: 100%;
  gap: 0;
  overflow: hidden;
}

/* Left pane */
.plan-list-pane {
  width: 280px;
  min-width: 240px;
  max-width: 320px;
  border-right: 1px solid var(--line, rgba(255, 255, 255, 0.08));
  display: flex;
  flex-direction: column;
  background: var(--bg-panel, rgba(30, 20, 28, 0.95));
  overflow-y: auto;
}

.plan-list-header {
  padding: 1rem;
  border-bottom: 1px solid var(--line, rgba(255, 255, 255, 0.08));
  flex-shrink: 0;
}

.plan-list-title {
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--text, #f0e6ef);
  margin: 0;
}

.plan-list-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 2rem 1rem;
  flex: 1;
}

.plan-list-items {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow-y: auto;
}

.plan-list-item {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 0.75rem 1rem;
  background: transparent;
  border: none;
  border-bottom: 1px solid var(--line, rgba(255, 255, 255, 0.05));
  cursor: pointer;
  text-align: left;
  transition: all 0.15s ease;
  color: var(--text, #f0e6ef);
}

.plan-list-item:hover {
  background: rgba(236, 64, 122, 0.08);
}

.plan-list-item--selected {
  background: rgba(236, 64, 122, 0.12);
  border-left: 3px solid var(--accent, #ec407a);
}

.plan-list-item--active .plan-item-title {
  color: var(--accent, #ec407a);
}

.plan-item-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.plan-item-title {
  font-size: 0.9rem;
  font-weight: 600;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.plan-item-badge {
  color: var(--accent, #ec407a);
  font-size: 0.75rem;
  animation: pulse-dot 2s ease-in-out infinite;
}

@keyframes pulse-dot {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.plan-item-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
}

.plan-item-phase {
  font-size: 0.75rem;
  color: var(--text-muted, rgba(240, 230, 239, 0.55));
  text-transform: capitalize;
}

.plan-item-progress {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--accent, #ec407a);
  background: var(--accent-bg-light, rgba(236, 64, 122, 0.12));
  padding: 0.125rem 0.5rem;
  border-radius: 9999px;
}

/* Right pane */
.plan-detail-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1rem;
  overflow-y: auto;
  background: var(--bg-main, rgba(20, 14, 18, 0.6));
}

.plan-detail-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  flex: 1;
  gap: 0.5rem;
}
</style>

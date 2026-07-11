<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { Loader2, AlertCircle, Inbox } from 'lucide-vue-next';
import type { PlanSummary, PlanDetail } from '../../api/planning';
import PlanDocument from './PlanDocument.vue';

const { t } = useI18n();

const props = defineProps<{
  initialPlanId?: string | null;
}>();

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

async function loadInitialPlan() {
  if (!props.initialPlanId) {
    await loadActivePlan();
    return;
  }
  selectedPlanId.value = props.initialPlanId;
  await loadPlanDetail(props.initialPlanId);
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
  await loadInitialPlan();
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
        <h2 class="plan-list-title">计划历史</h2>
      </div>

      <!-- Loading state -->
      <div v-if="loading && plans.length === 0" class="plan-list-empty">
        <Loader2 :size="20" class="animate-spin" style="color: var(--accent)" />
      </div>

      <!-- Error state -->
      <div v-else-if="error && plans.length === 0" class="plan-list-empty">
        <AlertCircle :size="20" style="color: var(--danger)" />
        <span class="text-sm" style="color: var(--danger)">{{ error }}</span>
      </div>

      <!-- Empty state -->
      <div v-else-if="plans.length === 0" class="plan-list-empty">
        <Inbox :size="24" style="color: var(--text-muted)" />
        <span class="text-sm" style="color: var(--text-muted)">{{ t('planning.noActivePlan') }}</span>
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
            <span class="plan-item-progress">{{ plan.phase }}</span>
          </div>
        </button>
      </div>
    </div>

    <!-- Right pane: plan detail -->
    <div class="plan-detail-pane">
      <!-- No selection -->
      <div v-if="!selectedPlanId" class="plan-detail-empty">
        <Inbox :size="32" style="color: var(--text-muted)" />
        <p class="mt-2" style="color: var(--text-muted)">{{ t('planning.selectPlan') }}</p>
      </div>

      <!-- Loading detail -->
      <div v-else-if="detailLoading && !selectedPlan" class="plan-detail-empty">
        <Loader2 :size="24" class="animate-spin" style="color: var(--accent)" />
      </div>

      <!-- Plan detail content -->
      <PlanDocument
        v-else-if="selectedPlan"
        :title="selectedPlan.title"
        :goal="selectedPlan.goal"
        :strategy="selectedPlan.strategy"
        :steps="selectedPlan.steps"
        :assumptions="selectedPlan.assumptions"
        :risks="selectedPlan.risks"
        :open-questions="selectedPlan.open_questions"
      />
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
  border-right: 1px solid var(--line);
  display: flex;
  flex-direction: column;
  background: var(--panel);
  overflow-y: auto;
}

.plan-list-header {
  padding: 1rem;
  border-bottom: 1px solid var(--line);
  flex-shrink: 0;
}

.plan-list-title {
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--text);
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
  border-bottom: 1px solid var(--line);
  cursor: pointer;
  text-align: left;
  transition: all 0.15s ease;
  color: var(--text);
}

.plan-list-item:hover {
  background: var(--accent-bg-light);
}

.plan-list-item--selected {
  background: var(--accent-bg-hover);
  border-left: 3px solid var(--accent);
}

.plan-list-item--active .plan-item-title {
  color: var(--accent);
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
  color: var(--accent);
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
  color: var(--text-muted);
  text-transform: capitalize;
}

.plan-item-progress {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--accent);
  background: var(--accent-bg-light);
  padding: 0.125rem 0.5rem;
  border-radius: 9999px;
}

.plan-delete-button { display: inline-flex; align-items: center; justify-content: center; flex: 0 0 auto; padding: 3px; border: 0; border-radius: 5px; color: var(--text-muted); background: transparent; cursor: pointer; }
.plan-delete-button:hover { color: var(--danger); background: color-mix(in srgb, var(--danger) 10%, transparent); }

/* Right pane */
.plan-detail-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1rem;
  overflow-y: auto;
  background: var(--panel);
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

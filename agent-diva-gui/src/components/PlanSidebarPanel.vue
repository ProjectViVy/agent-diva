<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { CheckCircle2, ClipboardList, Loader2, X } from 'lucide-vue-next';
import type { PlanRuntimeState, PlanSummary } from '../api/planning';
import { isTauriRuntime } from '../api/desktop';

const props = defineProps<{
  activePlan?: PlanRuntimeState | null;
}>();

const emit = defineEmits<{
  (event: 'select-plan', planId: string): void;
  (event: 'close'): void;
}>();

const plans = ref<PlanSummary[]>([]);
const loading = ref(false);

const visiblePlans = computed(() => {
  const merged = [...plans.value];
  const active = props.activePlan;
  if (active && !merged.some((plan) => plan.id === active.plan_id)) {
    merged.unshift({
      id: active.plan_id,
      title: active.title,
      goal: active.goal,
      phase: active.phase,
      status: active.status,
      todo_count: active.todos.length,
      todo_completed: active.todos.filter((todo) => todo.status.toLowerCase() === 'completed').length,
      is_active: true,
    });
  }
  return merged.sort((a, b) => Number(b.is_active) - Number(a.is_active));
});

async function loadPlans() {
  if (!isTauriRuntime()) return;
  loading.value = true;
  try {
    plans.value = await invoke<PlanSummary[]>('get_plans');
  } catch (error) {
    console.warn('[PlanSidebarPanel] Failed to load plans:', error);
  } finally {
    loading.value = false;
  }
}

onMounted(loadPlans);
watch(() => props.activePlan?.updated_at, loadPlans);
</script>

<template>
  <aside class="plan-sidebar-panel" aria-label="计划列表">
    <header class="plan-panel-header">
      <div class="plan-panel-title"><ClipboardList :size="16" /> <span>计划</span></div>
      <button type="button" class="plan-panel-close" title="收起计划栏" @click="emit('close')">
        <X :size="16" />
      </button>
    </header>
    <div class="plan-panel-list">
      <button
        v-for="plan in visiblePlans"
        :key="plan.id"
        type="button"
        class="plan-panel-item"
        :class="{ 'plan-panel-item-active': plan.is_active }"
        @click="emit('select-plan', plan.id)"
      >
        <span class="plan-panel-item-icon"><ClipboardList :size="13" /></span>
        <span class="plan-panel-item-body">
          <span class="plan-panel-item-title">{{ plan.title }}</span>
          <span class="plan-panel-item-meta">
            <span>{{ plan.phase }}</span>
            <span>{{ plan.todo_completed }}/{{ plan.todo_count }}</span>
          </span>
        </span>
        <CheckCircle2 v-if="plan.status.toLowerCase() === 'completed'" :size="14" class="text-emerald-500" />
        <span v-else-if="plan.is_active" class="plan-panel-active-dot" />
      </button>
      <div v-if="loading" class="plan-panel-empty"><Loader2 :size="15" class="animate-spin" /></div>
      <div v-else-if="visiblePlans.length === 0" class="plan-panel-empty">暂无计划</div>
    </div>
  </aside>
</template>

<style scoped>
.plan-sidebar-panel { position: absolute; top: 56px; right: var(--plan-sidebar-right, 12px); z-index: 45; display: flex; flex-direction: column; width: 280px; max-height: calc(100% - 68px); border: 1px solid var(--line, #e5e7eb); border-radius: 12px; background: var(--panel-solid, #fff); box-shadow: 0 10px 30px rgba(15, 23, 42, .14); overflow: hidden; }
.plan-panel-header { display: flex; align-items: center; justify-content: space-between; padding: 11px 12px; border-bottom: 1px solid var(--line, #e5e7eb); color: var(--text, #111827); background: color-mix(in srgb, var(--panel, #fff) 94%, var(--brand, #ec4899)); }
.plan-panel-title { display: flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 700; }
.plan-panel-title svg { color: var(--brand, #ec4899); }
.plan-panel-close { display: flex; align-items: center; justify-content: center; padding: 4px; border: 0; border-radius: 6px; color: var(--text-muted, #9ca3af); background: transparent; cursor: pointer; }
.plan-panel-close:hover { color: var(--text, #111827); background: var(--nav-hover, rgba(0, 0, 0, .05)); }
.plan-panel-list { padding: 8px; overflow-y: auto; }
.plan-panel-item { display: flex; align-items: center; width: 100%; gap: 8px; padding: 8px; border: 0; border-radius: 8px; color: var(--text, #111827); background: transparent; text-align: left; cursor: pointer; }
.plan-panel-item:hover, .plan-panel-item-active { background: var(--nav-active, rgba(0, 0, 0, .06)); }
.plan-panel-item-icon { display: flex; align-items: center; justify-content: center; width: 25px; height: 25px; flex: 0 0 auto; border: 1px solid var(--line, #e5e7eb); border-radius: 6px; color: var(--brand, #ec4899); background: var(--panel-solid, #fff); }
.plan-panel-item-body { display: flex; flex: 1; min-width: 0; flex-direction: column; gap: 2px; }
.plan-panel-item-title { overflow: hidden; font-size: 12px; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }
.plan-panel-item-meta { display: flex; justify-content: space-between; gap: 6px; color: var(--text-muted, #9ca3af); font-size: 10px; }
.plan-panel-active-dot { width: 6px; height: 6px; flex: 0 0 auto; border-radius: 50%; background: var(--brand, #ec4899); }
.plan-panel-empty { display: flex; align-items: center; justify-content: center; min-height: 50px; color: var(--text-muted, #9ca3af); font-size: 11px; }
</style>

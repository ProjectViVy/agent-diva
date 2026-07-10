<script setup lang="ts">
import { ref } from 'vue';
import { ChevronDown, ChevronRight, ClipboardList, CheckCircle2, Clock, Loader2, Lock } from 'lucide-vue-next';
import type { PlanRuntimeState } from '../../api/planning';

defineProps<{ plan: PlanRuntimeState }>();
const detailsOpen = ref(false);

function isCompleted(status: string) {
  return status.toLowerCase() === 'completed';
}

function isInProgress(status: string) {
  return ['inprogress', 'in_progress'].includes(status.toLowerCase());
}
</script>

<template>
  <section class="plan-history-card" aria-label="Plan history">
    <div class="plan-history-header">
      <div class="plan-history-title-wrap">
        <div class="plan-history-icon"><ClipboardList :size="16" /></div>
        <div class="min-w-0">
          <div class="plan-history-eyebrow">计划记录</div>
          <h3 class="plan-history-title">{{ plan.title }}</h3>
        </div>
      </div>
      <span class="plan-history-phase">{{ plan.phase }}</span>
    </div>
    <p class="plan-history-goal">{{ plan.goal }}</p>

    <div v-if="plan.todos.length" class="plan-history-todos">
      <div v-for="todo in plan.todos" :key="todo.id" class="plan-history-todo">
        <CheckCircle2 v-if="isCompleted(todo.status)" :size="14" class="text-emerald-500" />
        <Loader2 v-else-if="isInProgress(todo.status)" :size="14" class="text-amber-500 animate-spin" />
        <Lock v-else-if="todo.status.toLowerCase() === 'blocked'" :size="14" class="text-red-500" />
        <Clock v-else :size="14" class="text-gray-400" />
        <span :class="{ 'line-through text-gray-400': isCompleted(todo.status) }">{{ todo.title }}</span>
      </div>
    </div>

    <button type="button" class="plan-history-toggle" @click="detailsOpen = !detailsOpen">
      <ChevronDown v-if="detailsOpen" :size="14" />
      <ChevronRight v-else :size="14" />
      {{ detailsOpen ? '收起详情' : '查看详情' }}
    </button>
    <div v-if="detailsOpen" class="plan-history-details">
      <p v-if="plan.strategy"><strong>策略：</strong>{{ plan.strategy }}</p>
      <div v-for="step in plan.steps" :key="step.id" class="plan-history-step">
        <strong>{{ step.ordinal + 1 }}. {{ step.title }}</strong>
        <span v-if="step.rationale">原因：{{ step.rationale }}</span>
        <span v-if="step.expected_output">预期产出：{{ step.expected_output }}</span>
      </div>
      <p v-for="todo in plan.todos.filter((item) => item.detail)" :key="`${todo.id}-detail`">
        <strong>{{ todo.title }}：</strong>{{ todo.detail }}
      </p>
    </div>
  </section>
</template>

<style scoped>
.plan-history-card { margin: 0 0 16px; max-width: 680px; border: 1px solid #c7d2fe; border-radius: 16px; padding: 15px; background: linear-gradient(135deg, #f8faff, #eef2ff); box-shadow: 0 6px 20px rgba(79, 70, 229, .08); }
.plan-history-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 10px; }
.plan-history-title-wrap { display: flex; align-items: flex-start; gap: 9px; min-width: 0; }
.plan-history-icon { display: flex; align-items: center; justify-content: center; width: 28px; height: 28px; border-radius: 8px; color: #4338ca; background: #e0e7ff; flex: 0 0 auto; }
.plan-history-eyebrow { color: #6366f1; font-size: 10px; font-weight: 700; }
.plan-history-title { margin: 2px 0 0; color: #1e1b4b; font-size: 14px; font-weight: 700; overflow-wrap: anywhere; }
.plan-history-phase { border: 1px solid #c7d2fe; border-radius: 999px; padding: 3px 8px; color: #4338ca; background: #eef2ff; font-size: 10px; white-space: nowrap; }
.plan-history-goal { margin: 10px 0 0; color: #3730a3; font-size: 12px; line-height: 1.5; }
.plan-history-todos { display: flex; flex-direction: column; gap: 6px; margin-top: 12px; }
.plan-history-todo { display: flex; align-items: flex-start; gap: 7px; color: #373737; font-size: 12px; line-height: 1.4; }
.plan-history-toggle { display: inline-flex; align-items: center; gap: 4px; margin-top: 11px; padding: 0; border: 0; color: #4f46e5; background: transparent; font-size: 11px; cursor: pointer; }
.plan-history-details { display: flex; flex-direction: column; gap: 8px; margin-top: 9px; padding: 10px; border: 1px solid #c7d2fe; border-radius: 9px; color: #4338ca; background: rgba(255,255,255,.6); font-size: 11px; line-height: 1.5; }
.plan-history-details p { margin: 0; }
.plan-history-step { display: flex; flex-direction: column; gap: 2px; }
</style>

<script setup lang="ts">
import { ref } from 'vue';
import { Check, ChevronDown, ChevronRight, ClipboardList, X, Loader2 } from 'lucide-vue-next';
import type { PlanRuntimeState } from '../../api/planning';

defineProps<{
  plan: PlanRuntimeState;
  approving?: boolean;
}>();

const emit = defineEmits<{
  (event: 'approve'): void;
  (event: 'revoke'): void;
}>();

const detailsOpen = ref(false);
</script>

<template>
  <section class="plan-approval-card" aria-label="Plan ready for approval">
    <div class="plan-approval-header">
      <div class="plan-approval-title-wrap">
        <div class="plan-approval-icon"><ClipboardList :size="17" /></div>
        <div>
          <div class="plan-approval-eyebrow">计划已生成</div>
          <h3 class="plan-approval-title">{{ plan.title }}</h3>
        </div>
      </div>
      <span class="plan-approval-badge">待审批</span>
    </div>

    <p class="plan-approval-goal">{{ plan.goal }}</p>

    <ol v-if="plan.steps.length > 0" class="plan-approval-steps">
      <li v-for="(step, index) in plan.steps" :key="step.id" class="plan-approval-step">
        <span class="plan-approval-step-index">{{ index + 1 }}</span>
        <span>{{ step.title }}</span>
      </li>
    </ol>
    <ol v-else-if="plan.todos.length > 0" class="plan-approval-steps">
      <li v-for="(todo, index) in plan.todos" :key="todo.id" class="plan-approval-step">
        <span class="plan-approval-step-index">{{ index + 1 }}</span>
        <span>{{ todo.title }}</span>
      </li>
    </ol>

    <button
      v-if="plan.strategy || plan.steps.some((step) => step.rationale || step.expected_output) || plan.todos.some((todo) => todo.detail)"
      type="button"
      class="plan-approval-details-toggle"
      @click="detailsOpen = !detailsOpen"
    >
      <ChevronDown v-if="detailsOpen" :size="15" />
      <ChevronRight v-else :size="15" />
      {{ detailsOpen ? '收起详情' : '预览详情' }}
    </button>

    <div v-if="detailsOpen" class="plan-approval-details">
      <div v-if="plan.strategy" class="plan-approval-detail-block">
        <strong>策略</strong>
        <p>{{ plan.strategy }}</p>
      </div>
      <div v-if="plan.steps.some((step) => step.rationale || step.expected_output)" class="plan-approval-detail-block">
        <strong>步骤说明</strong>
        <div v-for="step in plan.steps" :key="`${step.id}-detail`" class="plan-approval-step-detail">
          <span>{{ step.title }}</span>
          <small v-if="step.rationale">原因：{{ step.rationale }}</small>
          <small v-if="step.expected_output">预期产出：{{ step.expected_output }}</small>
        </div>
      </div>
      <div v-if="plan.todos.some((todo) => todo.detail)" class="plan-approval-detail-block">
        <strong>任务说明</strong>
        <p v-for="todo in plan.todos.filter((item) => item.detail)" :key="`${todo.id}-detail`">
          {{ todo.title }}：{{ todo.detail }}
        </p>
      </div>
    </div>

    <div class="plan-approval-actions">
      <button
        type="button"
        class="plan-approval-approve"
        :disabled="approving"
        @click="emit('approve')"
      >
        <Loader2 v-if="approving" :size="15" class="plan-approval-spinner" />
        <Check v-else :size="15" />
        {{ approving ? '正在启动…' : '执行计划' }}
      </button>
      <button type="button" class="plan-approval-revoke" :disabled="approving" @click="emit('revoke')">
        <X :size="15" />
        撤销
      </button>
    </div>
  </section>
</template>

<style scoped>
.plan-approval-card { margin: 0 0 16px; max-width: 680px; border: 1px solid #f3c66b; border-radius: 16px; padding: 16px; background: linear-gradient(135deg, #fffdf7, #fff8e7); box-shadow: 0 8px 24px rgba(180, 120, 20, 0.12); }
.plan-approval-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; }
.plan-approval-title-wrap { display: flex; align-items: flex-start; gap: 10px; min-width: 0; }
.plan-approval-icon { display: flex; align-items: center; justify-content: center; flex: 0 0 auto; width: 30px; height: 30px; border-radius: 9px; color: #a16207; background: #fef3c7; }
.plan-approval-eyebrow { color: #a16207; font-size: 11px; font-weight: 600; }
.plan-approval-title { margin: 2px 0 0; color: #292524; font-size: 15px; font-weight: 700; overflow-wrap: anywhere; }
.plan-approval-badge { flex: 0 0 auto; border: 1px solid #f3c66b; border-radius: 999px; padding: 4px 9px; color: #a16207; background: #fef3c7; font-size: 11px; }
.plan-approval-goal { margin: 12px 0 0; color: #57534e; font-size: 13px; line-height: 1.55; }
.plan-approval-steps { display: flex; flex-direction: column; gap: 7px; margin: 14px 0 0; padding: 0; list-style: none; }
.plan-approval-step { display: flex; align-items: flex-start; gap: 9px; color: #44403c; font-size: 13px; line-height: 1.45; }
.plan-approval-step-index { display: inline-flex; align-items: center; justify-content: center; flex: 0 0 auto; width: 20px; height: 20px; border-radius: 50%; color: #92400e; background: #fde68a; font-size: 11px; font-weight: 700; }
.plan-approval-details-toggle { display: inline-flex; align-items: center; gap: 4px; margin-top: 13px; padding: 0; border: 0; color: #a16207; background: transparent; font-size: 12px; cursor: pointer; }
.plan-approval-details { display: flex; flex-direction: column; gap: 12px; margin-top: 10px; padding: 12px; border: 1px solid #f5d58d; border-radius: 10px; color: #57534e; background: rgba(255, 255, 255, 0.55); font-size: 12px; line-height: 1.5; }
.plan-approval-detail-block strong { display: block; margin-bottom: 4px; color: #44403c; }
.plan-approval-detail-block p { margin: 3px 0; }
.plan-approval-step-detail { display: flex; flex-direction: column; gap: 2px; margin-top: 6px; }
.plan-approval-step-detail small { color: #78716c; }
.plan-approval-actions { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 16px; }
.plan-approval-actions button { display: inline-flex; align-items: center; gap: 6px; border-radius: 10px; padding: 8px 12px; font-size: 13px; font-weight: 600; cursor: pointer; }
.plan-approval-actions button:disabled { cursor: not-allowed; opacity: 0.6; }
.plan-approval-approve { border: 1px solid #d97706; color: white; background: #d97706; }
.plan-approval-revoke { border: 1px solid #d6d3d1; color: #57534e; background: white; }
.plan-approval-spinner { animation: plan-spin 0.9s linear infinite; }
@keyframes plan-spin { to { transform: rotate(360deg); } }
</style>

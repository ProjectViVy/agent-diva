<script setup lang="ts">
import { ref } from 'vue';
import { Check, ChevronDown, ChevronRight, ClipboardList, Loader2, Pencil, RefreshCw } from 'lucide-vue-next';
import type { PlanRuntimeState } from '../../api/planning';

export type ExecutionContextPolicy = 'retain' | 'compact' | 'clear';

defineProps<{
  plan: PlanRuntimeState;
  approving?: boolean;
}>();

const emit = defineEmits<{
  (event: 'approve', payload: { contextPolicy: ExecutionContextPolicy }): void;
  (event: 'revoke', feedback: string): void;
  (event: 'refresh'): void;
}>();

const detailsOpen = ref(false);
const editingFeedback = ref(false);
const feedback = ref('');
const contextPolicy = ref<ExecutionContextPolicy>('compact');

const contextChoices: Array<{ value: ExecutionContextPolicy; title: string; detail: string }> = [
  { value: 'retain', title: '保留上下文（原型）', detail: '仅记录此选择，尚未传给运行时。' },
  { value: 'compact', title: '压缩上下文（原型）', detail: '仅记录此选择，尚未传给运行时。' },
  { value: 'clear', title: '清空探索（原型）', detail: '仅记录此选择，尚未传给运行时。' },
];
</script>

<template>
  <section class="plan-approval-card" aria-label="待审计划报告">
    <div class="plan-approval-header">
      <div class="plan-approval-title-wrap">
        <div class="plan-approval-icon"><ClipboardList :size="17" /></div>
        <div>
          <div class="plan-approval-eyebrow">PLAN REPORT · 待审</div>
          <h3 class="plan-approval-title">{{ plan.title }}</h3>
        </div>
      </div>
      <span class="plan-approval-badge">r{{ plan.revision ?? '?' }}</span>
    </div>

    <p class="plan-approval-goal">{{ plan.goal }}</p>

    <ol v-if="plan.steps.length > 0" class="plan-approval-steps">
      <li v-for="step in plan.steps" :key="step.id" class="plan-approval-step">
        <span class="plan-approval-step-index">{{ step.ordinal + 1 }}</span>
        <span>{{ step.title }}</span>
      </li>
    </ol>

    <button
      v-if="plan.strategy || plan.steps.some((step) => step.rationale || step.expected_output)"
      type="button"
      class="plan-approval-details-toggle"
      @click="detailsOpen = !detailsOpen"
    >
      <ChevronDown v-if="detailsOpen" :size="15" />
      <ChevronRight v-else :size="15" />
      {{ detailsOpen ? '收起报告' : '查看完整报告' }}
    </button>

    <div v-if="detailsOpen" class="plan-approval-details">
      <div v-if="plan.strategy" class="plan-approval-detail-block"><strong>策略</strong><p>{{ plan.strategy }}</p></div>
      <div class="plan-approval-detail-block">
        <strong>计划步骤</strong>
        <div v-for="step in plan.steps" :key="`${step.id}-detail`" class="plan-approval-step-detail">
          <span>{{ step.ordinal + 1 }}. {{ step.title }}</span>
          <small v-if="step.rationale">原因：{{ step.rationale }}</small>
          <small v-if="step.expected_output">预期产出：{{ step.expected_output }}</small>
        </div>
      </div>
    </div>

    <fieldset class="plan-context-choice" :disabled="approving">
      <legend>批准后如何带入执行上下文</legend>
      <label v-for="choice in contextChoices" :key="choice.value" class="plan-context-option">
        <input v-model="contextPolicy" type="radio" name="plan-context-policy" :value="choice.value" />
        <span><strong>{{ choice.title }}</strong><small>{{ choice.detail }}</small></span>
      </label>
      <p class="plan-context-note">此选择正在进行 GUI 验证；本次不会改变运行时上下文。</p>
    </fieldset>

    <div class="plan-approval-actions">
      <button type="button" class="plan-approval-approve" :disabled="approving || plan.revision == null" @click="emit('approve', { contextPolicy })">
        <Loader2 v-if="approving" :size="15" class="plan-approval-spinner" /><Check v-else :size="15" />
        {{ approving ? '正在批准…' : plan.revision == null ? '需刷新 revision' : '批准并进入执行' }}
      </button>
      <button type="button" class="plan-approval-revoke" :disabled="approving" @click="editingFeedback = !editingFeedback"><Pencil :size="15" /> 退回修改</button>
      <button type="button" class="plan-approval-refresh" :disabled="approving" @click="emit('refresh')"><RefreshCw :size="15" /> 刷新</button>
    </div>
    <div v-if="editingFeedback" class="plan-approval-feedback">
      <label for="plan-feedback">请 agent 重写或补充计划</label>
      <textarea id="plan-feedback" v-model="feedback" :disabled="approving" placeholder="例如：补充风险和验证步骤" />
      <button type="button" class="plan-approval-revoke" :disabled="approving" @click="emit('revoke', feedback)">提交修改意见</button>
    </div>
  </section>
</template>

<style scoped>
.plan-approval-card { margin: 0 0 16px; max-width: 680px; border: 1px solid #f3c66b; border-radius: 16px; padding: 16px; background: linear-gradient(135deg, #fffdf7, #fff8e7); box-shadow: 0 8px 24px rgba(180, 120, 20, .12); }
.plan-approval-header, .plan-approval-title-wrap, .plan-approval-actions { display: flex; align-items: flex-start; gap: 10px; }
.plan-approval-header { justify-content: space-between; }
.plan-approval-icon { display: grid; width: 30px; height: 30px; place-items: center; border-radius: 9px; color: #a16207; background: #fef3c7; }
.plan-approval-eyebrow, .plan-context-note { color: #a16207; font-size: 11px; font-weight: 600; }
.plan-approval-title { margin: 2px 0 0; color: #292524; font-size: 15px; }
.plan-approval-badge { border: 1px solid #f3c66b; border-radius: 999px; padding: 4px 9px; color: #a16207; background: #fef3c7; font-size: 11px; }
.plan-approval-goal { margin: 12px 0 0; color: #57534e; font-size: 13px; line-height: 1.55; }
.plan-approval-steps { display: flex; flex-direction: column; gap: 7px; margin: 14px 0 0; padding: 0; list-style: none; }
.plan-approval-step { display: flex; gap: 9px; color: #44403c; font-size: 13px; }
.plan-approval-step-index { display: grid; width: 20px; height: 20px; flex: 0 0 auto; place-items: center; border-radius: 50%; color: #92400e; background: #fde68a; font-size: 11px; font-weight: 700; }
.plan-approval-details-toggle { display: inline-flex; align-items: center; gap: 4px; margin-top: 13px; border: 0; color: #a16207; background: transparent; font-size: 12px; cursor: pointer; }
.plan-approval-details, .plan-context-choice { display: flex; flex-direction: column; gap: 9px; margin-top: 12px; padding: 12px; border: 1px solid #f5d58d; border-radius: 10px; color: #57534e; background: rgba(255, 255, 255, .55); font-size: 12px; }
.plan-approval-detail-block p { margin: 3px 0; }.plan-approval-step-detail { display: flex; flex-direction: column; gap: 2px; }.plan-approval-step-detail small { color: #78716c; }
.plan-context-choice legend { padding: 0 4px; color: #44403c; font-weight: 700; }.plan-context-option { display: flex; gap: 8px; cursor: pointer; }.plan-context-option span { display: flex; flex-direction: column; gap: 2px; }.plan-context-option small { color: #78716c; }
.plan-approval-actions { flex-wrap: wrap; margin-top: 16px; }.plan-approval-actions button, .plan-approval-feedback button { display: inline-flex; align-items: center; gap: 6px; border-radius: 10px; padding: 8px 12px; font-size: 13px; font-weight: 600; cursor: pointer; }.plan-approval-actions button:disabled, .plan-approval-feedback button:disabled { cursor: not-allowed; opacity: .6; }
.plan-approval-approve { border: 1px solid #d97706; color: white; background: #d97706; }.plan-approval-revoke, .plan-approval-refresh { border: 1px solid #d6d3d1; color: #57534e; background: white; }
.plan-approval-feedback { display: flex; flex-direction: column; gap: 7px; margin-top: 10px; color: #57534e; font-size: 12px; }.plan-approval-feedback textarea { min-height: 68px; resize: vertical; border: 1px solid #d6d3d1; border-radius: 8px; padding: 8px; font: inherit; }
.plan-approval-spinner { animation: plan-spin .9s linear infinite; } @keyframes plan-spin { to { transform: rotate(360deg); } }
</style>

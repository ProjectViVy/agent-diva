<script setup lang="ts">
import { computed, ref } from 'vue';
import { Check, ChevronDown, ChevronRight, Loader2, Pencil, RefreshCw } from '@lucide/vue';
import type { PlanRuntimeState } from '../../api/planning';
import { planReportValidationIssues } from '../../api/planning';
import PlanDocument from './PlanDocument.vue';

export type ExecutionContextPolicy = 'retain' | 'compact' | 'clear';
export type TodoPolicy = 'Never' | 'Optional' | 'Always';

const props = defineProps<{
  plan: PlanRuntimeState;
  approving?: boolean;
}>();

const validationIssues = computed(() => {
  if (props.plan.validation_issues?.length) return props.plan.validation_issues;
  return planReportValidationIssues(props.plan.markdown || props.plan.summary || props.plan.strategy);
});

const emit = defineEmits<{
  (event: 'approve', payload: {
    contextPolicy: ExecutionContextPolicy;
    todoPolicy: TodoPolicy;
    materializeTodos: boolean;
  }): void;
  (event: 'revoke', feedback: string): void;
  (event: 'refresh'): void;
  /** DN-2B: arm the approved plan as a bounded goal loop
   * (plan/decide 'start_goal'). */
  (event: 'start-goal', payload: { max_rounds: number }): void;
}>();

const detailsOpen = ref(false);
const editingFeedback = ref(false);
const feedback = ref('');
const contextPolicy = ref<ExecutionContextPolicy>('compact');
const goalRounds = ref(20);
const todoPolicy = ref<TodoPolicy>('Optional');
const materializeTodos = ref(false);

const contextChoices: Array<{ value: ExecutionContextPolicy; title: string; detail: string }> = [
  { value: 'compact', title: '压缩上下文', detail: '保留批准计划，并把探索过程压缩后进入执行。' },
  { value: 'retain', title: '保留上下文', detail: '带着当前对话和批准计划直接执行。' },
  { value: 'clear', title: '清空探索', detail: '只带批准计划进入执行，丢弃探索阶段上下文。' },
];

const todoChoices: Array<{ value: TodoPolicy; title: string; detail: string }> = [
  { value: 'Never', title: '不生成执行 TODO', detail: '批准后直接执行，不创建任务清单。' },
  { value: 'Optional', title: '按需生成 TODO', detail: '仅在下方明确勾选时从计划步骤生成。' },
  { value: 'Always', title: '必须生成 TODO', detail: '批准时始终从计划步骤生成执行清单。' },
];

const approvalBlocked = computed(() => props.plan.revision == null || validationIssues.value.length > 0);

function approve() {
  emit('approve', {
    contextPolicy: contextPolicy.value,
    todoPolicy: todoPolicy.value,
    materializeTodos: todoPolicy.value === 'Always'
      || (todoPolicy.value === 'Optional' && materializeTodos.value),
  });
}
</script>

<template>
  <section class="plan-approval-card" aria-label="待审批计划">
    <div class="plan-approval-header">
      <div class="plan-approval-title-wrap">
        <div class="plan-approval-icon"><Pencil :size="17" /></div>
        <div>
          <div class="plan-approval-eyebrow">计划 · 待审批</div>
          <h3 class="plan-approval-title">{{ plan.title }}</h3>
        </div>
      </div>
      <span class="plan-approval-badge">r{{ plan.revision ?? '?' }}</span>
    </div>

    <p class="plan-approval-goal">{{ plan.goal }}</p>

    <div v-if="validationIssues.length" class="plan-approval-warnings" role="status">
      <strong>计划不完整，请编辑补全后再批准</strong>
      <ul>
        <li v-for="issue in validationIssues" :key="issue">{{ issue }}</li>
      </ul>
    </div>

    <button type="button" class="ui-button ui-button--ghost ui-button--compact plan-approval-details-toggle" @click="detailsOpen = !detailsOpen">
      <ChevronDown v-if="detailsOpen" :size="15" />
      <ChevronRight v-else :size="15" />
      {{ detailsOpen ? '收起计划' : '查看完整计划' }}
    </button>

    <div v-if="detailsOpen" class="plan-approval-details">
      <PlanDocument :markdown="plan.markdown || plan.summary || plan.strategy || plan.goal" />
    </div>

    <fieldset class="plan-context-choice" :disabled="approving">
      <legend>批准后的执行上下文</legend>
      <label v-for="choice in contextChoices" :key="choice.value" class="plan-context-option">
        <input v-model="contextPolicy" type="radio" name="plan-context-policy" :value="choice.value" />
        <span><strong>{{ choice.title }}</strong><small>{{ choice.detail }}</small></span>
      </label>
    </fieldset>

    <fieldset class="plan-context-choice" :disabled="approving">
      <legend>执行 TODO 策略</legend>
      <label v-for="choice in todoChoices" :key="choice.value" class="plan-context-option">
        <input v-model="todoPolicy" type="radio" name="plan-todo-policy" :value="choice.value" />
        <span><strong>{{ choice.title }}</strong><small>{{ choice.detail }}</small></span>
      </label>
      <label v-if="todoPolicy === 'Optional'" class="plan-context-option">
        <input v-model="materializeTodos" type="checkbox" />
        <span><strong>本次生成执行 TODO</strong><small>每个已批准步骤生成一个待办项。</small></span>
      </label>
    </fieldset>

    <div class="plan-approval-actions">
      <button type="button" class="ui-button ui-button--primary plan-approval-approve" :disabled="approving || approvalBlocked" @click="approve">
        <Loader2 v-if="approving" :size="15" class="plan-approval-spinner" /><Check v-else :size="15" />
        {{ approving ? '正在批准...' : plan.revision == null ? '需要刷新 revision' : validationIssues.length ? '请先补全计划' : '批准并开始执行' }}
      </button>
      <button type="button" class="ui-button ui-button--ghost plan-approval-revoke" :disabled="approving" @click="editingFeedback = !editingFeedback"><Pencil :size="15" /> 编辑计划</button>
      <button type="button" class="ui-button ui-button--ghost ui-button--compact plan-approval-refresh" :disabled="approving" @click="emit('refresh')"><RefreshCw :size="15" /> 刷新</button>
    </div>

    <div class="plan-goal-start">
      <label for="plan-goal-rounds">目标循环轮数上限</label>
      <input class="ui-input"
        id="plan-goal-rounds"
        v-model.number="goalRounds"
        type="number"
        min="1"
        max="1000"
        :disabled="approving"
      />
      <button
        type="button"
        class="ui-button ui-button--outline plan-approval-revoke"
        :disabled="approving || approvalBlocked || !goalRounds || goalRounds < 1"
        @click="emit('start-goal', { max_rounds: goalRounds })"
      >批准并启动目标循环</button>
    </div>

    <div v-if="editingFeedback" class="plan-approval-feedback">
      <label for="plan-feedback">修改意见</label>
      <textarea class="ui-input" id="plan-feedback" v-model="feedback" :disabled="approving" placeholder="例如：补充风险、缩小范围、改写验证方法" />
      <button type="button" class="ui-button ui-button--outline plan-approval-revoke" :disabled="approving" @click="emit('revoke', feedback)">提交修改意见</button>
    </div>
  </section>
</template>

<style scoped>
.plan-approval-card { margin: 0 0 16px; max-width: 680px; border: 1px solid color-mix(in srgb, var(--warning) 34%, var(--border)); border-radius: 12px; padding: 16px; background: color-mix(in srgb, var(--card) 94%, var(--warning-soft)); box-shadow: var(--shadow-sm); }
.plan-approval-header, .plan-approval-title-wrap, .plan-approval-actions { display: flex; align-items: flex-start; gap: 10px; }
.plan-approval-header { justify-content: space-between; }
.plan-approval-icon { display: grid; width: 30px; height: 30px; place-items: center; border-radius: 9px; color: var(--warning); background: var(--warning-soft); }
.plan-approval-eyebrow { color: var(--warning); font-size: 11px; font-weight: 700; letter-spacing: .08em; text-transform: uppercase; }
.plan-approval-title { margin: 2px 0 0; color: var(--foreground); font-size: 15px; }
.plan-approval-badge { border: 1px solid var(--warning); border-radius: 999px; padding: 4px 9px; color: var(--warning); background: var(--warning-soft); font-size: 11px; }
.plan-approval-goal { margin: 12px 0 0; color: var(--muted-foreground); font-size: 13px; line-height: 1.55; }
.plan-approval-warnings { margin-top: 12px; border: 1px solid var(--warning); border-radius: 10px; padding: 10px 12px; color: var(--warning); background: var(--warning-soft); font-size: 12px; line-height: 1.45; }
.plan-approval-warnings strong { display: block; margin-bottom: 6px; font-size: 12px; }
.plan-approval-warnings ul { margin: 0; padding-left: 1.1rem; }
.plan-approval-warnings li { margin: 2px 0; }
.plan-approval-details-toggle { display: inline-flex; align-items: center; gap: 4px; margin-top: 13px; }
.plan-approval-details, .plan-context-choice { display: flex; flex-direction: column; gap: 9px; margin-top: 12px; padding: 12px; border: 1px solid var(--warning); border-radius: 10px; color: var(--muted-foreground); background: var(--warning-soft); font-size: 12px; }
.plan-context-choice legend { padding: 0 4px; color: var(--foreground); font-weight: 700; }
.plan-context-option { display: flex; gap: 8px; cursor: pointer; }
.plan-context-option span { display: flex; flex-direction: column; gap: 2px; }
.plan-context-option small { color: var(--muted-foreground); }
.plan-approval-actions { flex-wrap: wrap; margin-top: 16px; }
.plan-approval-actions button, .plan-approval-feedback button { display: inline-flex; align-items: center; gap: 6px; }
.plan-approval-feedback { display: flex; flex-direction: column; gap: 7px; margin-top: 10px; color: var(--muted-foreground); font-size: 12px; }
.plan-approval-feedback textarea { min-height: 68px; resize: vertical; border: 1px solid var(--warning); border-radius: 8px; padding: 8px; font: inherit; }
.plan-goal-start { display: flex; align-items: center; gap: 8px; margin-top: 10px; color: var(--muted-foreground); font-size: 12px; }
.plan-goal-start input { width: 72px; border: 1px solid var(--border); border-radius: 8px; padding: 6px 8px; font: inherit; }
.plan-goal-start button { display: inline-flex; align-items: center; gap: 6px; border-radius: 10px; padding: 8px 12px; font-size: 13px; font-weight: 600; cursor: pointer; }
.plan-goal-start button:disabled { cursor: not-allowed; opacity: .6; }
.plan-approval-spinner { animation: plan-spin .9s linear infinite; }
@keyframes plan-spin { to { transform: rotate(360deg); } }
</style>

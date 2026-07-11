<script setup lang="ts">
import { computed, ref } from 'vue';
import { Check, ChevronDown, ChevronRight, Loader2, Pencil, RefreshCw } from 'lucide-vue-next';
import type { PlanRuntimeState } from '../../api/planning';
import {
  planDocumentMarkdown,
  planReportValidationIssues,
  resolvePlanDisplayTitle,
} from '../../api/planning';
import PlanDocument from './PlanDocument.vue';

export type ExecutionContextPolicy = 'retain' | 'compact' | 'clear';

const props = defineProps<{
  plan: PlanRuntimeState;
  approving?: boolean;
}>();

const displayTitle = computed(() => resolvePlanDisplayTitle(props.plan));
const documentMarkdown = computed(() => planDocumentMarkdown(props.plan));

const validationIssues = computed(() => {
  if (props.plan.validation_issues?.length) return props.plan.validation_issues;
  return planReportValidationIssues(documentMarkdown.value);
});

const emit = defineEmits<{
  (event: 'approve', payload: { contextPolicy: ExecutionContextPolicy }): void;
  (event: 'revoke', feedback: string): void;
  (event: 'refresh'): void;
}>();

/** Context policy panel; plan body is always visible. */
const detailsOpen = ref(false);
const editingFeedback = ref(false);
const feedback = ref('');
const contextPolicy = ref<ExecutionContextPolicy>('compact');

const contextChoices: Array<{ value: ExecutionContextPolicy; title: string; detail: string }> = [
  { value: 'compact', title: '压缩上下文', detail: '保留批准计划，并把探索过程压缩后进入执行。' },
  { value: 'retain', title: '保留上下文', detail: '带着当前对话和批准计划直接执行。' },
  { value: 'clear', title: '清空探索', detail: '只带批准计划进入执行，丢弃探索阶段上下文。' },
];
</script>

<template>
  <section class="plan-approval-card" aria-label="待审批计划">
    <div class="plan-approval-header">
      <div class="plan-approval-title-wrap">
        <div class="plan-approval-icon"><Pencil :size="17" /></div>
        <div>
          <div class="plan-approval-eyebrow">待审批</div>
          <h3 class="plan-approval-title">{{ displayTitle }}</h3>
        </div>
      </div>
      <span class="plan-approval-badge">r{{ plan.revision ?? '?' }}</span>
    </div>

    <div v-if="validationIssues.length" class="plan-approval-warnings" role="status">
      <strong>章节不完整，仍可批准或点编辑继续完善</strong>
      <ul>
        <li v-for="issue in validationIssues" :key="issue">{{ issue }}</li>
      </ul>
    </div>

    <div class="plan-approval-body">
      <PlanDocument :markdown="documentMarkdown" />
    </div>

    <button type="button" class="plan-approval-details-toggle" @click="detailsOpen = !detailsOpen">
      <ChevronDown v-if="detailsOpen" :size="15" />
      <ChevronRight v-else :size="15" />
      {{ detailsOpen ? '收起上下文选项' : '展开上下文选项与操作说明' }}
    </button>

    <div v-if="detailsOpen" class="plan-approval-details">
      <fieldset class="plan-context-choice" :disabled="approving">
        <legend>批准后的执行上下文</legend>
        <label v-for="choice in contextChoices" :key="choice.value" class="plan-context-option">
          <input v-model="contextPolicy" type="radio" name="plan-context-policy" :value="choice.value" />
          <span><strong>{{ choice.title }}</strong><small>{{ choice.detail }}</small></span>
        </label>
      </fieldset>
    </div>

    <div class="plan-approval-actions">
      <button type="button" class="plan-approval-approve" :disabled="approving || plan.revision == null" @click="emit('approve', { contextPolicy })">
        <Loader2 v-if="approving" :size="15" class="plan-approval-spinner" /><Check v-else :size="15" />
        {{ approving ? '正在批准...' : plan.revision == null ? '需要刷新 revision' : '批准并开始执行' }}
      </button>
      <button type="button" class="plan-approval-revoke" :disabled="approving" @click="editingFeedback = !editingFeedback"><Pencil :size="15" /> 编辑计划</button>
      <button type="button" class="plan-approval-refresh" :disabled="approving" @click="emit('refresh')"><RefreshCw :size="15" /> 刷新</button>
    </div>

    <div v-if="editingFeedback" class="plan-approval-feedback">
      <label for="plan-feedback">修改意见</label>
      <textarea id="plan-feedback" v-model="feedback" :disabled="approving" placeholder="例如：补充风险、缩小范围、改写验证方法" />
      <button type="button" class="plan-approval-revoke" :disabled="approving" @click="emit('revoke', feedback)">提交修改意见</button>
    </div>
  </section>
</template>

<style scoped>
.plan-approval-card { margin: 0 0 16px; max-width: 680px; border: 1px solid color-mix(in srgb, #2563eb 34%, var(--line, #d9dce3)); border-radius: 12px; padding: 16px; background: color-mix(in srgb, var(--panel-solid, #fff) 94%, #eff6ff); box-shadow: 0 12px 30px rgba(30, 64, 175, .1); }
.plan-approval-header, .plan-approval-title-wrap, .plan-approval-actions { display: flex; align-items: flex-start; gap: 10px; }
.plan-approval-header { justify-content: space-between; }
.plan-approval-icon { display: grid; width: 30px; height: 30px; place-items: center; border-radius: 9px; color: #1d4ed8; background: #dbeafe; }
.plan-approval-eyebrow { color: #1d4ed8; font-size: 11px; font-weight: 700; letter-spacing: .08em; text-transform: uppercase; }
.plan-approval-title { margin: 2px 0 0; color: var(--text, #1f2937); font-size: 15px; line-height: 1.35; }
.plan-approval-badge { border: 1px solid #bfdbfe; border-radius: 999px; padding: 4px 9px; color: #1d4ed8; background: #dbeafe; font-size: 11px; flex-shrink: 0; }
.plan-approval-body { margin-top: 12px; }
/* Card header already shows the title — hide duplicate H1 from markdown body. */
.plan-approval-body :deep(.plan-document h1:first-child) { display: none; }
.plan-approval-warnings { margin-top: 12px; border: 1px solid #fcd34d; border-radius: 10px; padding: 10px 12px; color: #92400e; background: #fffbeb; font-size: 12px; line-height: 1.45; }
.plan-approval-warnings strong { display: block; margin-bottom: 6px; font-size: 12px; }
.plan-approval-warnings ul { margin: 0; padding-left: 1.1rem; }
.plan-approval-warnings li { margin: 2px 0; }
.plan-approval-details-toggle { display: inline-flex; align-items: center; gap: 4px; margin-top: 13px; border: 0; color: #1d4ed8; background: transparent; font-size: 12px; cursor: pointer; }
.plan-approval-details, .plan-context-choice { display: flex; flex-direction: column; gap: 9px; margin-top: 12px; padding: 12px; border: 1px solid #bfdbfe; border-radius: 10px; color: var(--text-muted, #667085); background: rgba(255, 255, 255, .62); font-size: 12px; }
.plan-context-choice { margin-top: 0; padding: 0; border: 0; background: transparent; }
.plan-context-choice legend { padding: 0 4px; color: var(--text, #1f2937); font-weight: 700; }
.plan-context-option { display: flex; gap: 8px; cursor: pointer; }
.plan-context-option span { display: flex; flex-direction: column; gap: 2px; }
.plan-context-option small { color: var(--text-muted, #667085); }
.plan-approval-actions { flex-wrap: wrap; margin-top: 16px; }
.plan-approval-actions button, .plan-approval-feedback button { display: inline-flex; align-items: center; gap: 6px; border-radius: 10px; padding: 8px 12px; font-size: 13px; font-weight: 600; cursor: pointer; }
.plan-approval-actions button:disabled, .plan-approval-feedback button:disabled { cursor: not-allowed; opacity: .6; }
.plan-approval-approve { border: 1px solid #2563eb; color: white; background: #2563eb; }
.plan-approval-revoke, .plan-approval-refresh { border: 1px solid #d6dce8; color: var(--text, #1f2937); background: white; }
.plan-approval-feedback { display: flex; flex-direction: column; gap: 7px; margin-top: 10px; color: var(--text-muted, #667085); font-size: 12px; }
.plan-approval-feedback textarea { min-height: 68px; resize: vertical; border: 1px solid #d6dce8; border-radius: 8px; padding: 8px; font: inherit; }
.plan-approval-spinner { animation: plan-spin .9s linear infinite; }
@keyframes plan-spin { to { transform: rotate(360deg); } }
</style>

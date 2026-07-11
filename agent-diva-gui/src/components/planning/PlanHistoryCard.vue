<script setup lang="ts">
import { computed } from 'vue';
import PlanDocument from './PlanDocument.vue';
import {
  planDocumentMarkdown,
  resolvePlanDisplayTitle,
  type PlanRuntimeState,
} from '../../api/planning';

const props = defineProps<{ plan: PlanRuntimeState }>();

const displayTitle = computed(() => resolvePlanDisplayTitle(props.plan));
const documentMarkdown = computed(() => planDocumentMarkdown(props.plan));
const phaseLabel = computed(() => {
  const phase = props.plan.phase || props.plan.status || '';
  const map: Record<string, string> = {
    AwaitingApproval: '待审批',
    Execute: '执行中',
    Verify: '验证中',
    Completed: '已完成',
    Failed: '失败',
    Partial: '部分完成',
    Approved: '已批准',
    InProgress: '进行中',
  };
  return map[phase] || phase;
});
</script>

<template>
  <section class="plan-history-card" aria-label="计划历史">
    <header class="plan-history-card__header">
      <div class="plan-history-card__label">
        <span class="plan-history-card__badge">历史计划</span>
        <span v-if="phaseLabel" class="plan-history-card__phase">{{ phaseLabel }}</span>
      </div>
      <h3 class="plan-history-card__title">{{ displayTitle }}</h3>
    </header>
    <PlanDocument :markdown="documentMarkdown" />
  </section>
</template>

<style scoped>
.plan-history-card { margin: 0 0 16px; max-width: 680px; padding: clamp(18px, 3vw, 30px); border: 1px solid color-mix(in srgb, var(--accent, #2563eb) 34%, var(--line, #d9dce3)); border-radius: 12px; background: color-mix(in srgb, var(--panel-solid, var(--panel, #fff)) 94%, #eff6ff); box-shadow: 0 14px 34px rgba(30, 64, 175, .08); }
.plan-history-card__header { margin-bottom: 1rem; }
.plan-history-card__label { display: inline-flex; align-items: center; gap: .45rem; margin-bottom: .55rem; color: #1d4ed8; font-family: ui-sans-serif, system-ui, sans-serif; font-size: .69rem; font-weight: 750; letter-spacing: .08em; line-height: 1; }
.plan-history-card__badge { display: inline-flex; align-items: center; min-height: 1.55rem; padding: 0 .52rem; border-radius: 999px; background: #dbeafe; }
.plan-history-card__phase { padding: .15rem .45rem; border-radius: 999px; color: #1e40af; background: #eff6ff; font-size: .62rem; letter-spacing: .04em; font-weight: 650; }
.plan-history-card__title { margin: 0; color: var(--text, #1f2937); font-family: ui-sans-serif, system-ui, sans-serif; font-size: 1.05rem; font-weight: 700; line-height: 1.35; }
/* Avoid double H1: document title is already in the card header. */
.plan-history-card :deep(.plan-document h1:first-child) { display: none; }
</style>

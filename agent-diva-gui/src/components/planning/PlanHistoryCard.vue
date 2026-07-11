<script setup lang="ts">
import PlanDocument from './PlanDocument.vue';
import type { PlanRuntimeState } from '../../api/planning';

defineProps<{ plan: PlanRuntimeState }>();
</script>

<template>
  <section class="plan-history-card" aria-label="计划文档历史">
    <div class="plan-history-card__label"><span aria-hidden="true">计划</span><span>Plan</span></div>
    <PlanDocument
      :title="plan.title"
      :goal="plan.goal"
      :strategy="plan.strategy"
      :steps="plan.steps"
    />
  </section>
</template>

<style scoped>
.plan-history-card { margin: 0 0 16px; max-width: 680px; padding: clamp(18px, 3vw, 30px); border: 1px solid color-mix(in srgb, var(--accent, #2563eb) 34%, var(--line, #d9dce3)); border-radius: 12px; background: color-mix(in srgb, var(--panel-solid, var(--panel, #fff)) 94%, #eff6ff); box-shadow: 0 14px 34px rgba(30, 64, 175, .08); }
.plan-history-card__label { display: inline-flex; align-items: center; gap: .45rem; margin-bottom: 1rem; color: #1d4ed8; font-family: ui-sans-serif, system-ui, sans-serif; font-size: .69rem; font-weight: 750; letter-spacing: .12em; line-height: 1; text-transform: uppercase; }
.plan-history-card__label span:first-child { display: inline-flex; align-items: center; min-height: 1.55rem; padding: 0 .52rem; border-radius: 999px; background: #dbeafe; }
.plan-history-card__label span:last-child { color: #60a5fa; }
</style>

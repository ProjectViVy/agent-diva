<script setup lang="ts">
import PlanDocument from './PlanDocument.vue';
import type { PlanRuntimeState } from '../../api/planning';

defineProps<{ plan: PlanRuntimeState }>();
</script>

<template>
  <section class="plan-history-card" aria-label="计划文档历史">
    <PlanDocument
      :title="plan.title"
      :goal="plan.goal"
      :strategy="plan.strategy"
      :steps="plan.steps"
    />
  </section>
</template>

<style scoped>
.plan-history-card { margin: 0 0 16px; max-width: 680px; border: 1px solid #c7d2fe; border-radius: 16px; padding: 18px; background: linear-gradient(135deg, #f8faff, #eef2ff); box-shadow: 0 6px 20px rgba(79, 70, 229, .08); }
</style>

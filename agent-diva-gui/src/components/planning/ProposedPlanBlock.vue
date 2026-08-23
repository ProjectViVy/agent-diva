<script setup lang="ts">
import PlanDocument from './PlanDocument.vue';

defineProps<{
  markdown: string;
}>();
</script>

<template>
  <section class="proposed-plan-block" aria-label="计划消息">
    <div class="proposed-plan-block__label">
      <span aria-hidden="true">计划</span>
      <span>Plan</span>
    </div>
    <PlanDocument :markdown="markdown || '# Plan'" />
  </section>
</template>

<style scoped>
.proposed-plan-block {
  margin: 0.55rem 0 0.15rem;
  max-width: 680px;
  padding: clamp(14px, 2.4vw, 22px);
  border: 1px solid color-mix(in srgb, var(--accent, #2563eb) 34%, var(--line, #d9dce3));
  border-radius: 12px;
  background: color-mix(in srgb, var(--panel-solid, var(--panel, #fff)) 94%, #eff6ff);
  box-shadow: 0 10px 26px rgba(30, 64, 175, 0.08);
}
.proposed-plan-block__label {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  margin-bottom: 0.85rem;
  color: var(--accent, #1d4ed8);
  font-family: ui-sans-serif, system-ui, sans-serif;
  font-size: 0.69rem;
  font-weight: 750;
  letter-spacing: 0.12em;
  line-height: 1;
  text-transform: uppercase;
}
.proposed-plan-block__label span:first-child {
  display: inline-flex;
  align-items: center;
  min-height: 1.55rem;
  padding: 0 0.52rem;
  border-radius: 999px;
  background: var(--accent-bg-light, #dbeafe);
}
.proposed-plan-block__label span:last-child {
  color: #60a5fa;
}
</style>

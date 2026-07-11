<script setup lang="ts">
import { computed } from 'vue';
import MarkdownIt from 'markdown-it';

interface PlanDocumentStep {
  ordinal: number;
  title: string;
  rationale?: string | null;
  expected_output?: string | null;
}

const props = defineProps<{
  title: string;
  goal: string;
  strategy?: string | null;
  steps: PlanDocumentStep[];
  assumptions?: string[];
  risks?: string[];
  openQuestions?: string[];
}>();

const markdown = new MarkdownIt({ html: false, breaks: true, linkify: true });

const renderedPlan = computed(() => {
  const lines = [`# ${props.title}`, '', '## 目标', props.goal];
  if (props.strategy) lines.push('', '## 策略', props.strategy);
  lines.push('', '## 计划步骤');
  for (const step of props.steps) {
    lines.push(`${step.ordinal + 1}. ${step.title}`);
    if (step.rationale) lines.push(`   - 原因：${step.rationale}`);
    if (step.expected_output) lines.push(`   - 预期产出：${step.expected_output}`);
  }
  if (props.assumptions?.length) lines.push('', '## 假设', ...props.assumptions.map((item) => `- ${item}`));
  if (props.risks?.length) lines.push('', '## 风险', ...props.risks.map((item) => `- ${item}`));
  if (props.openQuestions?.length) lines.push('', '## 未决问题', ...props.openQuestions.map((item) => `- ${item}`));
  return markdown.render(lines.join('\n'));
});
</script>

<template>
  <article class="plan-document markdown-body" v-html="renderedPlan" />
</template>

<style scoped>
.plan-document { color: var(--text, #1f2937); line-height: 1.65; }
.plan-document :deep(h1) { margin: 0 0 1rem; font-size: 1.2rem; }.plan-document :deep(h2) { margin: 1.25rem 0 .45rem; font-size: .95rem; }.plan-document :deep(p), .plan-document :deep(ol), .plan-document :deep(ul) { margin: .4rem 0; }.plan-document :deep(li) { margin: .25rem 0; }
</style>

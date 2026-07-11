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
.plan-document { max-width: 46rem; color: var(--text, #1f2937); font-family: ui-serif, Georgia, Cambria, serif; font-size: .94rem; line-height: 1.78; text-wrap: pretty; }
.plan-document :deep(h1) { margin: 0 0 1.45rem; padding-bottom: .9rem; border-bottom: 1px solid color-mix(in srgb, var(--accent, #2563eb) 28%, var(--line, #d9dce3)); color: #1e3a8a; font-family: ui-sans-serif, system-ui, sans-serif; font-size: clamp(1.25rem, 2vw, 1.7rem); font-weight: 700; letter-spacing: -.025em; line-height: 1.2; }
.plan-document :deep(h2) { display: flex; align-items: center; gap: .65rem; margin: 1.9rem 0 .65rem; color: #1d4ed8; font-family: ui-sans-serif, system-ui, sans-serif; font-size: .72rem; font-weight: 750; letter-spacing: .1em; line-height: 1.2; text-transform: uppercase; }
.plan-document :deep(h2::after) { height: 1px; flex: 1; content: ''; background: color-mix(in srgb, #60a5fa 38%, transparent); }
.plan-document :deep(p) { margin: .55rem 0; }.plan-document :deep(ol), .plan-document :deep(ul) { margin: .7rem 0; padding-left: 1.45rem; }.plan-document :deep(li) { padding-left: .2rem; margin: .42rem 0; }.plan-document :deep(li::marker) { color: #2563eb; font-family: ui-sans-serif, system-ui, sans-serif; font-size: .78em; font-weight: 700; }
.plan-document :deep(li > ul) { margin: .18rem 0 .45rem; color: var(--text-muted, #667085); font-size: .9em; }.plan-document :deep(a) { color: #2563eb; text-decoration-thickness: 1px; text-underline-offset: 3px; }
</style>

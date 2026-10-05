<script setup lang="ts">
import MarkdownIt from 'markdown-it';

const props = defineProps<{
  markdown: string;
}>();

const renderer = new MarkdownIt({ html: false, breaks: true, linkify: true });
</script>

<template>
  <article class="plan-document markdown-body" v-html="renderer.render(props.markdown || '# Plan')" />
</template>

<style scoped>
.plan-document { max-width: 46rem; color: var(--foreground); font-family: ui-serif, Georgia, Cambria, serif; font-size: .94rem; line-height: 1.78; text-wrap: pretty; }
.plan-document :deep(h1) { margin: 0 0 1.45rem; padding-bottom: .9rem; border-bottom: 1px solid color-mix(in srgb, var(--primary) 28%, var(--border)); color: var(--info); font-family: ui-sans-serif, system-ui, sans-serif; font-size: clamp(1.25rem, 2vw, 1.7rem); font-weight: 700; letter-spacing: 0; line-height: 1.2; }
.plan-document :deep(h2) { display: flex; align-items: center; gap: .65rem; margin: 1.9rem 0 .65rem; color: var(--info); font-family: ui-sans-serif, system-ui, sans-serif; font-size: .72rem; font-weight: 750; letter-spacing: .1em; line-height: 1.2; text-transform: uppercase; }
.plan-document :deep(h2::after) { height: 1px; flex: 1; content: ''; background: color-mix(in srgb, var(--info-soft) 38%, transparent); }
.plan-document :deep(p) { margin: .55rem 0; }
.plan-document :deep(ol), .plan-document :deep(ul) { margin: .7rem 0; padding-left: 1.45rem; }
.plan-document :deep(li) { padding-left: .2rem; margin: .42rem 0; }
.plan-document :deep(li::marker) { color: var(--info); font-family: ui-sans-serif, system-ui, sans-serif; font-size: .78em; font-weight: 700; }
.plan-document :deep(li > ul) { margin: .18rem 0 .45rem; color: var(--muted-foreground); font-size: .9em; }
.plan-document :deep(a) { color: var(--info); text-decoration-thickness: 1px; text-underline-offset: 3px; }
</style>

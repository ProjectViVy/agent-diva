<script setup lang="ts">
import { computed } from 'vue';
import MarkdownIt from 'markdown-it';
import hljs from 'highlight.js';
import ProposedPlanBlock from './ProposedPlanBlock.vue';
import { demuxProposedPlan } from './proposedPlanMessage';

const props = defineProps<{
  content: string;
}>();

const escapeHtml = (text: string): string =>
  text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');

// Shared renderer — one instance for all agent bubbles.
const md =
  (globalThis as { __agentMsgMd?: MarkdownIt }).__agentMsgMd ??
  new MarkdownIt({
    html: false,
    linkify: true,
    breaks: true,
    highlight(str: string, lang: string): string {
      if (lang && hljs.getLanguage(lang)) {
        try {
          return (
            '<pre class="hljs"><code>' +
            hljs.highlight(str, { language: lang, ignoreIllegals: true }).value +
            '</code></pre>'
          );
        } catch {
          /* fall through */
        }
      }
      return '<pre class="hljs"><code>' + escapeHtml(str) + '</code></pre>';
    },
  });
(globalThis as { __agentMsgMd?: MarkdownIt }).__agentMsgMd = md;

const parts = computed(() => demuxProposedPlan(props.content));
</script>

<template>
  <div class="agent-message-body">
    <template v-if="parts.hasPlan">
      <div
        v-if="parts.preface"
        class="markdown-body"
        v-html="md.render(parts.preface)"
      />
      <ProposedPlanBlock :markdown="parts.planMarkdown" />
      <div
        v-if="parts.epilogue"
        class="markdown-body agent-message-body__epilogue"
        v-html="md.render(parts.epilogue)"
      />
    </template>
    <div v-else class="markdown-body" v-html="md.render(content || '')" />
  </div>
</template>

<style scoped>
.agent-message-body__epilogue {
  margin-top: 0.55rem;
}
</style>

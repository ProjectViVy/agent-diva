<template>
  <details class="thinking-block" :open="expanded">
    <summary class="thinking-summary" @click.prevent="toggleExpanded">
      <span class="thinking-icon">🧠</span>
      <span class="thinking-label">{{ $t('chat.thinkingProcess') }}</span>
      <span v-if="thinkingMs && thinkingMs > 0" class="thinking-duration">({{ formatDuration(thinkingMs) }})</span>
      <span class="thinking-toggle">{{ expanded ? '▾' : '▸' }}</span>
    </summary>
    <div class="thinking-content">
      <pre>{{ content }}</pre>
    </div>
  </details>
</template>

<script setup lang="ts">
import { ref } from 'vue'

defineProps<{
  content: string
  thinkingMs?: number
}>()

const expanded = ref(false)

function toggleExpanded() {
  expanded.value = !expanded.value
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}
</script>

<style scoped>
.thinking-block {
  margin: 8px 0;
  border: 1px solid var(--color-border, #e5e7eb);
  border-radius: 8px;
  background: var(--color-bg-secondary, #f9fafb);
}
.thinking-summary {
  padding: 6px 12px;
  cursor: pointer;
  font-size: 13px;
  color: var(--color-text-muted, #6b7280);
  display: flex;
  align-items: center;
  gap: 6px;
  user-select: none;
  list-style: none;
}
.thinking-summary::-webkit-details-marker {
  display: none;
}
.thinking-summary:hover {
  background: var(--color-bg-hover, #f3f4f6);
}
.thinking-icon {
  font-size: 14px;
}
.thinking-duration {
  opacity: 0.6;
}
.thinking-toggle {
  font-size: 12px;
}
.thinking-content {
  padding: 8px 12px;
  border-top: 1px solid var(--color-border, #e5e7eb);
  max-height: 300px;
  overflow-y: auto;
}
.thinking-content pre {
  margin: 0;
  white-space: pre-wrap;
  font-size: 12px;
  color: var(--color-text-muted, #6b7280);
  font-family: inherit;
}
</style>

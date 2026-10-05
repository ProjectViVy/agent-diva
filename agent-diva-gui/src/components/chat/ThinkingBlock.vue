<template>
  <div class="thinking-card">
    <!-- Header -->
    <div class="thinking-header" :class="{ 'thinking-header-expanded': isExpanded }">
      <button
        type="button"
        class="ui-button ui-button--ghost ui-button--compact thinking-header-toggle"
        :aria-expanded="isExpanded"
        :aria-controls="contentId"
        @click="toggleExpanded"
      >
        <Brain :size="16" class="thinking-icon" />
        <span class="thinking-label">{{ $t('chat.thinkingProcess') }}</span>
        <span v-if="thinkingMs && thinkingMs > 0" class="thinking-duration">
          ({{ formatDuration(thinkingMs) }})
        </span>
      </button>
      <div class="thinking-header-actions">
        <button
          type="button"
          class="ui-button ui-button--ghost thinking-expand-btn ui-button--compact ui-button--icon"
          :aria-expanded="isExpanded"
          :aria-controls="contentId"
          :title="isExpanded ? $t('chat.hideDetails') : $t('chat.viewDetails')"
          @click="toggleExpanded"
        >
          <span class="thinking-action-label">{{ isExpanded ? $t('chat.hideDetails') : $t('chat.viewDetails') }}</span>
          <ChevronDown
            :size="16"
            class="thinking-chevron"
            :class="{ 'thinking-chevron-rotated': !isExpanded }"
          />
        </button>
      </div>
    </div>

    <!-- Content with transition -->
    <Transition
      name="thinking-expand"
      @enter="onEnter"
      @after-enter="onAfterEnter"
      @leave="onLeave"
    >
      <div :id="contentId" v-show="isExpanded" class="thinking-content-wrapper">
        <div class="thinking-content">
          <pre class="thinking-pre">{{ content }}</pre>
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { Brain, ChevronDown } from '@lucide/vue'

const props = withDefaults(defineProps<{
  content: string
  thinkingMs?: number
  expanded?: boolean
}>(), {
  expanded: undefined,
})

const emit = defineEmits<{
  (event: 'update:expanded', expanded: boolean): void
}>()

const internalExpanded = ref(false)
const isExpanded = computed(() => props.expanded ?? internalExpanded.value)
const contentId = computed(() => `thinking-content-${Math.random().toString(36).slice(2)}`)

function toggleExpanded() {
  const nextExpanded = !isExpanded.value
  if (props.expanded === undefined) {
    internalExpanded.value = nextExpanded
  }
  emit('update:expanded', nextExpanded)
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

// Transition hooks for height animation
function onEnter(el: Element) {
  const htmlEl = el as HTMLElement
  htmlEl.style.height = '0px'
  // Force reflow
  void htmlEl.offsetHeight
  htmlEl.style.height = htmlEl.scrollHeight + 'px'
}

function onAfterEnter(el: Element) {
  const htmlEl = el as HTMLElement
  htmlEl.style.height = 'auto'
}

function onLeave(el: Element) {
  const htmlEl = el as HTMLElement
  htmlEl.style.height = htmlEl.scrollHeight + 'px'
  // Force reflow
  void htmlEl.offsetHeight
  htmlEl.style.height = '0px'
}
</script>

<style scoped>
.thinking-card {
  margin: 8px 0;
  border-radius: var(--radius-lg);
  border: 1px solid var(--border);
  background: var(--card);
  box-shadow: var(--shadow-md);
  overflow: hidden;
  transition: box-shadow 0.2s ease;
}

.thinking-card:hover {
  box-shadow: var(--shadow-sm);
}

.thinking-header {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  column-gap: 12px;
  padding: 10px 14px;
  border-radius: var(--radius-lg);
  transition: background-color 0.2s ease, border-radius 0.2s ease;
}

.thinking-header:hover {
  background: var(--accent);
}

.thinking-header-expanded {
  border-radius: var(--radius-lg) var(--radius-lg) 0 0;
}

.thinking-header-toggle {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  text-align: left;
}

.thinking-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  white-space: nowrap;
}

.thinking-expand-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 28px;
  gap: 5px;
  transition: background-color 0.15s ease, color 0.15s ease, transform 0.1s ease;
}

.thinking-action-label {
  font-size: 12px;
  line-height: 1;
}

.thinking-icon {
  color: var(--primary);
  flex-shrink: 0;
}

.thinking-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--foreground);
  white-space: nowrap;
}

.thinking-duration {
  font-size: 12px;
  color: var(--muted-foreground);
  white-space: nowrap;
}

.thinking-chevron {
  color: var(--muted-foreground);
  transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  flex-shrink: 0;
}

.thinking-chevron-rotated {
  transform: rotate(-90deg);
}

@media (max-width: 480px) {
  .thinking-header {
    column-gap: 6px;
    padding: 9px 10px;
  }

  .thinking-header-actions { gap: 2px; }
  .thinking-expand-btn { width: 28px; }
  .thinking-action-label { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
}

/* Content area */
.thinking-content-wrapper {
  overflow: hidden;
  transition: height 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.thinking-content {
  padding: 12px 14px;
  border-top: 1px solid var(--border);
  background: var(--card);
  max-height: 400px;
  overflow-y: auto;
}

.thinking-pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-size: 12px;
  line-height: 1.6;
  color: var(--muted-foreground);
  font-family:
    'SF Mono',
    'Fira Code',
    'Cascadia Code',
    'Source Code Pro',
    Consolas,
    'Liberation Mono',
    Menlo,
    Courier,
    monospace;
}

/* Vue transition classes */
.thinking-expand-enter-active,
.thinking-expand-leave-active {
  transition: height 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden;
}

.thinking-expand-enter-from,
.thinking-expand-leave-to {
  height: 0px !important;
}
</style>

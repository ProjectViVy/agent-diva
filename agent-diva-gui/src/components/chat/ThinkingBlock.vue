<template>
  <div class="thinking-card">
    <!-- Header -->
    <div
      class="thinking-header"
      :class="{ 'thinking-header-expanded': isExpanded }"
      @click="toggleExpanded"
    >
      <div class="thinking-header-left">
        <Brain :size="16" class="thinking-icon" />
        <span class="thinking-label">{{ $t('chat.thinkingProcess') }}</span>
        <span v-if="thinkingMs && thinkingMs > 0" class="thinking-duration">
          ({{ formatDuration(thinkingMs) }})
        </span>
      </div>
      <div class="thinking-header-right">
        <button
          class="thinking-copy-btn"
          :class="{ 'thinking-copy-success': copied }"
          :title="copied ? $t('common.copied') : $t('common.copy')"
          @click.stop="handleCopy"
        >
          <CheckCircle2 v-if="copied" :size="14" />
          <Copy v-else :size="14" />
        </button>
        <ChevronDown
          :size="16"
          class="thinking-chevron"
          :class="{ 'thinking-chevron-rotated': !isExpanded }"
        />
      </div>
    </div>

    <!-- Content with transition -->
    <Transition
      name="thinking-expand"
      @enter="onEnter"
      @after-enter="onAfterEnter"
      @leave="onLeave"
    >
      <div v-show="isExpanded" class="thinking-content-wrapper">
        <div class="thinking-content">
          <pre class="thinking-pre">{{ content }}</pre>
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { Brain, ChevronDown, Copy, CheckCircle2 } from 'lucide-vue-next'

const props = defineProps<{
  content: string
  thinkingMs?: number
}>()

const isExpanded = ref(false)
const copied = ref(false)

function toggleExpanded() {
  isExpanded.value = !isExpanded.value
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

async function handleCopy() {
  try {
    await navigator.clipboard.writeText(props.content)
    copied.value = true
    setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch (err) {
    console.error('Failed to copy thinking content:', err)
  }
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
  border-radius: 12px;
  border: 1px solid var(--color-border, #e5e7eb);
  background: var(--color-bg-card, #ffffff);
  box-shadow:
    0 1px 3px rgba(0, 0, 0, 0.05),
    0 1px 2px rgba(0, 0, 0, 0.03);
  overflow: hidden;
  transition: box-shadow 0.2s ease;
}

.thinking-card:hover {
  box-shadow:
    0 4px 6px rgba(0, 0, 0, 0.05),
    0 2px 4px rgba(0, 0, 0, 0.03);
}

/* Dark mode support */
@media (prefers-color-scheme: dark) {
  .thinking-card {
    border-color: var(--color-border, #374151);
    background: var(--color-bg-card, #1f2937);
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.2),
      0 1px 2px rgba(0, 0, 0, 0.15);
  }

  .thinking-card:hover {
    box-shadow:
      0 4px 6px rgba(0, 0, 0, 0.25),
      0 2px 4px rgba(0, 0, 0, 0.2);
  }
}

.thinking-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  cursor: pointer;
  user-select: none;
  border-radius: 12px;
  transition: background-color 0.2s ease, border-radius 0.2s ease;
}

.thinking-header:hover {
  background: var(--color-bg-hover, #f3f4f6);
}

.thinking-header-expanded {
  border-radius: 12px 12px 0 0;
}

@media (prefers-color-scheme: dark) {
  .thinking-header:hover {
    background: var(--color-bg-hover, #374151);
  }
}

.thinking-header-left {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
}

.thinking-header-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.thinking-icon {
  color: var(--color-accent, #8b5cf6);
  flex-shrink: 0;
}

.thinking-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--color-text-primary, #374151);
  white-space: nowrap;
}

@media (prefers-color-scheme: dark) {
  .thinking-label {
    color: var(--color-text-primary, #d1d5db);
  }
}

.thinking-duration {
  font-size: 12px;
  color: var(--color-text-muted, #9ca3af);
  white-space: nowrap;
}

.thinking-copy-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--color-text-muted, #9ca3af);
  cursor: pointer;
  transition:
    background-color 0.15s ease,
    color 0.15s ease,
    transform 0.1s ease;
}

.thinking-copy-btn:hover {
  background: var(--color-bg-hover, #e5e7eb);
  color: var(--color-text-primary, #374151);
}

.thinking-copy-btn:active {
  transform: scale(0.92);
}

.thinking-copy-success {
  color: var(--color-success, #10b981);
}

@media (prefers-color-scheme: dark) {
  .thinking-copy-btn:hover {
    background: var(--color-bg-hover, #4b5563);
    color: var(--color-text-primary, #d1d5db);
  }
}

.thinking-chevron {
  color: var(--color-text-muted, #9ca3af);
  transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  flex-shrink: 0;
}

.thinking-chevron-rotated {
  transform: rotate(-90deg);
}

/* Content area */
.thinking-content-wrapper {
  overflow: hidden;
  transition: height 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.thinking-content {
  padding: 12px 14px;
  border-top: 1px solid var(--color-border, #e5e7eb);
  background: var(--color-bg-secondary, #f9fafb);
  max-height: 400px;
  overflow-y: auto;
}

@media (prefers-color-scheme: dark) {
  .thinking-content {
    border-top-color: var(--color-border, #374151);
    background: var(--color-bg-secondary, #111827);
  }
}

.thinking-pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-size: 12px;
  line-height: 1.6;
  color: var(--color-text-secondary, #4b5563);
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

@media (prefers-color-scheme: dark) {
  .thinking-pre {
    color: var(--color-text-secondary, #9ca3af);
  }
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

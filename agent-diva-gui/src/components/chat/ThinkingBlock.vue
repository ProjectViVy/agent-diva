<template>
  <div class="thinking-card">
    <!-- Header -->
    <div class="thinking-header" :class="{ 'thinking-header-expanded': isExpanded }">
      <button
        type="button"
        class="thinking-header-toggle"
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
          class="thinking-copy-btn"
          :class="{ 'thinking-copy-success': copied }"
          :title="copied ? $t('common.copied') : $t('common.copy')"
          :aria-label="copied ? $t('common.copied') : $t('common.copy')"
          @click.stop="handleCopy"
        >
          <CheckCircle2 v-if="copied" :size="14" />
          <Copy v-else :size="14" />
          <span class="thinking-action-label">{{ copied ? $t('common.copied') : $t('common.copy') }}</span>
        </button>
        <button
          type="button"
          class="thinking-expand-btn"
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
import { Brain, ChevronDown, Copy, CheckCircle2 } from 'lucide-vue-next'

const props = defineProps<{
  content: string
  thinkingMs?: number
}>()

const isExpanded = ref(false)
const copied = ref(false)
const contentId = computed(() => `thinking-content-${Math.random().toString(36).slice(2)}`)

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
  border-radius: var(--radius);
  border: 1px solid var(--line);
  background: var(--panel);
  box-shadow: var(--shadow);
  overflow: hidden;
  transition: box-shadow 0.2s ease;
}

.thinking-card:hover {
  box-shadow: 0 4px 16px var(--accent-glow);
}

.thinking-header {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  column-gap: 12px;
  padding: 10px 14px;
  border-radius: var(--radius);
  transition: background-color 0.2s ease, border-radius 0.2s ease;
}

.thinking-header:hover {
  background: var(--accent-bg-light);
}

.thinking-header-expanded {
  border-radius: var(--radius) var(--radius) 0 0;
}

.thinking-header-toggle {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  padding: 0;
  border: 0;
  color: inherit;
  background: transparent;
  cursor: pointer;
  text-align: left;
}

.thinking-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  white-space: nowrap;
}

.thinking-copy-btn,
.thinking-expand-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 28px;
  gap: 5px;
  padding: 0 7px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition: background-color 0.15s ease, color 0.15s ease, transform 0.1s ease;
}

.thinking-action-label {
  font-size: 12px;
  line-height: 1;
}

.thinking-icon {
  color: var(--brand);
  flex-shrink: 0;
}

.thinking-label {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
  white-space: nowrap;
}

.thinking-duration {
  font-size: 12px;
  color: var(--text-muted);
  white-space: nowrap;
}

.thinking-copy-btn {
  flex: 0 0 auto;
}

.thinking-copy-btn:hover,
.thinking-expand-btn:hover {
  background: var(--accent-bg-light);
  color: var(--text);
}

.thinking-copy-btn:active {
  transform: scale(0.92);
}

.thinking-copy-success {
  color: var(--success);
}

.thinking-chevron {
  color: var(--text-muted);
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
  .thinking-copy-btn, .thinking-expand-btn { width: 28px; padding: 0; }
  .thinking-action-label { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
}

/* Content area */
.thinking-content-wrapper {
  overflow: hidden;
  transition: height 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.thinking-content {
  padding: 12px 14px;
  border-top: 1px solid var(--line);
  background: var(--panel-solid);
  max-height: 400px;
  overflow-y: auto;
}

.thinking-pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-muted);
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

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import MarkdownIt from 'markdown-it';
import hljs from 'highlight.js';
import 'highlight.js/styles/github-dark.css';
import { AlertCircle } from 'lucide-vue-next';
import type { LaputaSectionName } from '../../api/desktop';

const { t } = useI18n();

interface Props {
  sectionName: LaputaSectionName;
  content?: string;
  lastUpdated?: string;
  status?: 'owned' | 'tbd' | string;
  loading?: boolean;
  error?: string;
}

const props = withDefaults(defineProps<Props>(), {
  content: '',
  lastUpdated: '',
  status: 'tbd',
  loading: false,
  error: '',
});

const emit = defineEmits<{
  (e: 'retry'): void;
}>();

const md = new MarkdownIt({
  html: false,
  linkify: true,
  highlight(str: string, lang: string) {
    if (lang && hljs.getLanguage(lang)) {
      try {
        return hljs.highlight(str, { language: lang }).value;
      } catch {
        // fall through
      }
    }
    return (
      '<pre><code>' +
      str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;') +
      '</code></pre>'
    );
  },
});

const renderedContent = computed(() => md.render(props.content ?? ''));

const normalizedStatus = computed(() => (props.status === 'owned' ? 'owned' : 'tbd'));

const isUninitialized = computed(() => {
  return (
    !props.loading &&
    !props.error &&
    props.content === '' &&
    props.lastUpdated === '' &&
    props.status === 'tbd'
  );
});

const isEmpty = computed(() => {
  return (
    !props.loading &&
    !props.error &&
    props.content === '' &&
    !isUninitialized.value
  );
});

function onRetry(): void {
  emit('retry');
}
</script>

<template>
  <div class="section-editor">
    <header class="section-editor-toolbar">
      <div class="section-editor-title-block">
        <h2 class="section-editor-title">
          {{ t('laputa.sections.' + sectionName) }}
        </h2>
        <span
          class="section-editor-status"
          :class="'section-editor-status--' + normalizedStatus"
        >
          {{ t('laputa.status.' + normalizedStatus) }}
        </span>
        <span v-if="lastUpdated" class="section-editor-updated">
          {{ lastUpdated }}
        </span>
      </div>

      <div class="section-editor-actions">
        <button
          type="button"
          class="section-editor-btn section-editor-btn--secondary"
          disabled
        >
          {{ t('laputa.history') }}
        </button>
        <button
          type="button"
          class="section-editor-btn section-editor-btn--primary"
          disabled
        >
          {{ t('laputa.save') }}
        </button>
      </div>
    </header>

    <div v-if="loading" class="section-editor-skeleton">
      <div class="skeleton-line title" />
      <div class="skeleton-line" />
      <div class="skeleton-line" />
      <div class="skeleton-line short" />
      <div class="skeleton-line medium" />
    </div>

    <div
      v-else-if="error"
      class="section-editor-error"
      role="status"
    >
      <AlertCircle :size="20" />
      <div class="section-editor-error-body">
        <strong>{{ t('laputa.loadError') }}</strong>
        <p>{{ error }}</p>
      </div>
      <button
        type="button"
        class="section-editor-btn section-editor-btn--secondary"
        @click="onRetry"
      >
        {{ t('laputa.retry') }}
      </button>
    </div>

    <div
      v-else-if="isUninitialized"
      class="section-editor-empty-state"
    >
      <strong>{{ t('laputa.uninitializedTitle') }}</strong>
      <p>{{ t('laputa.uninitializedDesc') }}</p>
    </div>

    <div
      v-else-if="isEmpty"
      class="section-editor-empty-state"
    >
      <strong>{{ t('laputa.emptyTitle') }}</strong>
      <p>{{ t('laputa.emptyDesc') }}</p>
    </div>

    <div
      v-else
      class="section-editor-content markdown-body"
      v-html="renderedContent"
    />
  </div>
</template>

<style scoped>
.section-editor {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--panel);
  overflow: hidden;
}

.section-editor-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--line);
  flex-shrink: 0;
  min-height: 56px;
}

.section-editor-title-block {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.section-editor-title {
  margin: 0;
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.section-editor-status {
  flex-shrink: 0;
  padding: 2px 8px;
  border-radius: 9999px;
  font-size: 0.625rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.section-editor-status--owned {
  background: var(--accent-bg-light);
  color: var(--accent);
  border: 1px solid var(--accent-border);
}

.section-editor-status--tbd {
  background: transparent;
  color: var(--text-muted);
  border: 1px solid var(--line);
}

.section-editor-updated {
  flex-shrink: 0;
  font-size: 0.75rem;
  font-weight: 400;
  color: var(--text-muted);
}

.section-editor-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.section-editor-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 6px 14px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.section-editor-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.section-editor-btn--primary {
  border: 1px solid var(--accent-border);
  background: var(--accent);
  color: #fff;
}

.section-editor-btn--secondary {
  border: 1px solid var(--line);
  background: var(--panel-solid);
  color: var(--text);
}

.section-editor-btn--secondary:hover:not(:disabled) {
  background: var(--accent-bg-light);
  border-color: var(--accent-border);
  color: var(--accent);
}

.section-editor-skeleton {
  flex: 1;
  padding: 24px;
  overflow-y: auto;
}

.skeleton-line {
  height: 12px;
  border-radius: 4px;
  background: var(--accent-bg-light);
  margin-bottom: 10px;
  animation: skeleton-pulse 1.5s ease-in-out infinite;
}

.skeleton-line.short { width: 40%; }
.skeleton-line.medium { width: 65%; }
.skeleton-line.long { width: 85%; }
.skeleton-line.title { width: 55%; height: 18px; margin-bottom: 16px; }

@keyframes skeleton-pulse {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 0.8; }
}

.section-editor-error {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  margin: 16px;
  padding: 12px 16px;
  border: 1px solid var(--danger);
  border-radius: var(--radius-sm);
  background: var(--danger-bg);
  color: var(--danger);
  font-size: 13px;
}

.section-editor-error svg {
  flex-shrink: 0;
  margin-top: 2px;
}

.section-editor-error-body {
  flex: 1;
  min-width: 0;
}

.section-editor-error-body strong {
  display: block;
  margin-bottom: 4px;
}

.section-editor-error-body p {
  margin: 0;
  overflow-wrap: anywhere;
}

.section-editor-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  flex: 1;
  padding: 24px;
  text-align: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: 14px;
}

.section-editor-empty-state strong {
  color: var(--text);
  font-weight: 500;
}

.section-editor-empty-state p {
  margin: 0;
  max-width: 360px;
}

.section-editor-content {
  flex: 1;
  padding: 16px 24px;
  overflow-y: auto;
  font-size: 0.875rem;
  line-height: 1.7;
  color: var(--text);
}

.section-editor-content :deep(p) {
  margin-bottom: 0.75em;
}

.section-editor-content :deep(p:last-child) {
  margin-bottom: 0;
}

.section-editor-content :deep(pre) {
  background-color: #1e1e1e;
  border-radius: 0.375rem;
  padding: 0.75rem;
  margin: 0.75rem 0;
  overflow-x: auto;
}

.section-editor-content :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 0.85em;
  background-color: rgba(0, 0, 0, 0.1);
  padding: 0.15em 0.35em;
  border-radius: 0.25rem;
}

.section-editor-content :deep(pre code) {
  background-color: transparent;
  padding: 0;
  color: #e5e7eb;
}

.section-editor-content :deep(ul),
.section-editor-content :deep(ol) {
  padding-left: 1.5em;
  margin-bottom: 0.75em;
}

.section-editor-content :deep(ul) {
  list-style-type: disc;
}

.section-editor-content :deep(ol) {
  list-style-type: decimal;
}

.section-editor-content :deep(blockquote) {
  border-left: 3px solid var(--line);
  padding-left: 0.75rem;
  color: var(--text-muted);
  margin: 0.75rem 0;
}

.section-editor-content :deep(h1),
.section-editor-content :deep(h2),
.section-editor-content :deep(h3) {
  color: var(--text);
  margin-top: 1.25em;
  margin-bottom: 0.5em;
}

.section-editor-content :deep(h1) { font-size: 1.5em; }
.section-editor-content :deep(h2) { font-size: 1.25em; }
.section-editor-content :deep(h3) { font-size: 1.1em; }

.section-editor-content :deep(a) {
  color: var(--accent);
  text-decoration: underline;
}

.section-editor-content :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 0.75rem 0;
}

.section-editor-content :deep(th),
.section-editor-content :deep(td) {
  border: 1px solid var(--line);
  padding: 6px 10px;
  text-align: left;
  font-size: 0.85em;
}

.section-editor-content :deep(th) {
  background: var(--accent-bg-light);
  font-weight: 600;
}
</style>

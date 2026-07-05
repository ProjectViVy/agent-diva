<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { Loader2 } from 'lucide-vue-next';
import MarkdownIt from 'markdown-it';
import hljs from 'highlight.js';
import 'highlight.js/styles/github-dark.css';
import { writeLaputaSection } from '../../api/desktop';
import { appConfirm } from '../../utils/appDialog';
import type { LaputaSectionName } from '../../api/desktop';

const { t } = useI18n();

const props = defineProps<{
  sectionName: LaputaSectionName;
  displayName: string;
  initialContent: string;
}>();

const emit = defineEmits<{
  (e: 'saved', sectionName: LaputaSectionName): void;
}>();

const md = new MarkdownIt({
  html: false,
  linkify: true,
  highlight(str: string, lang: string) {
    if (lang && hljs.getLanguage(lang)) {
      try {
        return hljs.highlight(str, { language: lang, ignoreIllegals: true }).value;
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

const originalContent = ref<string>(props.initialContent ?? '');
const draftContent = ref<string>(props.initialContent ?? '');
const saving = ref(false);
const saveError = ref<string | null>(null);
const activeTab = ref<'edit' | 'preview'>('edit');

const isDirty = computed(() => draftContent.value !== originalContent.value);

watch(
  () => props.initialContent,
  (next) => {
    originalContent.value = next ?? '';
    draftContent.value = next ?? '';
    saveError.value = null;
  },
);

const renderedHtml = computed(() => md.render(draftContent.value));

function formatError(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (
    err !== null &&
    typeof err === 'object' &&
    'message' in err &&
    typeof (err as { message: unknown }).message === 'string'
  ) {
    return (err as { message: string }).message;
  }
  return String(err);
}

async function handleSave(): Promise<void> {
  if (!isDirty.value || saving.value) return;

  if (originalContent.value.trim().length > 0) {
    const confirmed = await appConfirm(
      t('laputa.confirmSave.message', { section: props.displayName }),
      {
        title: t('laputa.confirmSave.title'),
        confirmLabel: t('laputa.confirmSave.confirm'),
        cancelLabel: t('laputa.confirmSave.cancel'),
      },
    );
    if (!confirmed) return;
  }

  saving.value = true;
  saveError.value = null;
  try {
    await writeLaputaSection(props.sectionName, draftContent.value);
    originalContent.value = draftContent.value;
    emit('saved', props.sectionName);
  } catch (err: unknown) {
    saveError.value = formatError(err);
  } finally {
    saving.value = false;
  }
}

function onKeyDown(event: KeyboardEvent): void {
  if ((event.ctrlKey || event.metaKey) && event.key === 's') {
    event.preventDefault();
    void handleSave();
  }
}
</script>

<template>
  <div class="section-editor">
    <header class="section-editor-toolbar">
      <div class="section-editor-toolbar-title">
        <span>{{ t('laputa.sections.' + sectionName) }}</span>
      </div>
      <div class="section-editor-toolbar-actions">
        <button
          type="button"
          class="section-editor-save-btn"
          :disabled="!isDirty || saving"
          @click="handleSave"
        >
          <Loader2 v-if="saving" :size="14" class="spin" />
          <span>{{ saving ? t('laputa.saving') : t('laputa.save') }}</span>
        </button>
      </div>
    </header>

    <div v-if="saveError" class="section-editor-save-error" role="alert">
      <span>{{ t('laputa.saveFailed', { message: saveError }) }}</span>
    </div>

    <div class="section-editor-tabs" role="tablist" aria-label="Editor view">
      <button
        type="button"
        role="tab"
        class="section-editor-tab"
        :class="{ active: activeTab === 'edit' }"
        :aria-selected="activeTab === 'edit'"
        aria-controls="section-editor-edit-pane"
        @click="activeTab = 'edit'"
      >
        {{ t('laputa.edit') }}
      </button>
      <button
        type="button"
        role="tab"
        class="section-editor-tab"
        :class="{ active: activeTab === 'preview' }"
        :aria-selected="activeTab === 'preview'"
        aria-controls="section-editor-preview-pane"
        @click="activeTab = 'preview'"
      >
        {{ t('laputa.preview') }}
      </button>
    </div>

    <div class="section-editor-panes">
      <div
        id="section-editor-edit-pane"
        class="section-editor-pane"
        role="tabpanel"
        :class="{ 'pane-hidden': activeTab !== 'edit' }"
      >
        <textarea
          v-model="draftContent"
          class="section-editor-textarea"
          aria-label="Laputa section editor"
          @keydown="onKeyDown"
        />
      </div>

      <div
        id="section-editor-preview-pane"
        class="section-editor-pane"
        role="tabpanel"
        :class="{ 'pane-hidden': activeTab !== 'preview' }"
      >
        <div class="section-editor-preview markdown-body" v-html="renderedHtml" />
      </div>
    </div>
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

.section-editor-toolbar-title {
  display: flex;
  align-items: center;
  min-width: 0;
}

.section-editor-toolbar-title span {
  margin: 0;
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.section-editor-toolbar-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.section-editor-save-btn {
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
  border: 1px solid var(--accent-border);
  background: var(--accent);
  color: #fff;
}

.section-editor-save-btn:hover:not(:disabled) {
  filter: brightness(1.08);
}

.section-editor-save-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.section-editor-save-error {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 12px 16px 0;
  padding: 10px 14px;
  border: 1px solid var(--danger);
  border-radius: var(--radius-sm);
  background: var(--danger-bg);
  color: var(--danger);
  font-size: 13px;
}

.section-editor-tabs {
  display: flex;
  gap: 4px;
  padding: 12px 16px 0;
  flex-shrink: 0;
}

.section-editor-tab {
  padding: 6px 14px;
  border: 1px solid var(--line);
  border-bottom: none;
  border-radius: var(--radius-sm) var(--radius-sm) 0 0;
  background: var(--panel-solid);
  color: var(--text-muted);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.section-editor-tab:hover {
  color: var(--text);
  background: var(--accent-bg-light);
}

.section-editor-tab.active {
  background: var(--panel);
  color: var(--accent);
  border-color: var(--accent-border);
}

.section-editor-panes {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  flex: 1;
  min-height: 0;
  padding: 0 16px 16px;
  overflow: hidden;
}

.section-editor-pane {
  display: flex;
  flex-direction: column;
  flex: 1 1 0;
  min-width: 0;
  min-height: 0;
}

.section-editor-pane.pane-hidden {
  display: none;
}

.section-editor-textarea {
  width: 100%;
  min-height: 320px;
  padding: 1rem;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
  color: var(--text);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.875rem;
  line-height: 1.6;
  resize: vertical;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.section-editor-textarea:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-glow);
}

.section-editor-preview {
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  padding: 1rem;
  overflow-y: auto;
  min-height: 320px;
  font-size: 0.875rem;
  line-height: 1.7;
  color: var(--text);
}

.section-editor-preview :deep(p) {
  margin-bottom: 0.75em;
}

.section-editor-preview :deep(p:last-child) {
  margin-bottom: 0;
}

.section-editor-preview :deep(pre) {
  background: var(--panel-solid);
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  padding: 0.75rem;
  margin: 0.75rem 0;
  overflow-x: auto;
}

.section-editor-preview :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', 'Courier New', monospace;
  font-size: 0.85em;
  background: var(--accent-bg-light);
  padding: 0.15em 0.35em;
  border-radius: 0.25rem;
  color: var(--text);
}

.section-editor-preview :deep(pre code) {
  background: transparent;
  padding: 0;
  border-radius: 0;
}

.section-editor-preview :deep(ul),
.section-editor-preview :deep(ol) {
  padding-left: 1.5em;
  margin-bottom: 0.75em;
}

.section-editor-preview :deep(ul) {
  list-style-type: disc;
}

.section-editor-preview :deep(ol) {
  list-style-type: decimal;
}

.section-editor-preview :deep(blockquote) {
  border-left: 3px solid var(--line);
  padding-left: 0.75rem;
  color: var(--text-muted);
  margin: 0.75rem 0;
}

.section-editor-preview :deep(h1),
.section-editor-preview :deep(h2),
.section-editor-preview :deep(h3) {
  color: var(--text);
  margin-top: 1.25em;
  margin-bottom: 0.5em;
}

.section-editor-preview :deep(h1) {
  font-size: 1.5em;
}

.section-editor-preview :deep(h2) {
  font-size: 1.25em;
}

.section-editor-preview :deep(h3) {
  font-size: 1.1em;
}

.section-editor-preview :deep(a) {
  color: var(--accent);
  text-decoration: underline;
}

.section-editor-preview :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 0.75rem 0;
}

.section-editor-preview :deep(th),
.section-editor-preview :deep(td) {
  border: 1px solid var(--line);
  padding: 6px 10px;
  text-align: left;
  font-size: 0.85em;
}

.section-editor-preview :deep(th) {
  background: var(--accent-bg-light);
  font-weight: 600;
}

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@media (min-width: 1024px) {
  .section-editor-tabs {
    display: none;
  }

  .section-editor-panes {
    flex-direction: row;
  }

  .section-editor-pane.pane-hidden {
    display: flex;
  }
}
</style>

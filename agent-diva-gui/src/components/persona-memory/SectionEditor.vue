<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { Loader2 } from '@lucide/vue';
import { writeLaputaSection } from '../../api/desktop';
import { appConfirm } from '../../utils/appDialog';
import HistoryModal from './HistoryModal.vue';
import type { LaputaSectionName, WriteLaputaSectionResult } from '../../api/desktop';

const { t } = useI18n();

const props = withDefaults(
  defineProps<{
    modelValue: string;
    displayName: string;
    sectionName: LaputaSectionName;
    initialContent: string;
    status?: 'owned' | 'tbd';
    lastUpdated?: string;
    pendingProposal?: boolean;
  }>(),
  {
    modelValue: '',
    status: 'tbd',
    lastUpdated: undefined,
    pendingProposal: false,
  },
);

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void;
  (e: 'update:dirty', isDirty: boolean): void;
  (e: 'proposal-created', sectionName: LaputaSectionName, result: WriteLaputaSectionResult): void;
  (e: 'save-failed', sectionName: LaputaSectionName, message: string): void;
}>();

const originalContent = ref<string>(props.initialContent ?? '');
const draftContent = ref<string>(props.modelValue ?? '');
const saving = ref(false);
const saveError = ref<string | null>(null);
const changeReason = ref('');
const activeTab = ref<'edit' | 'preview'>('edit');
const historyOpen = ref(false);
const historyTriggerRef = ref<HTMLButtonElement | null>(null);

const isDirty = computed(() => draftContent.value !== originalContent.value);

watch(isDirty, (next) => {
  emit('update:dirty', next);
}, { immediate: true });

watch(
  () => props.modelValue,
  (next) => {
    draftContent.value = next ?? '';
  },
);

watch(
  () => props.initialContent,
  (next) => {
    originalContent.value = next ?? '';
    draftContent.value = next ?? '';
    saveError.value = null;
  },
);

function formatDate(value?: string): string {
  if (!value) return '';
  try {
    return new Date(value).toLocaleString();
  } catch {
    return value;
  }
}

const jsonError = computed(() => {
  try {
    JSON.parse(draftContent.value);
    return '';
  } catch (error) {
    return formatError(error);
  }
});

const formattedJson = computed(() => {
  if (jsonError.value) return draftContent.value;
  return JSON.stringify(JSON.parse(draftContent.value), null, 2);
});

function formatJson(): void {
  if (jsonError.value) return;
  draftContent.value = formattedJson.value;
  onInput();
}

function onInput(): void {
  emit('update:modelValue', draftContent.value);
}

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
  if (!isDirty.value || saving.value || jsonError.value || !changeReason.value.trim()) return;

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
    const result = await writeLaputaSection(
      props.sectionName,
      draftContent.value,
      changeReason.value.trim(),
    );
    draftContent.value = originalContent.value;
    emit('update:modelValue', originalContent.value);
    changeReason.value = '';
    emit('proposal-created', props.sectionName, result);
  } catch (err: unknown) {
    const message = formatError(err);
    saveError.value = message;
    emit('save-failed', props.sectionName, message);
  } finally {
    saving.value = false;
  }
}

function onHistoryClose() {
  historyOpen.value = false;
  nextTick(() => historyTriggerRef.value?.focus());
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
        <span
          v-if="status"
          class="section-editor-status-badge"
          :class="'section-editor-status-badge--' + status"
        >
          {{ t('laputa.status.' + status) }}
        </span>
        <span
          v-if="lastUpdated"
          class="section-editor-last-updated"
        >
          {{ t('laputa.lastUpdated', { time: formatDate(lastUpdated) }) }}
        </span>
      </div>
      <div class="section-editor-toolbar-actions">
        <slot name="toolbar-actions">
          <button
            type="button"
            class="section-editor-history-btn"
            :disabled="Boolean(jsonError)"
            @click="formatJson"
          >
            {{ t('laputa.formatJson') }}
          </button>
          <button
            ref="historyTriggerRef"
            type="button"
            class="section-editor-history-btn"
            :disabled="!props.sectionName"
            :aria-label="t('laputa.a11y.historyButton', { section: props.displayName })"
            @click="historyOpen = true"
          >
            {{ t('laputa.history') }}
          </button>
          <button
            type="button"
            class="section-editor-save-btn"
            :disabled="!isDirty || saving || Boolean(jsonError) || !changeReason.trim()"
            :aria-label="t('laputa.a11y.saveButton', { section: props.displayName })"
            @click="handleSave"
          >
            <Loader2 v-if="saving" :size="14" class="spin" />
            <span>{{ saving ? t('laputa.submitting') : t('laputa.submitProposal') }}</span>
          </button>
        </slot>
      </div>
    </header>

    <HistoryModal
      :open="historyOpen"
      :section-name="props.sectionName"
      @close="onHistoryClose"
    />

    <div v-if="saveError" class="section-editor-save-error" role="alert">
      <span>{{ t('laputa.saveFailed', { message: saveError }) }}</span>
    </div>

    <p v-if="pendingProposal" class="proposal-pending-note" role="status">
      {{ t('laputa.workspace.proposalPending') }}
    </p>

    <div class="section-editor-reason">
      <label :for="`laputa-reason-${sectionName}`">{{ t('laputa.changeReason') }}</label>
      <input
        :id="`laputa-reason-${sectionName}`"
        v-model="changeReason"
        type="text"
        :placeholder="t('laputa.changeReasonPlaceholder')"
      />
      <span v-if="jsonError" class="section-editor-json-error" role="alert">
        {{ t('laputa.invalidJson', { message: jsonError }) }}
      </span>
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
          :aria-label="t('laputa.a11y.editor', { section: props.displayName })"
          @input="onInput"
          @keydown="onKeyDown"
        />
      </div>

      <div
        id="section-editor-preview-pane"
        class="section-editor-pane"
        role="tabpanel"
        :class="{ 'pane-hidden': activeTab !== 'preview' }"
      >
        <pre class="section-editor-preview">{{ formattedJson }}</pre>
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
  gap: 10px;
  min-width: 0;
  flex: 1;
}

.section-editor-toolbar-title > span:first-child {
  margin: 0;
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.section-editor-status-badge {
  flex-shrink: 0;
  padding: 2px 8px;
  border-radius: 9999px;
  font-size: 0.625rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.section-editor-status-badge--owned {
  background: var(--accent-bg-light);
  color: var(--accent);
  border: 1px solid var(--accent-border);
}

.section-editor-status-badge--tbd {
  background: transparent;
  color: var(--text-muted);
  border: 1px solid var(--line);
}

.section-editor-last-updated {
  flex-shrink: 0;
  font-size: 0.75rem;
  font-weight: 400;
  color: var(--text-muted);
  white-space: nowrap;
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

.section-editor-save-btn:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--accent-glow), 0 0 0 4px var(--accent);
}

.section-editor-history-btn {
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
  border: 1px solid var(--line);
  background: var(--panel);
  color: var(--text);
}

.section-editor-history-btn:hover:not(:disabled) {
  background: var(--accent-bg-light);
  border-color: var(--accent-border);
}

.section-editor-history-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.section-editor-history-btn:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--accent-glow), 0 0 0 4px var(--accent);
}

.section-editor-tab:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--accent-glow), 0 0 0 4px var(--accent);
}

.section-editor-textarea:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-glow);
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

.proposal-pending-note {
  margin: 12px 16px 0;
  padding: 8px 10px;
  border: 1px solid var(--warning);
  border-radius: var(--radius-sm);
  color: var(--warning);
  font-size: 11px;
  line-height: 1.45;
}

.section-editor-reason {
  display: grid;
  grid-template-columns: auto minmax(180px, 1fr) auto;
  align-items: center;
  gap: 10px;
  padding: 12px 16px 0;
  color: var(--text-muted);
  font-size: 12px;
}

.section-editor-reason input {
  min-width: 0;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
  color: var(--text);
}

.section-editor-json-error { color: var(--danger); }

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
  white-space: pre-wrap;
  word-break: break-word;
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

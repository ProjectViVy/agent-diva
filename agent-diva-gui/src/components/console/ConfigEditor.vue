<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { FileJson, RefreshCcw, Save, Check, AlertCircle } from 'lucide-vue-next';

const { t } = useI18n();

const props = defineProps<{
  configText: string;
  loading?: boolean;
  saving?: boolean;
  error?: string;
  savedAt?: number | null;
}>();

const emit = defineEmits<{
  (e: 'update:configText', value: string): void;
  (e: 'reload'): void;
  (e: 'save'): void;
}>();

const localConfigText = ref(props.configText);
const validationError = ref<string | null>(null);

// Sync props to local state
watch(
  () => props.configText,
  (newVal) => {
    localConfigText.value = newVal;
  }
);

// Validate JSON on input
watch(localConfigText, (newVal) => {
  emit('update:configText', newVal);
  if (!newVal.trim()) {
    validationError.value = null;
    return;
  }
  try {
    JSON.parse(newVal);
    validationError.value = null;
  } catch (e) {
    validationError.value = (e as SyntaxError).message;
  }
});

const isValidJson = computed(() => {
  if (!localConfigText.value.trim()) return true;
  return validationError.value === null;
});

const lastSavedLabel = computed(() => {
  if (!props.savedAt) return '';
  return new Date(props.savedAt).toLocaleString();
});

function updateConfigText(event: Event) {
  const target = event.target as HTMLTextAreaElement;
  localConfigText.value = target.value;
}

function reload() {
  emit('reload');
}

function save() {
  if (!isValidJson.value) return;
  emit('save');
}
</script>

<template>
  <div class="config-editor space-y-3">
    <!-- Header -->
    <div class="flex items-center justify-between gap-4">
      <div class="flex items-center gap-3">
        <div class="editor-icon">
          <FileJson :size="20" />
        </div>
        <div>
          <h3 class="editor-title">{{ t('console.configTitle') }}</h3>
          <p class="editor-desc">{{ t('console.configDesc') }}</p>
        </div>
      </div>

      <div class="flex gap-2">
        <button
          type="button"
          class="action-btn action-btn--ghost"
          :disabled="loading"
          @click="reload"
        >
          <RefreshCcw :size="14" :class="loading ? 'animate-spin' : ''" />
          {{ t('console.reloadConfig') }}
        </button>
        <button
          type="button"
          class="action-btn action-btn--primary"
          :class="{ 'opacity-50 cursor-not-allowed': !isValidJson }"
          :disabled="saving || !isValidJson"
          @click="save"
        >
          <Save :size="14" />
          {{ saving ? t('console.saving') : t('console.saveConfig') }}
        </button>
      </div>
    </div>

    <!-- Validation status -->
    <div class="flex items-center justify-between text-xs">
      <div class="flex items-center gap-2">
        <template v-if="isValidJson && localConfigText.trim()">
          <Check :size="12" class="text-emerald-500" />
          <span class="text-emerald-600">Valid JSON</span>
        </template>
        <template v-else-if="validationError">
          <AlertCircle :size="12" class="text-rose-500" />
          <span class="text-rose-600">{{ validationError }}</span>
        </template>
        <span v-else class="editor-hint">{{ t('console.jsonEditorHint') }}</span>
      </div>
      <span v-if="lastSavedLabel" class="editor-saved-time">
        {{ t('console.savedAt') }}: {{ lastSavedLabel }}
      </span>
    </div>

    <!-- Editor textarea -->
    <div class="relative">
      <textarea
        :value="localConfigText"
        class="config-textarea"
        :class="{ 'config-textarea--error': validationError }"
        spellcheck="false"
        @input="updateConfigText"
      />
      <!-- Saving overlay -->
      <div
        v-if="saving"
        class="saving-overlay"
      >
        <RefreshCcw :size="20" class="animate-spin text-emerald-600" />
      </div>
    </div>

    <!-- Error message -->
    <p v-if="error" class="editor-error">
      {{ error }}
    </p>
  </div>
</template>

<style scoped>
.config-editor {
  @apply flex flex-col;
}

.editor-icon {
  width: 2.5rem;
  height: 2.5rem;
  border-radius: var(--radius-sm);
  background: var(--accent-bg-light);
  color: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
}

.editor-title {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--text);
}

.editor-desc {
  font-size: 0.875rem;
  color: var(--text-muted);
}

.editor-hint {
  color: var(--text-muted);
  opacity: 0.7;
}

.editor-saved-time {
  color: var(--text-muted);
}

.editor-error {
  font-size: 0.75rem;
  color: var(--danger);
  word-break: break-all;
}

.action-btn {
  @apply inline-flex items-center gap-2 px-3 py-2 text-xs rounded-lg transition-all;
}

.action-btn--primary {
  background: var(--accent);
  color: white;
}

.action-btn--primary:hover:not(:disabled) {
  filter: brightness(1.1);
}

.action-btn--ghost {
  background: transparent;
  border: 1px solid var(--line);
  color: var(--text-muted);
}

.action-btn--ghost:hover:not(:disabled) {
  background: var(--accent-bg-light);
  color: var(--text);
}

.action-btn:disabled {
  opacity: 0.6;
}

.config-textarea {
  width: 100%;
  min-height: 240px;
  border-radius: var(--radius);
  border: 1px solid var(--line);
  background: var(--accent-bg-light);
  padding: 0.75rem 1rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.75rem;
  color: var(--text);
  resize: vertical;
  line-height: 1.5;
  outline: none;
}

.config-textarea:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-glow);
}

.config-textarea--error {
  border-color: var(--danger);
}

.config-textarea--error:focus {
  box-shadow: 0 0 0 2px var(--danger-bg);
}

.saving-overlay {
  position: absolute;
  inset: 0;
  background: rgba(255, 255, 255, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius);
}

.theme-dark .saving-overlay {
  background: rgba(15, 23, 42, 0.5);
}
</style>
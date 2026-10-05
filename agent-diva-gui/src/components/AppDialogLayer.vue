<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  getAppDialogOpen,
  appDialogConfirmError,
  appDialogConfirmPending,
  dismissAppDialogConfirm,
  dismissAppDialogAlert,
  dismissAppDialogPrompt,
  runAppDialogConfirm,
} from '../utils/appDialog';

const props = withDefaults(
  defineProps<{
    /** Matches `NormalMode` app-shell theme (`love` | `dark` | …). */
    themeMode?: string;
  }>(),
  { themeMode: 'love' },
);

const { t } = useI18n();
const dialogOpen = getAppDialogOpen();

const open = computed(() => dialogOpen.value);

const promptValue = ref('');

watch(open, () => {
  promptValue.value = '';
});

const shellTheme = computed(() => `theme-${props.themeMode || 'love'}`);

function onBackdropClick() {
  const d = open.value;
  if (!d || (d.kind === 'confirm' && appDialogConfirmPending.value)) return;
  if (d.kind === 'confirm') {
    dismissAppDialogConfirm(false);
  } else if (d.kind === 'prompt') {
    dismissAppDialogPrompt(null);
  }
}

function onEscape(e: KeyboardEvent) {
  if (e.key !== 'Escape' || !open.value) return;
  if (open.value.kind === 'confirm' && appDialogConfirmPending.value) return;
  if (open.value.kind === 'confirm') {
    dismissAppDialogConfirm(false);
  } else if (open.value.kind === 'prompt') {
    dismissAppDialogPrompt(null);
  }
}

onMounted(() => {
  window.addEventListener('keydown', onEscape);
});

onUnmounted(() => {
  window.removeEventListener('keydown', onEscape);
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="app-dialog-backdrop fixed inset-0 z-[600] flex items-center justify-center p-4 backdrop-blur-[2px]"
      role="dialog"
      aria-modal="true"
      :aria-label="open.title || t('appDialog.defaultTitle')"
      @click.self="onBackdropClick"
    >
      <div class="w-full max-w-md" :class="shellTheme" @click.stop>
        <div
          class="app-dialog-panel rounded-2xl shadow-2xl border overflow-hidden flex flex-col"
        >
        <div class="app-dialog-content px-5 pt-4 pb-3">
          <h2
            v-if="open.title"
            class="app-dialog-title text-base font-semibold"
          >
            {{ open.title }}
          </h2>
          <p
            class="app-dialog-message text-sm leading-relaxed whitespace-pre-wrap"
            :class="open.title ? 'mt-2' : ''"
          >
            {{ open.message }}
          </p>
          <p
            v-if="open.kind === 'confirm' && appDialogConfirmError"
            class="app-dialog-error mt-3 rounded-lg border px-3 py-2 text-sm"
            role="alert"
          >
            {{ appDialogConfirmError }}
          </p>
          <textarea
            v-if="open.kind === 'prompt'"
            v-model="promptValue"
            class="app-dialog-input mt-3 w-full rounded-lg border px-3 py-2 text-sm outline-none transition resize-none"
            rows="3"
            :placeholder="open.placeholder || ''"
            @keydown.enter.prevent="dismissAppDialogPrompt(promptValue.trim() || null)"
          ></textarea>
        </div>

        <div
          class="app-dialog-footer px-5 py-3 border-t flex flex-wrap items-center justify-end gap-2 shrink-0"
        >
          <template v-if="open.kind === 'confirm'">
            <button
              type="button"
              class="app-dialog-action app-dialog-cancel"
              :disabled="appDialogConfirmPending"
              @click="dismissAppDialogConfirm(false)"
            >
              {{ open.cancelLabel || t('appDialog.cancel') }}
            </button>
            <button
              type="button"
              class="app-dialog-action app-dialog-confirm"
              :disabled="appDialogConfirmPending"
              :aria-busy="appDialogConfirmPending"
              @click="runAppDialogConfirm"
            >
              {{ appDialogConfirmPending ? t('appDialog.working') : open.confirmLabel || t('appDialog.confirm') }}
            </button>
          </template>
          <template v-else-if="open.kind === 'prompt'">
            <button
              type="button"
              class="app-dialog-action app-dialog-cancel"
              @click="dismissAppDialogPrompt(null)"
            >
              {{ open.cancelLabel || t('appDialog.cancel') }}
            </button>
            <button
              type="button"
              class="app-dialog-action app-dialog-confirm"
              :disabled="!promptValue.trim()"
              @click="dismissAppDialogPrompt(promptValue.trim())"
            >
              {{ open.confirmLabel || t('appDialog.confirm') }}
            </button>
          </template>
          <template v-else>
            <button
              type="button"
              class="app-dialog-action app-dialog-confirm"
              @click="dismissAppDialogAlert()"
            >
              {{ open.okLabel || t('appDialog.ok') }}
            </button>
          </template>
        </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.app-dialog-backdrop {
  background: var(--overlay, rgb(15 23 42 / 0.45));
}

.app-dialog-panel {
  color: var(--text, #111827);
  background: var(--panel-solid, #fff);
  border-color: var(--line, #e5e7eb);
}

.app-dialog-title { color: var(--text, #111827); }
.app-dialog-message { color: var(--text-muted, #4b5563); }

.app-dialog-footer {
  background: var(--surface-sunken, var(--panel));
  border-color: var(--line, #e5e7eb);
}

.app-dialog-error {
  color: var(--danger, #b91c1c);
  background: var(--danger-bg, #fef2f2);
  border-color: var(--danger, #ef4444);
}

.app-dialog-input {
  color: var(--text, #111827);
  background: var(--panel-solid, #fff);
  border-color: var(--line, #e5e7eb);
}

.app-dialog-input:focus-visible {
  outline: 2px solid var(--brand, #ec4899);
  outline-offset: 2px;
}

.app-dialog-action {
  min-height: 36px;
  padding: 0 16px;
  border-radius: 10px;
  font-size: 12px;
  font-weight: 600;
  transition: background-color 120ms ease, border-color 120ms ease, color 120ms ease;
}

.app-dialog-action:focus-visible {
  outline: 2px solid var(--brand, #ec4899);
  outline-offset: 2px;
}

.app-dialog-action:disabled {
  cursor: not-allowed;
  opacity: 0.58;
}

.app-dialog-cancel {
  color: var(--text, #374151);
  background: var(--surface-raised, #fff);
  border: 1px solid var(--line, #e5e7eb);
}

.app-dialog-cancel:hover:not(:disabled) {
  background: var(--nav-hover, var(--surface-raised));
}

.app-dialog-confirm {
  color: var(--brand-foreground, #fff);
  background: var(--brand, #ec4899);
  border: 1px solid transparent;
}

.app-dialog-confirm:hover:not(:disabled) {
  filter: brightness(0.94);
}
</style>

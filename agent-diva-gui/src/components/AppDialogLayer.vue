<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  getAppDialogOpen,
  dismissAppDialogConfirm,
  dismissAppDialogAlert,
  dismissAppDialogPrompt,
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
  if (!d) return;
  if (d.kind === 'confirm') {
    dismissAppDialogConfirm(false);
  } else if (d.kind === 'prompt') {
    dismissAppDialogPrompt(null);
  }
}

function onEscape(e: KeyboardEvent) {
  if (e.key !== 'Escape' || !open.value) return;
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
      class="fixed inset-0 z-[600] flex items-center justify-center bg-overlay p-4 "
      role="dialog"
      aria-modal="true"
      :aria-label="open.title || t('appDialog.defaultTitle')"
      @click.self="onBackdropClick"
    >
      <div class="w-full max-w-md" :class="shellTheme" @click.stop>
        <div
          class="rounded-2xl shadow-2xl border border-border overflow-hidden flex flex-col bg-card"
        >
        <div class="px-5 pt-4 pb-3">
          <h2
            v-if="open.title"
            class="text-base font-semibold text-foreground"
          >
            {{ open.title }}
          </h2>
          <p
            class="text-sm text-foreground leading-relaxed whitespace-pre-wrap"
            :class="open.title ? 'mt-2' : ''"
          >
            {{ open.message }}
          </p>
          <textarea
            v-if="open.kind === 'prompt'"
            v-model="promptValue"
            class="ui-input mt-3 w-full resize-none"
            rows="3"
            :placeholder="open.placeholder || ''"
            @keydown.enter.prevent="dismissAppDialogPrompt(promptValue.trim() || null)"
          ></textarea>
        </div>

        <div
          class="px-5 py-3 border-t border-border flex flex-wrap items-center justify-end gap-2 bg-muted shrink-0"
        >
          <template v-if="open.kind === 'confirm'">
            <button
              type="button"
              class="ui-button ui-button--outline"
              @click="dismissAppDialogConfirm(false)"
            >
              {{ open.cancelLabel || t('appDialog.cancel') }}
            </button>
            <button
              type="button"
              class="ui-button ui-button--primary transition "
              @click="dismissAppDialogConfirm(true)"
            >
              {{ open.confirmLabel || t('appDialog.confirm') }}
            </button>
          </template>
          <template v-else-if="open.kind === 'prompt'">
            <button
              type="button"
              class="ui-button ui-button--outline"
              @click="dismissAppDialogPrompt(null)"
            >
              {{ open.cancelLabel || t('appDialog.cancel') }}
            </button>
            <button
              type="button"
              class="ui-button ui-button--primary transition  disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="!promptValue.trim()"
              @click="dismissAppDialogPrompt(promptValue.trim())"
            >
              {{ open.confirmLabel || t('appDialog.confirm') }}
            </button>
          </template>
          <template v-else>
            <button
              type="button"
              class="ui-button ui-button--primary transition "
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

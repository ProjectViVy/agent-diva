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
      class="fixed inset-0 z-[600] flex items-center justify-center bg-black/45 p-4 backdrop-blur-[2px]"
      role="dialog"
      aria-modal="true"
      :aria-label="open.title || t('appDialog.defaultTitle')"
      @click.self="onBackdropClick"
    >
      <div class="w-full max-w-md" :class="shellTheme" @click.stop>
        <div
          class="rounded-2xl shadow-2xl border border-gray-100 overflow-hidden flex flex-col bg-white"
        >
        <div class="px-5 pt-4 pb-3">
          <h2
            v-if="open.title"
            class="text-base font-semibold text-gray-900"
          >
            {{ open.title }}
          </h2>
          <p
            class="text-sm text-gray-700 leading-relaxed whitespace-pre-wrap"
            :class="open.title ? 'mt-2' : ''"
          >
            {{ open.message }}
          </p>
          <textarea
            v-if="open.kind === 'prompt'"
            v-model="promptValue"
            class="mt-3 w-full rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm text-gray-800 outline-none transition focus:border-pink-300 focus:ring-2 focus:ring-pink-100 resize-none"
            rows="3"
            :placeholder="open.placeholder || ''"
            @keydown.enter.prevent="dismissAppDialogPrompt(promptValue.trim() || null)"
          ></textarea>
        </div>

        <div
          class="px-5 py-3 border-t border-gray-100 flex flex-wrap items-center justify-end gap-2 bg-gray-50/80 shrink-0"
        >
          <template v-if="open.kind === 'confirm'">
            <button
              type="button"
              class="px-4 py-2 rounded-lg border border-gray-200 text-xs font-medium text-gray-700 transition hover:bg-white hover:border-pink-200 hover:text-pink-800"
              @click="dismissAppDialogConfirm(false)"
            >
              {{ open.cancelLabel || t('appDialog.cancel') }}
            </button>
            <button
              type="button"
              class="px-4 py-2 rounded-lg bg-pink-500 text-white text-xs font-semibold shadow-sm shadow-pink-500/25 transition hover:bg-pink-600"
              @click="dismissAppDialogConfirm(true)"
            >
              {{ open.confirmLabel || t('appDialog.confirm') }}
            </button>
          </template>
          <template v-else-if="open.kind === 'prompt'">
            <button
              type="button"
              class="px-4 py-2 rounded-lg border border-gray-200 text-xs font-medium text-gray-700 transition hover:bg-white hover:border-pink-200 hover:text-pink-800"
              @click="dismissAppDialogPrompt(null)"
            >
              {{ open.cancelLabel || t('appDialog.cancel') }}
            </button>
            <button
              type="button"
              class="px-4 py-2 rounded-lg bg-pink-500 text-white text-xs font-semibold shadow-sm shadow-pink-500/25 transition hover:bg-pink-600 disabled:opacity-50 disabled:cursor-not-allowed"
              :disabled="!promptValue.trim()"
              @click="dismissAppDialogPrompt(promptValue.trim())"
            >
              {{ open.confirmLabel || t('appDialog.confirm') }}
            </button>
          </template>
          <template v-else>
            <button
              type="button"
              class="px-4 py-2 rounded-lg bg-pink-500 text-white text-xs font-semibold shadow-sm shadow-pink-500/25 transition hover:bg-pink-600"
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

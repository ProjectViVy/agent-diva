<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  getAppDialogOpen,
  dismissAppDialogConfirm,
  dismissAppDialogAlert,
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

const shellTheme = computed(() => `theme-${props.themeMode || 'love'}`);

function onBackdropClick() {
  const d = open.value;
  if (!d) return;
  if (d.kind === 'confirm') {
    dismissAppDialogConfirm(false);
  }
}

function onEscape(e: KeyboardEvent) {
  if (e.key !== 'Escape' || !open.value) return;
  if (open.value.kind === 'confirm') {
    dismissAppDialogConfirm(false);
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

<script setup lang="ts">
import { MessageSquare } from 'lucide-vue-next';
import { onMounted, onUnmounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { CortexState } from '../../api/cortex';
import { isTauriRuntime } from '../../api/desktop';
import { setCortexEnabled } from '../../api/cortex';
import {
  getNeuroOverviewSnapshot,
  previewNeuroOverviewSnapshot,
  type NeuroOverviewSnapshotV0,
} from '../../api/neuro';
import BrainOverview from './BrainOverview.vue';
import NeuroDetailPanel from './NeuroDetailPanel.vue';

const { t } = useI18n();

const emit = defineEmits<{
  (e: 'back'): void;
  (e: 'open-settings'): void;
}>();

const selectedSide = ref<'left' | 'right'>('left');
const snapshot = ref<NeuroOverviewSnapshotV0 | null>(null);
const loading = ref(true);
const loadError = ref(false);

let unlistenCortex: UnlistenFn | null = null;

const onBack = () => {
  emit('back');
};

async function onDisableCortex() {
  if (!isTauriRuntime()) {
    return;
  }
  try {
    await setCortexEnabled(false);
    await refreshSnapshot();
  } catch (e) {
    console.error('[NervousSystemView] set_cortex_enabled(false) failed', e);
  }
}

const onSelectHemisphere = (side: 'left' | 'right') => {
  selectedSide.value = side;
};

async function refreshSnapshot() {
  if (!isTauriRuntime()) {
    snapshot.value = previewNeuroOverviewSnapshot();
    loadError.value = false;
    loading.value = false;
    return;
  }
  loading.value = true;
  loadError.value = false;
  try {
    snapshot.value = await getNeuroOverviewSnapshot();
  } catch (e) {
    console.error('[NervousSystemView] get_neuro_overview_snapshot failed', e);
    loadError.value = true;
    snapshot.value = null;
  } finally {
    loading.value = false;
  }
}

onMounted(async () => {
  await refreshSnapshot();
  if (isTauriRuntime()) {
    try {
      unlistenCortex = await listen<CortexState>('cortex_toggled', () => {
        void refreshSnapshot();
      });
    } catch (e) {
      console.error('[NervousSystemView] cortex_toggled listen failed', e);
    }
  }
});

onUnmounted(() => {
  if (unlistenCortex) {
    unlistenCortex();
    unlistenCortex = null;
  }
});
</script>

<template>
  <div class="neuro-shell flex h-full min-h-0 flex-col overflow-hidden">
    <header
      class="flex shrink-0 items-center justify-between gap-3 border-b border-gray-200/80 bg-white/90 px-4 py-3"
    >
      <h2 class="text-sm font-semibold tracking-tight text-gray-900">
        {{ t('neuro.viewTitle') }}
      </h2>
      <button
        type="button"
        class="inline-flex items-center gap-1.5 rounded-lg border border-gray-200 bg-white px-3 py-1.5 text-xs font-medium text-gray-700 shadow-sm transition hover:border-pink-200 hover:bg-pink-50 hover:text-pink-800"
        @click="onBack"
      >
        <MessageSquare :size="14" class="text-pink-500" />
        {{ t('neuro.backToChat') }}
      </button>
    </header>

    <div class="flex min-h-0 flex-1 flex-col overflow-hidden md:flex-row">
      <div class="min-h-0 min-w-0 flex-1 overflow-auto p-4">
        <BrainOverview @select-hemisphere="onSelectHemisphere" />
        <!-- FR16：愿景占位折叠，非 MVP 主路径（Story 3.1 测试与 3.4 边界） -->
        <details class="vision-stub mt-6 rounded-xl border border-violet-100 bg-violet-50/40 px-4 py-3 text-left">
          <summary
            class="cursor-pointer list-none text-sm font-medium text-violet-900 outline-none marker:content-none [&::-webkit-details-marker]:hidden"
          >
            <span
              class="mr-2 inline-flex rounded-md border border-violet-200 bg-white/90 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-violet-800"
            >
              {{ t('neuro.vision.badge') }}
            </span>
            <span class="text-violet-900/90">{{ t('neuro.vision.title') }}</span>
          </summary>
          <div class="mt-3 space-y-2 border-t border-violet-100/80 pt-3 text-xs leading-relaxed text-violet-950/85">
            <p>{{ t('neuro.vision.subtitle') }}</p>
            <p>{{ t('neuro.vision.body') }}</p>
          </div>
        </details>
      </div>
      <NeuroDetailPanel
        :side="selectedSide"
        :snapshot="snapshot"
        :loading="loading"
        :load-error="loadError"
        :show-disable-cortex-action="isTauriRuntime()"
        @retry="() => void refreshSnapshot()"
        @back-to-chat="onBack"
        @open-settings="emit('open-settings')"
        @disable-cortex="() => void onDisableCortex()"
      />
    </div>
  </div>
</template>

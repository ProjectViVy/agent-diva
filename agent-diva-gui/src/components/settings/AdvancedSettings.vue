<script setup lang="ts">
import { onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { getRunTelemetrySnapshot, type RunTelemetrySnapshotV0 } from '../../api/runTelemetry';
import RunTelemetryHint from '../RunTelemetryHint.vue';

const { t } = useI18n();

const LS_FEATURE = 'agentDivaFeatureRunTelemetry';

const showDevTelemetry = ref(false);
const snapshot = ref<RunTelemetrySnapshotV0 | null>(null);
const loading = ref(false);
const loadError = ref<string | null>(null);

function readFlag(): boolean {
  try {
    return localStorage.getItem(LS_FEATURE) === '1';
  } catch {
    return false;
  }
}

function writeFlag(on: boolean) {
  try {
    if (on) localStorage.setItem(LS_FEATURE, '1');
    else localStorage.removeItem(LS_FEATURE);
  } catch {
    /* ignore */
  }
}

async function refreshSnapshot() {
  loadError.value = null;
  loading.value = true;
  try {
    snapshot.value = await getRunTelemetrySnapshot();
  } catch (e) {
    loadError.value = e instanceof Error ? e.message : String(e);
    snapshot.value = null;
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  showDevTelemetry.value = readFlag();
  if (showDevTelemetry.value) {
    void refreshSnapshot();
  }
});

watch(showDevTelemetry, (on) => {
  writeFlag(on);
  if (on) void refreshSnapshot();
});
</script>

<template>
  <div class="p-6 max-w-2xl mx-auto space-y-6">
    <div>
      <h3 class="text-lg font-semibold text-gray-900">{{ t('advanced.title') }}</h3>
      <p class="text-sm text-gray-500 mt-1">{{ t('advanced.intro') }}</p>
    </div>

    <div class="flex items-start gap-3 rounded-xl border border-gray-100 bg-white p-4 shadow-sm">
      <input
        id="flag-run-telemetry"
        v-model="showDevTelemetry"
        type="checkbox"
        class="mt-1 h-4 w-4 rounded border-gray-300 text-pink-600 focus:ring-pink-500"
      />
      <label for="flag-run-telemetry" class="text-sm text-gray-700 cursor-pointer">
        <span class="font-medium block">{{ t('advanced.featureRunTelemetry') }}</span>
        <span class="text-gray-500 block mt-0.5">{{ t('advanced.featureRunTelemetryHint') }}</span>
      </label>
    </div>

    <div v-if="showDevTelemetry" class="space-y-3">
      <div class="flex items-center justify-between gap-2">
        <span class="text-sm font-medium text-gray-800">{{ t('advanced.runTelemetryPanelTitle') }}</span>
        <button
          type="button"
          class="text-xs px-2 py-1 rounded-md border border-gray-200 hover:bg-gray-50 text-gray-700"
          @click="refreshSnapshot"
        >
          {{ t('advanced.refresh') }}
        </button>
      </div>
      <p v-if="loadError" class="text-xs text-red-600">{{ loadError }}</p>
      <RunTelemetryHint :snapshot="snapshot" :loading="loading" />
      <p class="text-xs text-gray-400">{{ t('advanced.nfrNote') }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Loader2, Package } from 'lucide-vue-next';

import {
  getCapabilityRegistrySummary,
  isTauriRuntime,
  submitCapabilityManifestJson,
  type CapabilityManifestErrorDto,
  type RegistrySummaryDto,
} from '../../api/desktop';
import CapabilityManifestErrorsDisplay from './CapabilityManifestErrorsDisplay.vue';

const { t } = useI18n();

const jsonText = ref('');
const submitting = ref(false);
const loadError = ref('');
const validationErrors = ref<CapabilityManifestErrorDto[]>([]);
const summary = ref<RegistrySummaryDto | null>(null);
const previewMode = !isTauriRuntime();

const exampleValid = `{
  "schema_version": "0",
  "capabilities": [
    { "id": "demo.skill", "name": "Demo", "description": "Example capability" }
  ]
}`;

async function refreshSummary() {
  if (previewMode) {
    summary.value = null;
    return;
  }
  loadError.value = '';
  try {
    summary.value = await getCapabilityRegistrySummary();
  } catch (e) {
    loadError.value = String(e);
    summary.value = null;
  }
}

async function submit() {
  if (previewMode) {
    loadError.value = t('settings.capabilityManifest.previewOnly');
    return;
  }
  submitting.value = true;
  validationErrors.value = [];
  loadError.value = '';
  try {
    const result = await submitCapabilityManifestJson(jsonText.value);
    if (result.ok && result.summary) {
      summary.value = result.summary;
      validationErrors.value = [];
    } else {
      validationErrors.value = result.errors ?? [];
      if (result.message) {
        loadError.value = result.message;
      } else if (!result.ok && validationErrors.value.length === 0) {
        loadError.value = t('settings.capabilityManifest.unknownFailure');
      }
    }
  } catch (e) {
    loadError.value = String(e);
  } finally {
    submitting.value = false;
  }
}

function loadExample() {
  jsonText.value = exampleValid;
  validationErrors.value = [];
  loadError.value = '';
}

onMounted(refreshSummary);
</script>

<template>
  <section
    class="bg-white border border-gray-100 rounded-xl p-4 flex flex-col min-h-[280px]"
    :aria-busy="submitting"
  >
    <div class="flex items-center gap-2 text-gray-800 mb-1">
      <div class="w-9 h-9 rounded-lg bg-violet-100 text-violet-700 flex items-center justify-center">
        <Package :size="18" aria-hidden="true" />
      </div>
      <div>
        <h4 class="font-bold text-base">{{ t('settings.capabilityManifest.sectionTitle') }}</h4>
        <p class="text-xs text-gray-500">{{ t('settings.capabilityManifest.sectionHint') }}</p>
      </div>
    </div>

    <div class="mt-4 flex-1 flex flex-col gap-3 min-h-0">
      <div class="flex flex-col gap-1">
        <label for="cap-manifest-json" class="text-sm font-medium text-gray-700">
          {{ t('settings.capabilityManifest.jsonLabel') }}
        </label>
        <textarea
          id="cap-manifest-json"
          v-model="jsonText"
          rows="12"
          class="w-full rounded-lg border border-gray-200 px-3 py-2 text-sm font-mono text-gray-900 focus:ring-2 focus:ring-violet-300 focus:border-violet-400 outline-none resize-y min-h-[160px]"
          :placeholder="t('settings.capabilityManifest.placeholder')"
          spellcheck="false"
          autocomplete="off"
        />
      </div>

      <CapabilityManifestErrorsDisplay v-if="validationErrors.length" :errors="validationErrors" />

      <p v-if="loadError" class="text-sm text-red-700" role="alert">{{ loadError }}</p>

      <div
        v-if="summary !== null && !validationErrors.length && !loadError"
        class="rounded-lg border border-emerald-100 bg-emerald-50/80 p-3 text-sm text-emerald-950"
        role="status"
      >
        <p class="font-medium">{{ t('settings.capabilityManifest.registeredTitle', { count: summary.count }) }}</p>
        <p v-if="summary.count === 0" class="mt-1 text-emerald-900">
          {{ t('settings.capabilityManifest.emptyRegistry') }}
        </p>
        <ul v-else class="mt-2 list-disc list-inside font-mono text-xs break-all">
          <li v-for="id in summary.ids" :key="id">{{ id }}</li>
        </ul>
      </div>

      <div class="flex flex-wrap items-center justify-end gap-2 pt-2 border-t border-gray-100 mt-auto">
        <button
          type="button"
          class="px-3 py-2 text-sm rounded-lg border border-gray-200 text-gray-700 hover:bg-gray-50"
          @click="loadExample"
        >
          {{ t('settings.capabilityManifest.loadExample') }}
        </button>
        <button
          type="button"
          class="px-4 py-2 text-sm rounded-lg bg-violet-600 text-white hover:bg-violet-700 disabled:opacity-50 inline-flex items-center gap-2"
          :disabled="submitting || previewMode"
          @click="submit"
        >
          <Loader2 v-if="submitting" class="animate-spin" :size="16" aria-hidden="true" />
          {{ submitting ? t('settings.capabilityManifest.saving') : t('settings.capabilityManifest.saveValidate') }}
        </button>
      </div>
    </div>
  </section>
</template>

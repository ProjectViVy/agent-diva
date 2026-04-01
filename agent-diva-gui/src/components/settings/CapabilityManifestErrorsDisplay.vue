<script setup lang="ts">
import { computed } from 'vue';
import { AlertCircle, FileWarning } from 'lucide-vue-next';

import type { CapabilityManifestErrorDto } from '../../api/desktop';

const props = defineProps<{
  errors: CapabilityManifestErrorDto[];
}>();

const fileErrors = computed(() =>
  props.errors.filter((e) => e.location_kind === 'file')
);
const fieldErrors = computed(() =>
  props.errors.filter((e) => e.location_kind === 'field')
);
</script>

<template>
  <div
    v-if="errors.length"
    class="space-y-4"
    role="alert"
    aria-live="polite"
  >
    <div v-if="fileErrors.length" class="rounded-lg border border-amber-200 bg-amber-50/90 p-3">
      <div class="flex items-center gap-2 text-amber-900 font-medium text-sm mb-2">
        <FileWarning :size="18" class="shrink-0" aria-hidden="true" />
        <span>{{ $t('settings.capabilityManifest.fileLevelTitle') }}</span>
      </div>
      <ul class="list-none space-y-2 text-sm text-amber-950">
        <li
          v-for="(err, i) in fileErrors"
          :key="'f-' + i + err.code"
          class="pl-1 border-l-2 border-amber-400"
        >
          <span class="font-mono text-xs text-amber-800">{{ err.code }}</span>
          <p class="mt-0.5">{{ err.message }}</p>
        </li>
      </ul>
    </div>

    <div v-if="fieldErrors.length" class="rounded-lg border border-rose-200 bg-rose-50/90 p-3">
      <div class="flex items-center gap-2 text-rose-900 font-medium text-sm mb-2">
        <AlertCircle :size="18" class="shrink-0" aria-hidden="true" />
        <span>{{ $t('settings.capabilityManifest.fieldLevelTitle') }}</span>
      </div>
      <ul class="list-none space-y-2 text-sm text-rose-950">
        <li
          v-for="(err, i) in fieldErrors"
          :key="'e-' + i + err.code + (err.path ?? '')"
          class="pl-1 border-l-2 border-rose-400"
        >
          <span class="font-mono text-xs text-rose-800">{{ err.code }}</span>
          <p v-if="err.path" class="text-xs text-rose-700 mt-0.5 font-mono">{{ err.path }}</p>
          <p class="mt-0.5">{{ err.message }}</p>
        </li>
      </ul>
    </div>
  </div>
</template>

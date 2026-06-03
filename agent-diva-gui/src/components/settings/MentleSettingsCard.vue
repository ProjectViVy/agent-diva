<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { BrainCircuit, LoaderCircle, RotateCcw } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { listMentleTools, type MentleToolConfigShape } from '../../api/desktop';

interface ToolsConfigShape {
  web: {
    search: {
      provider: string;
      enabled: boolean;
      api_key: string;
      max_results: number;
    };
    fetch: {
      enabled: boolean;
    };
  };
  mentle: MentleToolConfigShape;
}

const DEFAULT_MENTLE: MentleToolConfigShape = {
  enabled: false,
  mode: 'off',
  allowed_tools: [],
};

const FALLBACK_TOOLS = ['memtle_status', 'memtle_search'];

const { t } = useI18n();

const props = defineProps<{
  toolsConfig: ToolsConfigShape;
  saveToolsConfigAction: (tools: ToolsConfigShape) => Promise<void>;
}>();

const localConfig = ref<ToolsConfigShape>(JSON.parse(JSON.stringify(props.toolsConfig)));
const lastSavedSnapshot = ref(JSON.stringify(props.toolsConfig));
const isSaving = ref(false);
const discoveredTools = ref<string[]>([]);
const discoveryAvailable = ref(false);
const discoveryLoading = ref(false);

watch(
  () => props.toolsConfig,
  (val) => {
    localConfig.value = JSON.parse(JSON.stringify(val));
    lastSavedSnapshot.value = JSON.stringify(val);
  },
  { deep: true }
);

const modeOptions = [
  { value: 'off', labelKey: 'general.mentleModeOff' },
  { value: 'read_only', labelKey: 'general.mentleModeReadOnly' },
  { value: 'full', labelKey: 'general.mentleModeFull' },
  { value: 'custom', labelKey: 'general.mentleModeCustom' },
] as const;

const checklistTools = computed(() => {
  if (discoveredTools.value.length > 0) {
    return discoveredTools.value;
  }
  return FALLBACK_TOOLS;
});

const showCustomChecklist = computed(() => localConfig.value.mentle.mode === 'custom');

const currentSnapshot = computed(() => JSON.stringify(localConfig.value));
const isDirty = computed(() => currentSnapshot.value !== lastSavedSnapshot.value);

const syncEnabledWithMode = () => {
  if (localConfig.value.mentle.mode === 'off') {
    localConfig.value.mentle.enabled = false;
    return;
  }
};

watch(
  () => localConfig.value.mentle.enabled,
  (enabled) => {
    if (enabled && localConfig.value.mentle.mode === 'off') {
      localConfig.value.mentle.mode = 'read_only';
    }
    if (!enabled) {
      localConfig.value.mentle.mode = 'off';
    }
  }
);

watch(
  () => localConfig.value.mentle.mode,
  () => {
    syncEnabledWithMode();
  }
);

const toggleAllowedTool = (toolName: string) => {
  const allowed = localConfig.value.mentle.allowed_tools;
  const index = allowed.indexOf(toolName);
  if (index >= 0) {
    allowed.splice(index, 1);
  } else {
    allowed.push(toolName);
  }
};

const isToolAllowed = (toolName: string) =>
  localConfig.value.mentle.allowed_tools.includes(toolName);

const loadDiscoveredTools = async () => {
  discoveryLoading.value = true;
  try {
    const response = await listMentleTools();
    discoveryAvailable.value = response.feature_available;
    discoveredTools.value = response.tools;
  } catch (error) {
    console.warn('Failed to load Mentle tool metadata:', error);
    discoveryAvailable.value = false;
    discoveredTools.value = [];
  } finally {
    discoveryLoading.value = false;
  }
};

onMounted(() => {
  void loadDiscoveredTools();
});

const saveConfig = async () => {
  if (isSaving.value || !isDirty.value) {
    return;
  }
  isSaving.value = true;
  try {
    syncEnabledWithMode();
    await props.saveToolsConfigAction(JSON.parse(JSON.stringify(localConfig.value)));
    lastSavedSnapshot.value = JSON.stringify(localConfig.value);
  } finally {
    isSaving.value = false;
  }
};

const resetDefaults = () => {
  localConfig.value = {
    ...JSON.parse(JSON.stringify(localConfig.value)),
    mentle: { ...DEFAULT_MENTLE },
  };
};
</script>

<template>
  <div class="bg-white border border-gray-100 rounded-xl p-4 space-y-4">
    <div class="flex items-start justify-between gap-4">
      <div class="flex items-center space-x-2 text-gray-700">
        <BrainCircuit :size="16" class="text-violet-500" />
        <div>
          <span class="text-sm font-semibold">{{ t('general.mentleTitle') }}</span>
          <p class="text-xs text-gray-500 mt-0.5">{{ t('general.mentleDesc') }}</p>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <button
          type="button"
          class="inline-flex items-center gap-1 rounded-lg border border-gray-200 px-3 py-1.5 text-xs text-gray-600 hover:bg-gray-50"
          @click="resetDefaults"
        >
          <RotateCcw :size="14" />
          {{ t('general.mentleReset') }}
        </button>
        <button
          type="button"
          :disabled="!isDirty || isSaving"
          class="inline-flex items-center gap-1 rounded-lg bg-violet-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-violet-700 disabled:cursor-not-allowed disabled:opacity-50"
          @click="saveConfig"
        >
          <LoaderCircle v-if="isSaving" :size="14" class="animate-spin" />
          {{ t('settings.save') }}
        </button>
      </div>
    </div>

    <label class="text-sm text-gray-700 flex items-center space-x-2">
      <input
        v-model="localConfig.mentle.enabled"
        type="checkbox"
        class="rounded border-gray-300 text-violet-600 focus:ring-violet-500"
      />
      <span>{{ t('general.mentleEnabled') }}</span>
    </label>

    <div class="space-y-2">
      <label class="text-xs font-medium uppercase tracking-wider text-gray-400">
        {{ t('general.mentleModeLabel') }}
      </label>
      <select
        v-model="localConfig.mentle.mode"
        class="w-full max-w-md rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm text-gray-800 focus:outline-none focus:ring-2 focus:ring-violet-500/40"
      >
        <option v-for="option in modeOptions" :key="option.value" :value="option.value">
          {{ t(option.labelKey) }}
        </option>
      </select>
      <p class="text-xs text-gray-500 leading-relaxed">{{ t('general.mentleModeHint') }}</p>
    </div>

    <div v-if="showCustomChecklist" class="space-y-2">
      <div class="flex items-center justify-between gap-3">
        <label class="text-xs font-medium uppercase tracking-wider text-gray-400">
          {{ t('general.mentleCustomTools') }}
        </label>
        <button
          type="button"
          class="text-xs text-violet-600 hover:text-violet-700"
          @click="loadDiscoveredTools"
        >
          {{ t('general.mentleRefreshTools') }}
        </button>
      </div>

      <p v-if="discoveryLoading" class="text-xs text-gray-500">{{ t('general.mentleLoadingTools') }}</p>
      <p v-else-if="!discoveryAvailable" class="text-xs text-amber-700">
        {{ t('general.mentleDiscoveryUnavailable') }}
      </p>

      <div class="space-y-2 pl-1">
        <label
          v-for="toolName in checklistTools"
          :key="toolName"
          class="text-sm text-gray-700 flex items-center space-x-2"
        >
          <input
            type="checkbox"
            :checked="isToolAllowed(toolName)"
            class="rounded border-gray-300 text-violet-600 focus:ring-violet-500"
            @change="toggleAllowedTool(toolName)"
          />
          <span class="font-mono text-xs">{{ toolName }}</span>
        </label>
      </div>
    </div>

    <p class="text-xs text-gray-500 leading-relaxed border-t border-gray-100 pt-3">
      {{ t('general.mentleRuntimeNote') }}
    </p>
  </div>
</template>

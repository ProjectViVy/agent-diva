<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Globe, LoaderCircle } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { openExternalUrl } from '../../utils/openExternal';

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
}

const { t } = useI18n();

const props = defineProps<{
  toolsConfig: ToolsConfigShape;
  saveToolsConfigAction: (tools: ToolsConfigShape) => Promise<void>;
}>();

const localConfig = ref<ToolsConfigShape>(JSON.parse(JSON.stringify(props.toolsConfig)));
const lastSavedSnapshot = ref(JSON.stringify(props.toolsConfig));
const isSaving = ref(false);
const bochaApiKeyGuideUrl = 'https://aq6ky2b8nql.feishu.cn/wiki/HmtOw1z6vik14Fkdu5uc9VaInBb';

const isExtendedSearchProvider = (provider: string) =>
  provider === 'zhipu' || provider === 'bocha';

watch(
  () => props.toolsConfig,
  (val) => {
    localConfig.value = JSON.parse(JSON.stringify(val));
    lastSavedSnapshot.value = JSON.stringify(val);
  },
  { deep: true }
);

const sanitizeConfig = (source: ToolsConfigShape): ToolsConfigShape => {
  const sanitized = JSON.parse(JSON.stringify(source)) as ToolsConfigShape;
  const provider = sanitized.web.search.provider;
  const maxLimit = isExtendedSearchProvider(provider) ? 50 : 10;
  sanitized.web.search.max_results = Math.min(
    maxLimit,
    Math.max(1, Number(sanitized.web.search.max_results) || 5)
  );
  return sanitized;
};

const syncSanitizedMaxResults = () => {
  const sanitized = sanitizeConfig(localConfig.value);
  if (sanitized.web.search.max_results !== localConfig.value.web.search.max_results) {
    localConfig.value.web.search.max_results = sanitized.web.search.max_results;
  }
};

const currentSnapshot = computed(() => JSON.stringify(sanitizeConfig(localConfig.value)));
const isDirty = computed(() => currentSnapshot.value !== lastSavedSnapshot.value);

const maxResultsLimit = computed(() =>
  isExtendedSearchProvider(localConfig.value.web.search.provider) ? 50 : 10
);

const isBochaProvider = computed(() =>
  localConfig.value.web.search.provider === 'bocha'
);

const apiKeyLabel = computed(() =>
  isBochaProvider.value
    ? t('network.apiKeyBocha')
    : localConfig.value.web.search.provider === 'zhipu'
      ? t('network.apiKeyZhipu')
      : t('network.apiKeyBrave')
);

const apiKeyPlaceholder = computed(() =>
  isBochaProvider.value
    ? t('network.apiKeyPlaceholderBocha')
    : localConfig.value.web.search.provider === 'zhipu'
      ? t('network.apiKeyPlaceholderZhipu')
      : t('network.apiKeyPlaceholderBrave')
);

watch(
  () => localConfig.value.web.search.provider,
  () => {
    syncSanitizedMaxResults();
  }
);

const saveConfig = async () => {
  if (isSaving.value || !isDirty.value) return;
  isSaving.value = true;
  try {
    const sanitized = sanitizeConfig(localConfig.value);
    localConfig.value = JSON.parse(JSON.stringify(sanitized));
    await props.saveToolsConfigAction(sanitized);
    lastSavedSnapshot.value = JSON.stringify(sanitized);
  } finally {
    isSaving.value = false;
  }
};

const openBochaGuide = () => {
  void openExternalUrl(bochaApiKeyGuideUrl);
};
</script>

<template>
  <div class="p-6 space-y-6 fade-in">
    <div class="flex items-start justify-between gap-4">
      <div class="flex items-center space-x-3">
        <div class="w-10 h-10 rounded-lg bg-blue-100 text-blue-600 flex items-center justify-center">
          <Globe :size="20" />
        </div>
        <div>
          <h3 class="text-lg font-bold text-gray-800">{{ t('network.title') }}</h3>
          <p class="text-sm text-gray-500">{{ t('network.desc') }}</p>
        </div>
      </div>
      <button
        type="button"
        class="btn-save-config inline-flex min-w-[112px] items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-semibold"
        :disabled="isSaving || !isDirty"
        @click="saveConfig"
      >
        <LoaderCircle v-if="isSaving" :size="16" class="animate-spin" />
        <span>{{ isSaving ? t('console.saving') : t('console.saveConfig') }}</span>
      </button>
    </div>

    <div class="bg-white border border-gray-100 rounded-xl p-4 space-y-4">
      <div class="grid grid-cols-2 gap-4">
        <div class="space-y-1">
          <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('network.provider') }}</label>
          <select v-model="localConfig.web.search.provider" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg text-sm">
            <option value="bocha">{{ t('network.providerBocha') }}</option>
            <option value="brave">{{ t('network.providerBrave') }}</option>
            <option value="zhipu">{{ t('network.providerZhipu') }}</option>
          </select>
        </div>
        <div class="space-y-1">
          <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('network.maxResults') }}</label>
          <input v-model.number="localConfig.web.search.max_results" type="number" min="1" :max="maxResultsLimit" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg text-sm" />
        </div>
      </div>

      <div class="space-y-1">
        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ apiKeyLabel }}</label>
        <input v-model="localConfig.web.search.api_key" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg font-mono text-sm" :placeholder="apiKeyPlaceholder" />
        <button
          v-if="isBochaProvider"
          type="button"
          class="inline-flex text-sm text-blue-600 hover:text-blue-700 underline-offset-2 hover:underline bg-transparent border-0 p-0 cursor-pointer text-left"
          @click="openBochaGuide"
        >
          {{ t('network.apiKeyGuideBocha') }}
        </button>
      </div>

      <div class="flex space-x-6">
        <label class="text-sm text-gray-600 flex items-center space-x-2">
          <input type="checkbox" v-model="localConfig.web.search.enabled" />
          <span>{{ t('network.enableSearch') }}</span>
        </label>
        <label class="text-sm text-gray-600 flex items-center space-x-2">
          <input type="checkbox" v-model="localConfig.web.fetch.enabled" />
          <span>{{ t('network.enableFetch') }}</span>
        </label>
      </div>
    </div>
  </div>
</template>

<style scoped>
.fade-in {
  animation: slideIn 0.3s ease-out;
}

@keyframes slideIn {
  from { opacity: 0; transform: translateX(20px); }
  to { opacity: 1; transform: translateX(0); }
}
</style>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Save, Globe } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const props = defineProps<{
  toolsConfig: {
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
  };
}>();

const emit = defineEmits<{
  (e: 'save-tools-config', tools: typeof props.toolsConfig): void;
}>();

const localConfig = ref(JSON.parse(JSON.stringify(props.toolsConfig)));

watch(
  () => props.toolsConfig,
  (val) => {
    localConfig.value = JSON.parse(JSON.stringify(val));
  },
  { deep: true }
);

const save = () => {
  const maxLimit = localConfig.value.web.search.provider === 'zhipu' ? 50 : 10;
  localConfig.value.web.search.max_results = Math.min(
    maxLimit,
    Math.max(1, Number(localConfig.value.web.search.max_results) || 5)
  );
  emit('save-tools-config', localConfig.value);
};

const maxResultsLimit = computed(() =>
  localConfig.value.web.search.provider === 'zhipu' ? 50 : 10
);

const apiKeyLabel = computed(() =>
  localConfig.value.web.search.provider === 'zhipu'
    ? 'Zhipu API Key'
    : t('network.apiKey')
);

const apiKeyPlaceholder = computed(() =>
  localConfig.value.web.search.provider === 'zhipu'
    ? 'Enter Zhipu API key...'
    : t('network.apiKeyPlaceholder')
);

watch(
  () => localConfig.value.web.search.provider,
  () => {
    localConfig.value.web.search.max_results = Math.min(
      maxResultsLimit.value,
      Math.max(1, Number(localConfig.value.web.search.max_results) || 5)
    );
  }
);
</script>

<template>
  <div class="p-6 space-y-6 fade-in">
    <div class="flex items-center space-x-3">
      <div class="w-10 h-10 rounded-lg bg-blue-100 text-blue-600 flex items-center justify-center">
        <Globe :size="20" />
      </div>
      <div>
        <h3 class="text-lg font-bold text-gray-800">{{ t('network.title') }}</h3>
        <p class="text-sm text-gray-500">{{ t('network.desc') }}</p>
      </div>
    </div>

    <div class="bg-white border border-gray-100 rounded-xl p-4 space-y-4">
      <div class="grid grid-cols-2 gap-4">
        <div class="space-y-1">
          <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('network.provider') }}</label>
          <select v-model="localConfig.web.search.provider" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg text-sm">
            <option value="brave">Brave</option>
            <option value="zhipu">Zhipu</option>
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

    <div>
      <button @click="save" class="px-6 py-3 bg-gradient-to-r from-blue-500 to-blue-600 text-white rounded-full font-medium shadow-lg flex items-center space-x-2">
        <Save :size="18" />
        <span>{{ t('network.save') }}</span>
      </button>
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

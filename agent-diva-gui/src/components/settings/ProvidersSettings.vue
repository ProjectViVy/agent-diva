<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Server, Check, Cpu, Save } from 'lucide-vue-next';

interface ProviderSpec {
  name: string;
  api_type: string;
  keywords: string[];
  env_key: string;
  display_name: string;
  litellm_prefix: string;
  skip_prefixes: string[];
  is_gateway: boolean;
  is_local: boolean;
  default_api_base: string;
  models: string[];
}

interface SavedModel {
  id: string;
  provider: string;
  model: string;
  apiBase: string;
  apiKey: string;
  displayName: string;
}

const props = defineProps<{
  config: {
    apiBase: string;
    apiKey: string;
    model: string;
  };
  savedModels?: SavedModel[];
  lang: 'zh' | 'en';
}>();

const emit = defineEmits<{
  (e: 'save', config: typeof props.config): void;
  (e: 'update-saved-models', models: SavedModel[]): void;
}>();

const providers = ref<ProviderSpec[]>([]);
const localConfig = ref({ ...props.config });
const selectedProvider = ref<ProviderSpec | null>(null);
const searchTerm = ref('');
const providerApiKeys = ref<Record<string, string>>({});

// Computed translations
const t = computed(() => {
  return props.lang === 'zh' ? {
    searchProviders: '搜索供应商...',
    local: '本地',
    gateway: '网关',
    cloud: '云端',
    connectConfig: '连接配置',
    apiKey: 'API Key',
    enterApiKey: '请输入 API Key...',
    apiBaseUrl: 'API Base URL',
    customApi: '自定义 API 地址',
    availableModels: '可用模型',
    checkToAdd: '勾选以添加到快捷切换列表',
    noModels: '该供应商暂无预设模型',
    selectProvider: '请从左侧选择一个供应商以配置模型',
    saveConfig: '保存当前配置',
    saving: '保存中...',
    saved: '已保存',
    saveFailed: '保存失败',
  } : {
    searchProviders: 'Search Providers...',
    local: 'Local',
    gateway: 'Gateway',
    cloud: 'Cloud',
    connectConfig: 'Connection Config',
    apiKey: 'API Key',
    enterApiKey: 'Enter API Key...',
    apiBaseUrl: 'API Base URL',
    customApi: 'Custom API Address',
    availableModels: 'Available Models',
    checkToAdd: 'Check to add to shortcuts',
    noModels: 'No models available for this provider',
    selectProvider: 'Select a provider from the left to configure models',
    saveConfig: 'Save Config',
    saving: 'Saving...',
    saved: 'Saved',
    saveFailed: 'Save Failed',
  };
});

onMounted(async () => {
  try {
    providers.value = await invoke('get_providers');
    
    if (props.savedModels) {
      props.savedModels.forEach(m => {
        if (m.apiKey) {
          providerApiKeys.value[m.provider] = m.apiKey;
        }
      });
    }
    
    if (providers.value.length > 0) {
      let found = providers.value.find(p => p.default_api_base === localConfig.value.apiBase);
      selectedProvider.value = found || providers.value[0];
      
      if (found && props.config.apiKey) {
        providerApiKeys.value[found.name] = props.config.apiKey;
      }
    }
  } catch (e) {
    console.error('Failed to load providers:', e);
  }
});

const filteredProviders = computed(() => {
  if (!searchTerm.value) return providers.value;
  const lower = searchTerm.value.toLowerCase();
  return providers.value.filter(p => 
    p.display_name.toLowerCase().includes(lower) || 
    p.name.toLowerCase().includes(lower)
  );
});

const selectProvider = (provider: ProviderSpec) => {
  selectedProvider.value = provider;
  localConfig.value.apiBase = provider.default_api_base;
  const key = providerApiKeys.value[provider.name] || '';
  localConfig.value.apiKey = key;
};

const isModelSaved = (providerName: string, modelName: string) => {
  return props.savedModels?.some(m => m.provider === providerName && m.model === modelName) ?? false;
};

const toggleModel = (modelName: string) => {
  if (!selectedProvider.value) return;
  const provider = selectedProvider.value;
  const providerKey = providerApiKeys.value[provider.name] || '';
  
  localConfig.value.model = modelName;
  localConfig.value.apiBase = provider.default_api_base;
  localConfig.value.apiKey = providerKey;
  
  const existingIndex = props.savedModels?.findIndex(m => m.provider === provider.name && m.model === modelName) ?? -1;
  
  let newSavedModels = [...(props.savedModels || [])];
  
  if (existingIndex >= 0) {
    newSavedModels.splice(existingIndex, 1);
  } else {
    newSavedModels.push({
      id: `${provider.name}:${modelName}`,
      provider: provider.name,
      model: modelName,
      apiBase: provider.default_api_base,
      apiKey: providerKey,
      displayName: `${provider.display_name} - ${modelName}`
    });
  }
  
  emit('update-saved-models', newSavedModels);
};

const updateProviderKey = (key: string) => {
  if (!selectedProvider.value) return;
  const providerName = selectedProvider.value.name;
  providerApiKeys.value[providerName] = key;
  localConfig.value.apiKey = key;
  
  if (props.savedModels) {
    const newModels = props.savedModels.map(m => {
      if (m.provider === providerName) {
        return { ...m, apiKey: key };
      }
      return m;
    });
    if (JSON.stringify(newModels) !== JSON.stringify(props.savedModels)) {
        emit('update-saved-models', newModels);
    }
  }
};

const handleSave = () => {
  emit('save', localConfig.value);
};

watch(() => props.config, (newVal) => {
  localConfig.value = { ...newVal };
}, { deep: true });
</script>

<template>
  <div class="flex h-full fade-in">
    <!-- Sidebar: List of Providers -->
    <div class="w-1/3 min-w-[200px] border-r border-gray-100 flex flex-col bg-gray-50/30">
      <div class="p-4 border-b border-gray-100">
        <input 
          v-model="searchTerm" 
          :placeholder="t.searchProviders" 
          class="w-full px-4 py-2 bg-white border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all"
        />
      </div>
      
      <div class="flex-1 overflow-y-auto p-2 space-y-1">
          <button
            v-for="provider in filteredProviders"
            :key="provider.name"
            @click="selectProvider(provider)"
            class="w-full text-left px-4 py-3 rounded-xl transition-all flex items-center justify-between group"
            :class="selectedProvider?.name === provider.name ? 'bg-white shadow-sm border-l-4 border-pink-500 text-pink-700' : 'hover:bg-gray-100 text-gray-600 border-l-4 border-transparent'"
          >
            <div class="flex items-center">
               <div class="w-8 h-8 rounded-lg flex items-center justify-center mr-3" :class="selectedProvider?.name === provider.name ? 'bg-pink-100 text-pink-600' : 'bg-gray-200 text-gray-500'">
                  <Server :size="16" />
               </div>
               <div>
                  <div class="font-medium">{{ provider.display_name }}</div>
                  <div class="text-[10px] uppercase tracking-wider opacity-70" :class="provider.is_local ? 'text-green-600' : (provider.is_gateway ? 'text-purple-600' : 'text-blue-600')">
                      {{ provider.is_local ? t.local : (provider.is_gateway ? t.gateway : t.cloud) }}
                  </div>
               </div>
            </div>
            
            <div v-if="savedModels?.some(m => m.provider === provider.name)" class="bg-pink-100 text-pink-600 text-[10px] font-bold px-2 py-0.5 rounded-full">
                {{ savedModels?.filter(m => m.provider === provider.name).length }}
            </div>
          </button>
      </div>
    </div>

    <!-- Main Area -->
    <div class="flex-1 overflow-y-auto p-6 bg-white relative">
      <div v-if="selectedProvider" class="space-y-8 pb-20">
        <!-- Header -->
        <div class="flex items-center justify-between">
            <div class="flex items-center space-x-4">
                <div class="w-12 h-12 bg-gradient-to-br from-pink-500 to-purple-600 rounded-xl flex items-center justify-center text-white shadow-lg shadow-pink-500/30">
                    <Server :size="24" />
                </div>
                <div>
                    <h3 class="text-xl font-bold text-gray-800">{{ selectedProvider.display_name }}</h3>
                    <p class="text-sm text-gray-500">{{ selectedProvider.default_api_base || t.customApi }}</p>
                </div>
            </div>
        </div>

        <!-- Configuration -->
        <div class="space-y-4 p-4 bg-gray-50 rounded-xl border border-gray-100">
           <h4 class="font-semibold text-gray-700 text-sm">{{ t.connectConfig }}</h4>
           
           <!-- API Key -->
           <div class="space-y-1">
             <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.apiKey }}</label>
             <input 
               :value="providerApiKeys[selectedProvider.name]"
               @input="e => updateProviderKey((e.target as HTMLInputElement).value)"
               type="password" 
               :placeholder="`${t.enterApiKey} (${selectedProvider.display_name})`"
               class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" 
             />
           </div>
           
           <!-- API Base -->
           <div class="space-y-1">
             <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.apiBaseUrl }}</label>
             <input 
               disabled
               :value="selectedProvider.default_api_base || 'Local/Custom'"
               class="w-full px-3 py-2 bg-gray-100 border border-gray-200 rounded-lg text-gray-500 font-mono text-sm cursor-not-allowed" 
             />
           </div>
        </div>

        <!-- Model Selection -->
        <div class="space-y-4">
          <h4 class="font-semibold text-gray-700 text-sm flex items-center justify-between">
              <span>{{ t.availableModels }}</span>
              <span class="text-xs font-normal text-gray-500">{{ t.checkToAdd }}</span>
          </h4>
          
          <div class="grid grid-cols-1 gap-2">
            <div
              v-for="model in selectedProvider.models"
              :key="model"
              @click="toggleModel(model)"
              class="px-4 py-3 rounded-xl border transition-all flex items-center justify-between cursor-pointer group"
              :class="isModelSaved(selectedProvider.name, model) ? 'border-pink-500 bg-pink-50' : 'border-gray-200 hover:border-pink-300 hover:bg-gray-50'"
            >
              <div class="flex items-center space-x-3">
                  <div class="w-5 h-5 rounded border flex items-center justify-center transition-colors"
                       :class="isModelSaved(selectedProvider.name, model) ? 'bg-pink-500 border-pink-500' : 'border-gray-300 bg-white'">
                      <Check v-if="isModelSaved(selectedProvider.name, model)" :size="12" class="text-white" />
                  </div>
                  <span class="font-medium text-sm text-gray-700">{{ model }}</span>
              </div>
              
              <!-- Status tag -->
              <span v-if="localConfig.model === model && localConfig.apiBase === selectedProvider.default_api_base" class="text-[10px] bg-green-100 text-green-700 px-2 py-0.5 rounded-full font-bold">
                  CURRENT
              </span>
            </div>
          </div>
          
          <div v-if="selectedProvider.models.length === 0" class="text-center py-8 text-gray-400 text-sm">
              {{ t.noModels }}
          </div>
        </div>
      </div>
      <div v-else class="h-full flex flex-col items-center justify-center text-gray-400 space-y-4">
        <Cpu :size="48" class="opacity-20" />
        <p>{{ t.selectProvider }}</p>
      </div>

      <!-- Floating Save Button -->
      <div class="absolute bottom-6 right-6" v-if="selectedProvider">
        <button 
          @click="handleSave" 
          class="px-6 py-3 bg-gradient-to-r from-pink-500 to-pink-600 hover:from-pink-400 hover:to-pink-500 text-white rounded-full font-medium shadow-lg shadow-pink-500/30 transition-all flex items-center space-x-2 hover:scale-105 active:scale-95"
        >
          <Save :size="18" />
          <span>{{ t.saveConfig }}</span>
        </button>
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

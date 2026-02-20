<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Server, Box, Cpu, Check, Trash2, MessageSquare, Save } from 'lucide-vue-next';

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
}>();

const emit = defineEmits<{
  (e: 'save', config: typeof props.config): void;
  (e: 'update-saved-models', models: SavedModel[]): void;
}>();

const activeTab = ref<'providers' | 'channels'>('providers');
const providers = ref<ProviderSpec[]>([]);
const channels = ref<any>({});
const localConfig = ref({ ...props.config });
const selectedProvider = ref<ProviderSpec | null>(null);
const selectedChannel = ref<string | null>(null);
const searchTerm = ref('');
// Temporary storage for API Keys while editing, indexed by provider name
const providerApiKeys = ref<Record<string, string>>({});

onMounted(async () => {
  try {
    providers.value = await invoke('get_providers');
    try {
        channels.value = await invoke('get_channels');
    } catch (e) {
        console.error('Failed to load channels:', e);
    }
    
    // Initialize API keys from saved models if available
    if (props.savedModels) {
      props.savedModels.forEach(m => {
        if (m.apiKey) {
          providerApiKeys.value[m.provider] = m.apiKey;
        }
      });
    }
    
    // Also set current config API key
    if (props.config.apiKey) {
        // Try to guess provider for current config
        // This is a bit tricky without knowing which provider the current config belongs to.
        // We'll skip for now or let user re-enter.
    }
    
    // Select first provider or current one
    if (providers.value.length > 0) {
        // Try to match current provider
        let found = providers.value.find(p => p.default_api_base === localConfig.value.apiBase);
        selectedProvider.value = found || providers.value[0];
        
        // If we found a provider for current config, set its key
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
  
  // Set current provider base URL
  // Even if default_api_base is empty (e.g. Anthropic), we must update localConfig.apiBase to match
  // Otherwise it might retain the previous provider's URL
  localConfig.value.apiBase = provider.default_api_base;
  
  // Restore key if we have one for this provider
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
  
  // Set as current model
  localConfig.value.model = modelName;
  // Always update apiBase, even if empty
  localConfig.value.apiBase = provider.default_api_base;
  localConfig.value.apiKey = providerKey;
  
  const existingIndex = props.savedModels?.findIndex(m => m.provider === provider.name && m.model === modelName) ?? -1;
  
  let newSavedModels = [...(props.savedModels || [])];
  
  if (existingIndex >= 0) {
    // Remove
    newSavedModels.splice(existingIndex, 1);
  } else {
    // Add
    newSavedModels.push({
      id: `${provider.name}:${modelName}`,
      provider: provider.name,
      model: modelName,
      // Use provider default base, even if empty
      apiBase: provider.default_api_base,
      apiKey: providerKey,
      displayName: `${provider.display_name} - ${modelName}`
    });
  }
  
  emit('update-saved-models', newSavedModels);
  
  // Also log the toggle action
  console.log('[SettingsView] Toggled model:', modelName, 'Current Config:', {
      ...localConfig.value,
      apiKey: localConfig.value.apiKey ? `${localConfig.value.apiKey.substring(0, 8)}...` : 'undefined'
  });
};

const updateProviderKey = (key: string) => {
  if (!selectedProvider.value) return;
  const providerName = selectedProvider.value.name;
  providerApiKeys.value[providerName] = key;
  
  // Update local config immediately if this is the current provider
  localConfig.value.apiKey = key;
  
  // Update all saved models for this provider with new key
  if (props.savedModels) {
    const newModels = props.savedModels.map(m => {
      if (m.provider === providerName) {
        return { ...m, apiKey: key };
      }
      return m;
    });
    // Only emit if changed
    if (JSON.stringify(newModels) !== JSON.stringify(props.savedModels)) {
        emit('update-saved-models', newModels);
    }
  }
};

const toggleChannelEnabled = (channelName: string) => {
    if (channels.value[channelName]) {
        channels.value[channelName].enabled = !channels.value[channelName].enabled;
    }
};

const saveChannel = async (channelName: string) => {
    if (!channels.value[channelName]) return;
    
    try {
        await invoke('update_channel', {
            name: channelName,
            enabled: channels.value[channelName].enabled,
            config: channels.value[channelName]
        });
        console.log(`Saved channel ${channelName}`);
    } catch (e) {
        console.error(`Failed to save channel ${channelName}:`, e);
    }
};

const handleSave = () => {
  console.log('[SettingsView] Saving config:', {
      ...localConfig.value,
      apiKey: localConfig.value.apiKey ? `${localConfig.value.apiKey.substring(0, 8)}...` : 'undefined'
  });
  emit('save', localConfig.value);
};

// Also watch config to sync back changes if needed
watch(() => props.config, (newVal) => {
  localConfig.value = { ...newVal };
}, { deep: true });

</script>

<template>
  <div class="h-full flex flex-col bg-white rounded-xl overflow-hidden">
    <div class="p-6 border-b border-gray-100 flex justify-between items-center">
      <h2 class="text-xl font-bold text-gray-800 flex items-center">
        <span class="mr-2">⚙️</span> 设置
      </h2>
      <button 
        @click="handleSave" 
        class="px-6 py-2 bg-gradient-to-r from-pink-500 to-pink-600 hover:from-pink-400 hover:to-pink-500 text-white rounded-lg font-medium shadow-md shadow-pink-500/20 transition-all text-sm"
      >
        保存当前配置
      </button>
    </div>
    
    <div class="flex-1 flex overflow-hidden">
      <!-- Sidebar: Providers -->
      <div class="w-1/3 border-r border-gray-100 flex flex-col bg-gray-50/30">
        <div class="p-4 border-b border-gray-100 flex space-x-2">
          <button 
            @click="activeTab = 'providers'"
            class="flex-1 py-2 rounded-lg text-sm font-medium transition-all"
            :class="activeTab === 'providers' ? 'bg-pink-100 text-pink-700' : 'text-gray-500 hover:bg-gray-100'"
          >
            供应商
          </button>
          <button 
            @click="activeTab = 'channels'"
            class="flex-1 py-2 rounded-lg text-sm font-medium transition-all"
            :class="activeTab === 'channels' ? 'bg-pink-100 text-pink-700' : 'text-gray-500 hover:bg-gray-100'"
          >
            频道
          </button>
        </div>
        
        <div class="p-4 border-b border-gray-100" v-if="activeTab === 'providers'">
          <input 
            v-model="searchTerm" 
            placeholder="搜索供应商..." 
            class="w-full px-4 py-2 bg-white border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all"
          />
        </div>
        
        <div class="flex-1 overflow-y-auto p-2 space-y-1">
          <template v-if="activeTab === 'providers'">
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
                          {{ provider.is_local ? 'Local' : (provider.is_gateway ? 'Gateway' : 'Cloud') }}
                      </div>
                   </div>
                </div>
                
                <div v-if="savedModels?.some(m => m.provider === provider.name)" class="bg-pink-100 text-pink-600 text-[10px] font-bold px-2 py-0.5 rounded-full">
                    {{ savedModels?.filter(m => m.provider === provider.name).length }}
                </div>
              </button>
          </template>
          <template v-else>
             <button
                v-for="(config, name) in channels"
                :key="name"
                @click="selectedChannel = name"
                class="w-full text-left px-4 py-3 rounded-xl transition-all flex items-center justify-between group"
                :class="selectedChannel === name ? 'bg-white shadow-sm border-l-4 border-pink-500 text-pink-700' : 'hover:bg-gray-100 text-gray-600 border-l-4 border-transparent'"
             >
                <div class="flex items-center">
                   <div class="w-8 h-8 rounded-lg flex items-center justify-center mr-3" :class="selectedChannel === name ? 'bg-pink-100 text-pink-600' : 'bg-gray-200 text-gray-500'">
                      <MessageSquare :size="16" />
                   </div>
                   <div>
                      <div class="font-medium capitalize">{{ name }}</div>
                      <div class="text-[10px] uppercase tracking-wider opacity-70" :class="config.enabled ? 'text-green-600' : 'text-gray-400'">
                          {{ config.enabled ? 'Enabled' : 'Disabled' }}
                      </div>
                   </div>
                </div>
             </button>
          </template>
        </div>
      </div>

      <!-- Main Area -->
      <div class="flex-1 overflow-y-auto p-6 bg-white">
        <template v-if="activeTab === 'providers'">
            <div v-if="selectedProvider" class="space-y-8">
              <!-- Header -->
              <div class="flex items-center space-x-4">
                  <div class="w-12 h-12 bg-gradient-to-br from-pink-500 to-purple-600 rounded-xl flex items-center justify-center text-white shadow-lg shadow-pink-500/30">
                      <Server :size="24" />
                  </div>
                  <div>
                      <h3 class="text-xl font-bold text-gray-800">{{ selectedProvider.display_name }}</h3>
                      <p class="text-sm text-gray-500">{{ selectedProvider.default_api_base || '自定义 API 地址' }}</p>
                  </div>
              </div>

              <!-- Configuration -->
              <div class="space-y-4 p-4 bg-gray-50 rounded-xl border border-gray-100">
                 <h4 class="font-semibold text-gray-700 text-sm">连接配置</h4>
                 
                 <!-- API Key -->
                 <div class="space-y-1">
                   <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">API Key</label>
                   <input 
                     :value="providerApiKeys[selectedProvider.name]"
                     @input="e => updateProviderKey((e.target as HTMLInputElement).value)"
                     type="password" 
                     :placeholder="`Enter ${selectedProvider.display_name} API Key...`"
                     class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" 
                   />
                 </div>
                 
                 <!-- API Base -->
                 <div class="space-y-1">
                   <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">API Base URL</label>
                   <input 
                     disabled
                     :value="selectedProvider.default_api_base || 'Local/Custom'"
                     class="w-full px-3 py-2 bg-gray-100 border border-gray-200 rounded-lg text-gray-500 font-mono text-sm cursor-not-allowed" 
                   />
                   <!-- We disable editing base URL for predefined providers for simplicity in this view, 
                        can add custom provider support later -->
                 </div>
              </div>

              <!-- Model Selection -->
              <div class="space-y-4">
                <h4 class="font-semibold text-gray-700 text-sm flex items-center justify-between">
                    <span>可用模型</span>
                    <span class="text-xs font-normal text-gray-500">勾选以添加到快捷切换列表</span>
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
                    该供应商暂无预设模型
                </div>
              </div>

            </div>
            <div v-else class="h-full flex flex-col items-center justify-center text-gray-400 space-y-4">
              <Cpu :size="48" class="opacity-20" />
              <p>请从左侧选择一个供应商以配置模型</p>
            </div>
        </template>
        <template v-else>
            <div v-if="selectedChannel" class="space-y-8">
                <!-- Header -->
                <div class="flex items-center space-x-4">
                    <div class="w-12 h-12 bg-gradient-to-br from-pink-500 to-purple-600 rounded-xl flex items-center justify-center text-white shadow-lg shadow-pink-500/30">
                        <MessageSquare :size="24" />
                    </div>
                    <div>
                        <h3 class="text-xl font-bold text-gray-800 capitalize">{{ selectedChannel }}</h3>
                        <div class="flex items-center space-x-2 mt-1">
                            <span class="text-sm text-gray-500">状态:</span>
                            <button 
                                @click="toggleChannelEnabled(selectedChannel)"
                                class="px-2 py-0.5 rounded-full text-xs font-bold transition-colors"
                                :class="channels[selectedChannel].enabled ? 'bg-green-100 text-green-700' : 'bg-gray-200 text-gray-500'"
                            >
                                {{ channels[selectedChannel].enabled ? '已启用' : '已禁用' }}
                            </button>
                        </div>
                    </div>
                </div>

                <!-- Config Form -->
                <div class="space-y-4 p-4 bg-gray-50 rounded-xl border border-gray-100">
                    <!-- Telegram -->
                    <div v-if="selectedChannel === 'telegram'" class="space-y-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">Bot Token</label>
                            <input v-model="channels.telegram.token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="Enter Telegram Bot Token" />
                        </div>
                    </div>
                    <!-- Discord -->
                    <div v-else-if="selectedChannel === 'discord'" class="space-y-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">Bot Token</label>
                            <input v-model="channels.discord.token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="Enter Discord Bot Token" />
                        </div>
                    </div>
                    <!-- WhatsApp -->
                    <div v-else-if="selectedChannel === 'whatsapp'" class="space-y-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">Bridge URL</label>
                            <input v-model="channels.whatsapp.bridge_url" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                    </div>
                    <!-- Feishu -->
                    <div v-else-if="selectedChannel === 'feishu'" class="space-y-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">App ID</label>
                            <input v-model="channels.feishu.app_id" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">App Secret</label>
                            <input v-model="channels.feishu.app_secret" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">Verification Token</label>
                            <input v-model="channels.feishu.verification_token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                    </div>
                    <!-- DingTalk -->
                    <div v-else-if="selectedChannel === 'dingtalk'" class="space-y-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">Client ID</label>
                            <input v-model="channels.dingtalk.client_id" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">Client Secret</label>
                            <input v-model="channels.dingtalk.client_secret" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                    </div>
                    <!-- Email -->
                    <div v-else-if="selectedChannel === 'email'" class="space-y-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">IMAP Username</label>
                            <input v-model="channels.email.imap_username" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">IMAP Password</label>
                            <input v-model="channels.email.imap_password" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                    </div>
                    <!-- Slack -->
                    <div v-else-if="selectedChannel === 'slack'" class="space-y-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">Bot Token</label>
                            <input v-model="channels.slack.bot_token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">App Token</label>
                            <input v-model="channels.slack.app_token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                    </div>
                    <!-- QQ -->
                    <div v-else-if="selectedChannel === 'qq'" class="space-y-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">App ID</label>
                            <input v-model="channels.qq.app_id" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">App Secret</label>
                            <input v-model="channels.qq.secret" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                    </div>
                    <!-- Generic fallback -->
                    <div v-else class="text-sm text-gray-500">
                        配置项暂未完全支持 UI 编辑，请直接编辑配置文件。
                    </div>
                </div>

                <button 
                    @click="saveChannel(selectedChannel)"
                    class="w-full px-4 py-3 bg-pink-500 hover:bg-pink-600 text-white rounded-xl font-medium shadow-md shadow-pink-500/20 transition-all flex items-center justify-center space-x-2"
                >
                    <Save :size="18" />
                    <span>保存频道配置</span>
                </button>
            </div>
            <div v-else class="h-full flex flex-col items-center justify-center text-gray-400 space-y-4">
                <MessageSquare :size="48" class="opacity-20" />
                <p>请从左侧选择一个频道以进行配置</p>
            </div>
        </template>
      </div>
    </div>
  </div>
</template>

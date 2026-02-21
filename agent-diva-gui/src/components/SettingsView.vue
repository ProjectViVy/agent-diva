<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Server, Box, Cpu, Check, Trash2, MessageSquare, Save, Play, Globe } from 'lucide-vue-next';

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

// Internationalization
const lang = ref<'zh' | 'en'>('zh');
const t = computed(() => {
  return lang.value === 'zh' ? {
    settings: '设置',
    saveConfig: '保存当前配置',
    providers: '供应商',
    channels: '频道',
    searchProviders: '搜索供应商...',
    local: '本地',
    gateway: '网关',
    cloud: '云端',
    enabled: '已启用',
    disabled: '已禁用',
    connectConfig: '连接配置',
    apiKey: 'API Key',
    enterApiKey: '请输入 API Key...',
    apiBaseUrl: 'API Base URL',
    customApi: '自定义 API 地址',
    availableModels: '可用模型',
    checkToAdd: '勾选以添加到快捷切换列表',
    noModels: '该供应商暂无预设模型',
    selectProvider: '请从左侧选择一个供应商以配置模型',
    status: '状态',
    botToken: 'Bot Token',
    appId: 'App ID',
    appSecret: 'App Secret',
    verificationToken: 'Verification Token',
    clientId: 'Client ID',
    clientSecret: 'Client Secret',
    robotCode: '机器人代码',
    dmPolicy: '私聊策略',
    groupPolicy: '群聊策略',
    open: '开放',
    allowlist: '白名单',
    bridgeUrl: 'Bridge URL',
    saveChannel: '保存频道配置',
    testConnection: '测试连接',
    selectChannel: '请从左侧选择一个频道以进行配置',
    imapHost: 'IMAP Host',
    imapPort: 'IMAP Port',
    smtpHost: 'SMTP Host',
    smtpPort: 'SMTP Port',
    username: '用户名',
    password: '密码',
    useSsl: '使用 SSL/TLS',
    useTls: '使用 STARTTLS',
    fromAddress: '发件人地址',
    pollInterval: '轮询间隔 (秒)',
    subjectPrefix: '主题前缀',
    consentGranted: '已授权 (必须)',
    enableAutoReply: '启用自动回复',
    markSeen: '标记为已读',
    receiveSettings: '接收设置 (IMAP)',
    sendSettings: '发送设置 (SMTP)',
    behaviorSettings: '行为设置',
    testing: '测试中...',
    testSuccess: '连接成功',
    testFailed: '连接失败',
    saving: '保存中...',
    saved: '已保存',
    saveFailed: '保存失败',
  } : {
    settings: 'Settings',
    saveConfig: 'Save Config',
    providers: 'Providers',
    channels: 'Channels',
    searchProviders: 'Search Providers...',
    local: 'Local',
    gateway: 'Gateway',
    cloud: 'Cloud',
    enabled: 'Enabled',
    disabled: 'Disabled',
    connectConfig: 'Connection Config',
    apiKey: 'API Key',
    enterApiKey: 'Enter API Key...',
    apiBaseUrl: 'API Base URL',
    customApi: 'Custom API Address',
    availableModels: 'Available Models',
    checkToAdd: 'Check to add to shortcuts',
    noModels: 'No models available for this provider',
    selectProvider: 'Select a provider from the left to configure models',
    status: 'Status',
    botToken: 'Bot Token',
    appId: 'App ID',
    appSecret: 'App Secret',
    verificationToken: 'Verification Token',
    clientId: 'Client ID',
    clientSecret: 'Client Secret',
    robotCode: 'Robot Code',
    dmPolicy: 'DM Policy',
    groupPolicy: 'Group Policy',
    open: 'Open',
    allowlist: 'Allowlist',
    bridgeUrl: 'Bridge URL',
    saveChannel: 'Save Channel Config',
    testConnection: 'Test Connection',
    selectChannel: 'Select a channel from the left to configure',
    imapHost: 'IMAP Host',
    imapPort: 'IMAP Port',
    smtpHost: 'SMTP Host',
    smtpPort: 'SMTP Port',
    username: 'Username',
    password: 'Password',
    useSsl: 'Use SSL/TLS',
    useTls: 'Use STARTTLS',
    fromAddress: 'From Address',
    pollInterval: 'Poll Interval (s)',
    subjectPrefix: 'Subject Prefix',
    consentGranted: 'Consent Granted (Required)',
    enableAutoReply: 'Enable Auto Reply',
    markSeen: 'Mark as Seen',
    receiveSettings: 'Receive Settings (IMAP)',
    sendSettings: 'Send Settings (SMTP)',
    behaviorSettings: 'Behavior Settings',
    testing: 'Testing...',
    testSuccess: 'Connected',
    testFailed: 'Connection Failed',
    saving: 'Saving...',
    saved: 'Saved',
    saveFailed: 'Save Failed',
  };
});

const testStatus = ref<'idle' | 'testing' | 'success' | 'failed'>('idle');
const testMessage = ref('');
const saveStatus = ref<'idle' | 'saving' | 'success' | 'failed'>('idle');

onMounted(async () => {
  try {
    providers.value = await invoke('get_providers');
    try {
        channels.value = await invoke('get_channels');
    } catch (e) {
        console.error('Failed to load channels:', e);
    }
    
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

const toggleChannelEnabled = (channelName: string) => {
    if (channels.value[channelName]) {
        channels.value[channelName].enabled = !channels.value[channelName].enabled;
    }
};

const testConnection = async (channelName: string) => {
    if (!channels.value[channelName]) return;
    
    testStatus.value = 'testing';
    testMessage.value = '';
    
    try {
        await invoke('test_channel', {
            name: channelName,
            config: channels.value[channelName]
        });
        
        testStatus.value = 'success';
    } catch (e: any) {
        testStatus.value = 'failed';
        testMessage.value = e.message || String(e);
    }
    
    // Reset status after 3 seconds
    setTimeout(() => {
        if (testStatus.value !== 'testing') {
            testStatus.value = 'idle';
        }
    }, 3000);
};

const saveChannel = async (channelName: string) => {
    if (!channels.value[channelName]) return;
    
    saveStatus.value = 'saving';
    
    try {
        await invoke('update_channel', {
            name: channelName,
            enabled: channels.value[channelName].enabled,
            config: channels.value[channelName]
        });
        
        saveStatus.value = 'success';
        console.log(`Saved channel ${channelName}`);
    } catch (e) {
        saveStatus.value = 'failed';
        console.error(`Failed to save channel ${channelName}:`, e);
    }
    
    setTimeout(() => {
        saveStatus.value = 'idle';
    }, 2000);
};

const handleSave = () => {
  emit('save', localConfig.value);
};

watch(() => props.config, (newVal) => {
  localConfig.value = { ...newVal };
}, { deep: true });

const toggleLang = () => {
    lang.value = lang.value === 'zh' ? 'en' : 'zh';
};
</script>

<template>
  <div class="h-full flex flex-col bg-white rounded-xl overflow-hidden min-w-[320px]">
    <div class="p-6 border-b border-gray-100 flex justify-between items-center">
      <h2 class="text-xl font-bold text-gray-800 flex items-center">
        <span class="mr-2">⚙️</span> {{ t.settings }}
      </h2>
      <div class="flex items-center space-x-2">
          <button @click="toggleLang" class="p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-lg transition-colors" title="Switch Language">
              <Globe :size="20" />
          </button>
          <button 
            @click="handleSave" 
            class="px-6 py-2 bg-gradient-to-r from-pink-500 to-pink-600 hover:from-pink-400 hover:to-pink-500 text-white rounded-lg font-medium shadow-md shadow-pink-500/20 transition-all text-sm"
          >
            {{ t.saveConfig }}
          </button>
      </div>
    </div>
    
    <div class="flex-1 flex overflow-hidden">
      <!-- Sidebar: Providers -->
      <div class="w-1/3 min-w-[200px] border-r border-gray-100 flex flex-col bg-gray-50/30">
        <div class="p-4 border-b border-gray-100 flex space-x-2">
          <button 
            @click="activeTab = 'providers'"
            class="flex-1 py-2 rounded-lg text-sm font-medium transition-all"
            :class="activeTab === 'providers' ? 'bg-pink-100 text-pink-700' : 'text-gray-500 hover:bg-gray-100'"
          >
            {{ t.providers }}
          </button>
          <button 
            @click="activeTab = 'channels'"
            class="flex-1 py-2 rounded-lg text-sm font-medium transition-all"
            :class="activeTab === 'channels' ? 'bg-pink-100 text-pink-700' : 'text-gray-500 hover:bg-gray-100'"
          >
            {{ t.channels }}
          </button>
        </div>
        
        <div class="p-4 border-b border-gray-100" v-if="activeTab === 'providers'">
          <input 
            v-model="searchTerm" 
            :placeholder="t.searchProviders" 
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
                          {{ provider.is_local ? t.local : (provider.is_gateway ? t.gateway : t.cloud) }}
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
                          {{ config.enabled ? t.enabled : t.disabled }}
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
                      <p class="text-sm text-gray-500">{{ selectedProvider.default_api_base || t.customApi }}</p>
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
                            <span class="text-sm text-gray-500">{{ t.status }}:</span>
                            <button 
                                @click="toggleChannelEnabled(selectedChannel)"
                                class="px-2 py-0.5 rounded-full text-xs font-bold transition-colors"
                                :class="channels[selectedChannel].enabled ? 'bg-green-100 text-green-700' : 'bg-gray-200 text-gray-500'"
                            >
                                {{ channels[selectedChannel].enabled ? t.enabled : t.disabled }}
                            </button>
                        </div>
                    </div>
                </div>

                <!-- Config Form -->
                <div class="space-y-4 p-4 bg-gray-50 rounded-xl border border-gray-100">
                    <!-- Telegram -->
                    <div v-if="selectedChannel === 'telegram'" class="space-y-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.botToken }}</label>
                            <input v-model="channels.telegram.token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="Enter Telegram Bot Token" />
                        </div>
                    </div>
                    <!-- Discord -->
                    <div v-else-if="selectedChannel === 'discord'" class="space-y-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.botToken }}</label>
                            <input v-model="channels.discord.token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="Enter Discord Bot Token" />
                        </div>
                    </div>
                    <!-- WhatsApp -->
                    <div v-else-if="selectedChannel === 'whatsapp'" class="space-y-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.bridgeUrl }}</label>
                            <input v-model="channels.whatsapp.bridge_url" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                    </div>
                    <!-- Feishu -->
                    <div v-else-if="selectedChannel === 'feishu'" class="space-y-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.appId }}</label>
                            <input v-model="channels.feishu.app_id" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.appSecret }}</label>
                            <input v-model="channels.feishu.app_secret" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.verificationToken }}</label>
                            <input v-model="channels.feishu.verification_token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                    </div>
                    <!-- DingTalk -->
                    <div v-else-if="selectedChannel === 'dingtalk'" class="space-y-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.clientId }}</label>
                            <input v-model="channels.dingtalk.client_id" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.clientSecret }}</label>
                            <input v-model="channels.dingtalk.client_secret" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.robotCode }}</label>
                            <input v-model="channels.dingtalk.robot_code" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="Optional" />
                        </div>
                        <div class="grid grid-cols-2 gap-4">
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.dmPolicy }}</label>
                                <select v-model="channels.dingtalk.dm_policy" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all text-sm">
                                    <option value="open">{{ t.open }}</option>
                                    <option value="allowlist">{{ t.allowlist }}</option>
                                </select>
                            </div>
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.groupPolicy }}</label>
                                <select v-model="channels.dingtalk.group_policy" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all text-sm">
                                    <option value="open">{{ t.open }}</option>
                                    <option value="allowlist">{{ t.allowlist }}</option>
                                </select>
                            </div>
                        </div>
                    </div>
                    <!-- Email -->
                    <div v-else-if="selectedChannel === 'email'" class="space-y-6">
                        <!-- IMAP Settings -->
                        <div class="space-y-4">
                            <h4 class="font-semibold text-gray-700 text-sm border-b pb-2">{{ t.receiveSettings }}</h4>
                            <div class="grid grid-cols-2 gap-4">
                                <div class="space-y-1">
                                    <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.imapHost }}</label>
                                    <input v-model="channels.email.imap_host" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="imap.example.com" />
                                </div>
                                <div class="space-y-1">
                                    <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.imapPort }}</label>
                                    <input v-model.number="channels.email.imap_port" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                                </div>
                            </div>
                            <div class="grid grid-cols-2 gap-4">
                                <div class="space-y-1">
                                    <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.username }}</label>
                                    <input v-model="channels.email.imap_username" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                                </div>
                                <div class="space-y-1">
                                    <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.password }}</label>
                                    <input v-model="channels.email.imap_password" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                                </div>
                            </div>
                            <div class="flex items-center space-x-2">
                                <input type="checkbox" v-model="channels.email.imap_use_ssl" id="imap_ssl" class="rounded text-pink-500 focus:ring-pink-500" />
                                <label for="imap_ssl" class="text-sm text-gray-600">{{ t.useSsl }}</label>
                            </div>
                        </div>

                        <!-- SMTP Settings -->
                        <div class="space-y-4">
                            <h4 class="font-semibold text-gray-700 text-sm border-b pb-2">{{ t.sendSettings }}</h4>
                            <div class="grid grid-cols-2 gap-4">
                                <div class="space-y-1">
                                    <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.smtpHost }}</label>
                                    <input v-model="channels.email.smtp_host" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="smtp.example.com" />
                                </div>
                                <div class="space-y-1">
                                    <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.smtpPort }}</label>
                                    <input v-model.number="channels.email.smtp_port" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                                </div>
                            </div>
                            <div class="grid grid-cols-2 gap-4">
                                <div class="space-y-1">
                                    <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.username }}</label>
                                    <input v-model="channels.email.smtp_username" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="Leave empty if same as IMAP" />
                                </div>
                                <div class="space-y-1">
                                    <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.password }}</label>
                                    <input v-model="channels.email.smtp_password" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="Leave empty if same as IMAP" />
                                </div>
                            </div>
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.fromAddress }}</label>
                                <input v-model="channels.email.from_address" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="Optional: Sender address" />
                            </div>
                            <div class="flex space-x-6">
                                <div class="flex items-center space-x-2">
                                    <input type="checkbox" v-model="channels.email.smtp_use_tls" id="smtp_tls" class="rounded text-pink-500 focus:ring-pink-500" />
                                    <label for="smtp_tls" class="text-sm text-gray-600">{{ t.useTls }}</label>
                                </div>
                                <div class="flex items-center space-x-2">
                                    <input type="checkbox" v-model="channels.email.smtp_use_ssl" id="smtp_ssl" class="rounded text-pink-500 focus:ring-pink-500" />
                                    <label for="smtp_ssl" class="text-sm text-gray-600">{{ t.useSsl }}</label>
                                </div>
                            </div>
                        </div>

                        <!-- Behavior Settings -->
                        <div class="space-y-4">
                            <h4 class="font-semibold text-gray-700 text-sm border-b pb-2">{{ t.behaviorSettings }}</h4>
                            <div class="grid grid-cols-2 gap-4">
                                <div class="space-y-1">
                                    <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.pollInterval }}</label>
                                    <input v-model.number="channels.email.poll_interval_seconds" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                                </div>
                                <div class="space-y-1">
                                    <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.subjectPrefix }}</label>
                                    <input v-model="channels.email.subject_prefix" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                                </div>
                            </div>
                            <div class="space-y-2">
                                <div class="flex items-center space-x-2">
                                    <input type="checkbox" v-model="channels.email.consent_granted" id="consent_granted" class="rounded text-pink-500 focus:ring-pink-500" />
                                    <label for="consent_granted" class="text-sm text-gray-600 font-medium">{{ t.consentGranted }}</label>
                                </div>
                                <p class="text-xs text-gray-400 ml-6">I confirm that I have explicit permission to access and send emails from this account.</p>
                                
                                <div class="flex items-center space-x-2 mt-2">
                                    <input type="checkbox" v-model="channels.email.auto_reply_enabled" id="auto_reply" class="rounded text-pink-500 focus:ring-pink-500" />
                                    <label for="auto_reply" class="text-sm text-gray-600">{{ t.enableAutoReply }}</label>
                                </div>
                                
                                <div class="flex items-center space-x-2">
                                    <input type="checkbox" v-model="channels.email.mark_seen" id="mark_seen" class="rounded text-pink-500 focus:ring-pink-500" />
                                    <label for="mark_seen" class="text-sm text-gray-600">{{ t.markSeen }}</label>
                                </div>
                            </div>
                        </div>
                    </div>
                    <!-- Slack -->
                    <div v-else-if="selectedChannel === 'slack'" class="space-y-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.botToken }}</label>
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
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.appId }}</label>
                            <input v-model="channels.qq.app_id" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t.appSecret }}</label>
                            <input v-model="channels.qq.secret" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                    </div>
                    <!-- Generic fallback -->
                    <div v-else class="text-sm text-gray-500">
                        配置项暂未完全支持 UI 编辑，请直接编辑配置文件。
                    </div>
                </div>

                <div class="flex space-x-3">
                    <button 
                        @click="testConnection(selectedChannel)"
                        class="flex-1 px-4 py-3 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-xl font-medium transition-all flex items-center justify-center space-x-2"
                        :disabled="testStatus === 'testing'"
                    >
                        <div v-if="testStatus === 'testing'" class="animate-spin rounded-full h-4 w-4 border-2 border-gray-500 border-t-transparent"></div>
                        <Play v-else :size="18" />
                        <span>{{ testStatus === 'testing' ? t.testing : t.testConnection }}</span>
                    </button>
                    
                    <button 
                        @click="saveChannel(selectedChannel)"
                        class="flex-[2] px-4 py-3 bg-pink-500 hover:bg-pink-600 text-white rounded-xl font-medium shadow-md shadow-pink-500/20 transition-all flex items-center justify-center space-x-2"
                        :disabled="saveStatus === 'saving'"
                    >
                        <div v-if="saveStatus === 'saving'" class="animate-spin rounded-full h-4 w-4 border-2 border-white border-t-transparent"></div>
                        <Save v-else :size="18" />
                        <span>{{ saveStatus === 'saving' ? t.saving : t.saveChannel }}</span>
                    </button>
                </div>
                
                <!-- Feedback messages -->
                <div v-if="testStatus === 'success'" class="p-3 bg-green-100 text-green-700 rounded-lg text-sm flex items-center">
                    <Check :size="16" class="mr-2" />
                    {{ t.testSuccess }}
                </div>
                <div v-if="testStatus === 'failed'" class="p-3 bg-red-100 text-red-700 rounded-lg text-sm">
                    {{ t.testFailed }}: {{ testMessage }}
                </div>
                
            </div>
            <div v-else class="h-full flex flex-col items-center justify-center text-gray-400 space-y-4">
                <MessageSquare :size="48" class="opacity-20" />
                <p>{{ t.selectChannel }}</p>
            </div>
        </template>
      </div>
    </div>
  </div>
</template>

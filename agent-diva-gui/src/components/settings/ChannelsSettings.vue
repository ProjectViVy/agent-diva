<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { MessageSquare, Play, Save, Check } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const props = defineProps<{
  // lang: 'zh' | 'en'; // Removed
}>();

const channels = ref<any>({});
const selectedChannel = ref<string | null>(null);
const testStatus = ref<'idle' | 'testing' | 'success' | 'failed'>('idle');
const testMessage = ref('');
const saveStatus = ref<'idle' | 'saving' | 'success' | 'failed'>('idle');

onMounted(async () => {
  try {
    channels.value = await invoke('get_channels');
  } catch (e) {
    console.error('Failed to load channels:', e);
  }
});

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
</script>

<template>
  <div class="flex h-full fade-in">
    <!-- Sidebar: List of Channels -->
    <div class="w-1/3 min-w-[200px] border-r border-gray-100 flex flex-col bg-gray-50/30">
      <div class="flex-1 overflow-y-auto p-2 space-y-1">
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
                      {{ config.enabled ? t('channels.enabled') : t('channels.disabled') }}
                  </div>
               </div>
            </div>
         </button>
      </div>
    </div>

    <!-- Main Area -->
    <div class="flex-1 overflow-y-auto p-6 bg-white">
        <div v-if="selectedChannel" class="space-y-8">
            <!-- Header -->
            <div class="flex items-center space-x-4">
                <div class="w-12 h-12 bg-gradient-to-br from-pink-500 to-purple-600 rounded-xl flex items-center justify-center text-white shadow-lg shadow-pink-500/30">
                    <MessageSquare :size="24" />
                </div>
                <div>
                    <h3 class="text-xl font-bold text-gray-800 capitalize">{{ selectedChannel }}</h3>
                    <div class="flex items-center space-x-2 mt-1">
                        <span class="text-sm text-gray-500">{{ t('channels.status') }}:</span>
                        <button 
                            @click="toggleChannelEnabled(selectedChannel)"
                            class="px-2 py-0.5 rounded-full text-xs font-bold transition-colors"
                            :class="channels[selectedChannel].enabled ? 'bg-green-100 text-green-700' : 'bg-gray-200 text-gray-500'"
                        >
                            {{ channels[selectedChannel].enabled ? t('channels.enabled') : t('channels.disabled') }}
                        </button>
                    </div>
                </div>
            </div>

            <!-- Config Form -->
            <div class="space-y-4 p-4 bg-gray-50 rounded-xl border border-gray-100">
                <!-- Telegram -->
                <div v-if="selectedChannel === 'telegram'" class="space-y-4">
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.botToken') }}</label>
                        <input v-model="channels.telegram.token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="Enter Telegram Bot Token" />
                    </div>
                </div>
                <!-- Discord -->
                <div v-else-if="selectedChannel === 'discord'" class="space-y-4">
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.botToken') }}</label>
                        <input v-model="channels.discord.token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="Enter Discord Bot Token" />
                    </div>
                </div>
                <!-- WhatsApp -->
                <div v-else-if="selectedChannel === 'whatsapp'" class="space-y-4">
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.bridgeUrl') }}</label>
                        <input v-model="channels.whatsapp.bridge_url" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                    </div>
                </div>
                <!-- Feishu -->
                <div v-else-if="selectedChannel === 'feishu'" class="space-y-4">
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.appId') }}</label>
                        <input v-model="channels.feishu.app_id" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                    </div>
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.appSecret') }}</label>
                        <input v-model="channels.feishu.app_secret" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                    </div>
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.verificationToken') }}</label>
                        <input v-model="channels.feishu.verification_token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                    </div>
                </div>
                <!-- DingTalk -->
                <div v-else-if="selectedChannel === 'dingtalk'" class="space-y-4">
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.clientId') }}</label>
                        <input v-model="channels.dingtalk.client_id" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                    </div>
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.clientSecret') }}</label>
                        <input v-model="channels.dingtalk.client_secret" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                    </div>
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.robotCode') }}</label>
                        <input v-model="channels.dingtalk.robot_code" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="Optional" />
                    </div>
                    <div class="grid grid-cols-2 gap-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.dmPolicy') }}</label>
                            <select v-model="channels.dingtalk.dm_policy" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all text-sm">
                                <option value="open">{{ t('channels.open') }}</option>
                                <option value="allowlist">{{ t('channels.allowlist') }}</option>
                            </select>
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.groupPolicy') }}</label>
                            <select v-model="channels.dingtalk.group_policy" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all text-sm">
                                <option value="open">{{ t('channels.open') }}</option>
                                <option value="allowlist">{{ t('channels.allowlist') }}</option>
                            </select>
                        </div>
                    </div>
                </div>
                <!-- Email -->
                <div v-else-if="selectedChannel === 'email'" class="space-y-6">
                    <!-- IMAP Settings -->
                    <div class="space-y-4">
                        <h4 class="font-semibold text-gray-700 text-sm border-b pb-2">{{ t('channels.receiveSettings') }}</h4>
                        <div class="grid grid-cols-2 gap-4">
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.imapHost') }}</label>
                                <input v-model="channels.email.imap_host" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="imap.example.com" />
                            </div>
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.imapPort') }}</label>
                                <input v-model.number="channels.email.imap_port" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                            </div>
                        </div>
                        <div class="grid grid-cols-2 gap-4">
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.username') }}</label>
                                <input v-model="channels.email.imap_username" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                            </div>
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.password') }}</label>
                                <input v-model="channels.email.imap_password" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                            </div>
                        </div>
                        <div class="flex items-center space-x-2">
                            <input type="checkbox" v-model="channels.email.imap_use_ssl" id="imap_ssl" class="rounded text-pink-500 focus:ring-pink-500" />
                            <label for="imap_ssl" class="text-sm text-gray-600">{{ t('channels.useSsl') }}</label>
                        </div>
                    </div>

                    <!-- SMTP Settings -->
                    <div class="space-y-4">
                        <h4 class="font-semibold text-gray-700 text-sm border-b pb-2">{{ t('channels.sendSettings') }}</h4>
                        <div class="grid grid-cols-2 gap-4">
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.smtpHost') }}</label>
                                <input v-model="channels.email.smtp_host" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="smtp.example.com" />
                            </div>
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.smtpPort') }}</label>
                                <input v-model.number="channels.email.smtp_port" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                            </div>
                        </div>
                        <div class="grid grid-cols-2 gap-4">
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.username') }}</label>
                                <input v-model="channels.email.smtp_username" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="Leave empty if same as IMAP" />
                            </div>
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.password') }}</label>
                                <input v-model="channels.email.smtp_password" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="Leave empty if same as IMAP" />
                            </div>
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.fromAddress') }}</label>
                            <input v-model="channels.email.from_address" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="Optional: Sender address" />
                        </div>
                        <div class="flex space-x-6">
                            <div class="flex items-center space-x-2">
                                <input type="checkbox" v-model="channels.email.smtp_use_tls" id="smtp_tls" class="rounded text-pink-500 focus:ring-pink-500" />
                                <label for="smtp_tls" class="text-sm text-gray-600">{{ t('channels.useTls') }}</label>
                            </div>
                            <div class="flex items-center space-x-2">
                                <input type="checkbox" v-model="channels.email.smtp_use_ssl" id="smtp_ssl" class="rounded text-pink-500 focus:ring-pink-500" />
                                <label for="smtp_ssl" class="text-sm text-gray-600">{{ t('channels.useSsl') }}</label>
                            </div>
                        </div>
                    </div>

                    <!-- Behavior Settings -->
                    <div class="space-y-4">
                        <h4 class="font-semibold text-gray-700 text-sm border-b pb-2">{{ t('channels.behaviorSettings') }}</h4>
                        <div class="grid grid-cols-2 gap-4">
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.pollInterval') }}</label>
                                <input v-model.number="channels.email.poll_interval_seconds" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                            </div>
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.subjectPrefix') }}</label>
                                <input v-model="channels.email.subject_prefix" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                            </div>
                        </div>
                        <div class="space-y-2">
                            <div class="flex items-center space-x-2">
                                <input type="checkbox" v-model="channels.email.consent_granted" id="consent_granted" class="rounded text-pink-500 focus:ring-pink-500" />
                                <label for="consent_granted" class="text-sm text-gray-600 font-medium">{{ t('channels.consentGranted') }}</label>
                            </div>
                            <p class="text-xs text-gray-400 ml-6">I confirm that I have explicit permission to access and send emails from this account.</p>
                            
                            <div class="flex items-center space-x-2 mt-2">
                                <input type="checkbox" v-model="channels.email.auto_reply_enabled" id="auto_reply" class="rounded text-pink-500 focus:ring-pink-500" />
                                <label for="auto_reply" class="text-sm text-gray-600">{{ t('channels.enableAutoReply') }}</label>
                            </div>
                            
                            <div class="flex items-center space-x-2">
                                <input type="checkbox" v-model="channels.email.mark_seen" id="mark_seen" class="rounded text-pink-500 focus:ring-pink-500" />
                                <label for="mark_seen" class="text-sm text-gray-600">{{ t('channels.markSeen') }}</label>
                            </div>
                        </div>
                    </div>
                </div>
                <!-- Slack -->
                <div v-else-if="selectedChannel === 'slack'" class="space-y-4">
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.botToken') }}</label>
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
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.appId') }}</label>
                        <input v-model="channels.qq.app_id" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                    </div>
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.appSecret') }}</label>
                        <input v-model="channels.qq.secret" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                    </div>
                </div>
                <!-- Generic fallback -->
                <div v-else class="text-sm text-gray-500">
                    {{ t('providers.unsupportedUI') }}
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
                    <span>{{ testStatus === 'testing' ? t('channels.testing') : t('channels.testConnection') }}</span>
                </button>
                
                <button 
                    @click="saveChannel(selectedChannel)"
                    class="flex-[2] px-4 py-3 bg-pink-500 hover:bg-pink-600 text-white rounded-xl font-medium shadow-md shadow-pink-500/20 transition-all flex items-center justify-center space-x-2"
                    :disabled="saveStatus === 'saving'"
                >
                    <div v-if="saveStatus === 'saving'" class="animate-spin rounded-full h-4 w-4 border-2 border-white border-t-transparent"></div>
                    <Save v-else :size="18" />
                    <span>{{ saveStatus === 'saving' ? t('channels.saving') : t('channels.saveChannel') }}</span>
                </button>
            </div>
            
            <!-- Feedback messages -->
            <div v-if="testStatus === 'success'" class="p-3 bg-green-100 text-green-700 rounded-lg text-sm flex items-center">
                <Check :size="16" class="mr-2" />
                {{ t('channels.testSuccess') }}
            </div>
            <div v-if="testStatus === 'failed'" class="p-3 bg-red-100 text-red-700 rounded-lg text-sm">
                {{ t('channels.testFailed') }}: {{ testMessage }}
            </div>
            
        </div>
        <div v-else class="h-full flex flex-col items-center justify-center text-gray-400 space-y-4">
            <MessageSquare :size="48" class="opacity-20" />
            <p>{{ t('channels.selectChannel') }}</p>
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

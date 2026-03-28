<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { MessageSquare } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { getConfigStatus, type ChannelStatusSummary } from '../../api/desktop';

const { t } = useI18n();

const channels = ref<Record<string, any>>({});
const channelStatuses = ref<ChannelStatusSummary[]>([]);
const selectedChannel = ref<string | null>(null);
const isInitializing = ref(true);
let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
let lastSavedSnapshot = '';

onMounted(async () => {
  try {
    channels.value = await invoke('get_channels');
    channelStatuses.value = (await getConfigStatus()).channels;
  } catch (e) {
    console.error('Failed to load channels:', e);
  } finally {
    isInitializing.value = false;
  }
});

const channelStatusMap = computed(() => {
  return new Map(channelStatuses.value.map((item) => [item.name, item]));
});

const toggleChannelEnabled = (channelName: string) => {
  if (channels.value[channelName]) {
    channels.value[channelName].enabled = !channels.value[channelName].enabled;
  }
};

const scheduleAutoSave = (channelName: string) => {
    if (isInitializing.value || !channels.value[channelName]) return;

    if (autosaveTimer) {
        clearTimeout(autosaveTimer);
    }

    autosaveTimer = setTimeout(async () => {
        const config = channels.value[channelName];
        if (!config) return;
        const snapshot = JSON.stringify(config);
        if (snapshot === lastSavedSnapshot) return;

        try {
            await invoke('update_channel', {
                name: channelName,
                enabled: config.enabled,
                config
            });
            lastSavedSnapshot = snapshot;
        } catch (e) {
            console.error(`Failed to auto-save channel ${channelName}:`, e);
        }
    }, 300);
};

watch(selectedChannel, (channelName) => {
    if (!channelName || !channels.value[channelName]) return;
    lastSavedSnapshot = JSON.stringify(channels.value[channelName]);
});

watch(
    () => (selectedChannel.value ? channels.value[selectedChannel.value] : null),
    () => {
        if (!selectedChannel.value) return;
        scheduleAutoSave(selectedChannel.value);
    },
    { deep: true }
);
</script>

<template>
  <div class="flex h-full min-h-0 fade-in">
    <!-- Sidebar: List of Channels -->
    <div class="w-1/3 min-w-[200px] min-h-0 border-r border-gray-100 flex flex-col bg-gray-50/30">
      <div class="flex-1 overflow-y-auto p-2 space-y-1">
         <div
            v-for="(config, name) in channels"
            :key="name"
            @click="selectedChannel = name"
            @keydown.enter="selectedChannel = name"
            @keydown.space.prevent="selectedChannel = name"
            role="button"
            tabindex="0"
            class="w-full text-left px-4 py-3 rounded-xl transition-all flex items-center justify-between group"
            :class="selectedChannel === name ? 'bg-white shadow-sm border-l-4 border-pink-500 text-pink-700' : 'hover:bg-gray-100 text-gray-600 border-l-4 border-transparent'"
         >
            <div class="flex min-w-0 items-center">
               <div class="w-8 h-8 rounded-lg flex items-center justify-center mr-3 shrink-0" :class="selectedChannel === name ? 'bg-pink-100 text-pink-600' : 'bg-gray-200 text-gray-500'">
                  <MessageSquare :size="16" />
               </div>
               <div class="min-w-0">
                  <div class="flex items-center gap-2">
                    <div class="font-medium capitalize truncate">{{ name }}</div>
                    <button
                      type="button"
                      class="shrink-0 rounded-full px-2.5 py-1 text-[11px] font-semibold transition-colors"
                      :class="
                        config.enabled
                          ? 'bg-gray-200 text-gray-700 hover:bg-gray-300'
                          : 'bg-pink-100 text-pink-700 hover:bg-pink-200'
                      "
                      @click.stop="toggleChannelEnabled(name)"
                    >
                      {{ config.enabled ? t('channels.deactivate') : t('channels.activate') }}
                    </button>
                  </div>
                  <div class="text-[10px] uppercase tracking-wider opacity-70" :class="config.enabled ? 'text-green-600' : 'text-gray-400'">
                      {{ !config.enabled ? t('channels.disabled') : (channelStatusMap.get(name)?.ready ? t('channels.ready') : t('channels.needsSetup')) }}
                  </div>
               </div>
            </div>
         </div>
      </div>
    </div>

    <!-- Main Area -->
    <div class="flex-1 min-h-0 overflow-y-auto p-6 bg-white">
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
                        <span
                            class="px-2 py-0.5 rounded-full text-xs font-bold"
                            :class="channels[selectedChannel].enabled ? 'bg-green-100 text-green-700' : 'bg-gray-200 text-gray-500'"
                        >
                            {{ channels[selectedChannel].enabled ? t('channels.enabled') : t('channels.disabled') }}
                        </span>
                    </div>
                </div>
            </div>

            <!-- Config Form -->
            <div class="space-y-4 p-4 bg-gray-50 rounded-xl border border-gray-100">
                <div v-if="channelStatusMap.get(selectedChannel)" class="grid grid-cols-1 md:grid-cols-2 gap-3">
                    <div class="rounded-lg bg-white border border-gray-200 px-3 py-2">
                        <div class="text-[11px] uppercase tracking-wider text-gray-400">{{ t('channels.readiness') }}</div>
                        <div class="mt-1 text-sm font-semibold" :class="channelStatusMap.get(selectedChannel)?.ready ? 'text-emerald-700' : 'text-amber-700'">
                            {{ channelStatusMap.get(selectedChannel)?.ready ? t('channels.ready') : t('channels.needsSetup') }}
                        </div>
                    </div>
                    <div class="rounded-lg bg-white border border-gray-200 px-3 py-2">
                        <div class="text-[11px] uppercase tracking-wider text-gray-400">{{ t('channels.missingFields') }}</div>
                        <div class="mt-1 text-xs text-gray-600">
                            {{ channelStatusMap.get(selectedChannel)?.missing_fields.length ? channelStatusMap.get(selectedChannel)?.missing_fields.join(', ') : t('channels.none') }}
                        </div>
                    </div>
                </div>
                <!-- Telegram -->
                <div v-if="selectedChannel === 'telegram'" class="space-y-4">
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.botToken') }}</label>
                        <input v-model="channels.telegram.token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.telegramToken')" />
                    </div>
                </div>
                <!-- Discord -->
                <div v-else-if="selectedChannel === 'discord'" class="space-y-4">
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.botToken') }}</label>
                        <input v-model="channels.discord.token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.discordToken')" />
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
                        <input v-model="channels.dingtalk.robot_code" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.optional')" />
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
                                <input v-model="channels.email.imap_host" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.imapHost')" />
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
                                <input v-model="channels.email.smtp_host" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.smtpHost')" />
                            </div>
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.smtpPort') }}</label>
                                <input v-model.number="channels.email.smtp_port" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                            </div>
                        </div>
                        <div class="grid grid-cols-2 gap-4">
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.username') }}</label>
                                <input v-model="channels.email.smtp_username" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.smtpSameAsImap')" />
                            </div>
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.password') }}</label>
                                <input v-model="channels.email.smtp_password" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.smtpSameAsImap')" />
                            </div>
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.fromAddress') }}</label>
                            <input v-model="channels.email.from_address" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.senderAddress')" />
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
                            <p class="text-xs text-gray-400 ml-6">{{ t('channels.emailConsentHint') }}</p>
                            
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
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.appToken') }}</label>
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
                <!-- Neuro-Link -->
                <div v-else-if="selectedChannel === 'neuro-link' || selectedChannel === 'generic_pipe'" class="space-y-4">
                    <div class="grid grid-cols-2 gap-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.listenHost') }}</label>
                            <input v-model="channels[selectedChannel].host" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="0.0.0.0" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.listenPort') }}</label>
                            <input v-model.number="channels[selectedChannel].port" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="9100" />
                        </div>
                    </div>
                </div>
                <!-- IRC -->
                <div v-else-if="selectedChannel === 'irc'" class="space-y-6">
                    <!-- Connection Settings -->
                    <div class="space-y-4">
                        <h4 class="font-semibold text-gray-700 text-sm border-b pb-2">{{ t('channels.connectionSettings') }}</h4>
                        <div class="grid grid-cols-2 gap-4">
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.server') }}</label>
                                <input v-model="channels.irc.server" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="irc.libera.chat" />
                            </div>
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.port') }}</label>
                                <input v-model.number="channels.irc.port" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                            </div>
                        </div>
                        <div class="grid grid-cols-2 gap-4">
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.nickname') }}</label>
                                <input v-model="channels.irc.nickname" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="diva-bot" />
                            </div>
                            <div class="space-y-1">
                                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.username') }}</label>
                                <input v-model="channels.irc.username" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.optional')" />
                            </div>
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.ircChannels') }}</label>
                            <input v-model="channels.irc.channels_str" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="#channel1, #channel2" />
                        </div>
                        <div class="flex space-x-6">
                            <div class="flex items-center space-x-2">
                                <input type="checkbox" v-model="channels.irc.use_tls" id="irc_tls" class="rounded text-pink-500 focus:ring-pink-500" />
                                <label for="irc_tls" class="text-sm text-gray-600">{{ t('channels.useSsl') }}</label>
                            </div>
                            <div class="flex items-center space-x-2">
                                <input type="checkbox" v-model="channels.irc.verify_tls" id="irc_verify_tls" class="rounded text-pink-500 focus:ring-pink-500" />
                                <label for="irc_verify_tls" class="text-sm text-gray-600">{{ t('channels.verifyTls') }}</label>
                            </div>
                        </div>
                    </div>
                    <!-- Authentication -->
                    <div class="space-y-4">
                        <h4 class="font-semibold text-gray-700 text-sm border-b pb-2">{{ t('channels.authSettings') }}</h4>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.serverPassword') }}</label>
                            <input v-model="channels.irc.server_password" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.optional')" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.nickservPassword') }}</label>
                            <input v-model="channels.irc.nickserv_password" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.optional')" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.saslPassword') }}</label>
                            <input v-model="channels.irc.sasl_password" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.optional')" />
                        </div>
                    </div>
                </div>
                <!-- Mattermost -->
                <div v-else-if="selectedChannel === 'mattermost'" class="space-y-4">
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.baseUrl') }}</label>
                        <input v-model="channels.mattermost.base_url" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="https://mattermost.example.com" />
                    </div>
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.botToken') }}</label>
                        <input v-model="channels.mattermost.bot_token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                    </div>
                    <div class="grid grid-cols-2 gap-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.channelId') }}</label>
                            <input v-model="channels.mattermost.channel_id" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.pollInterval') }}</label>
                            <input v-model.number="channels.mattermost.poll_interval_seconds" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                    </div>
                    <div class="flex space-x-6">
                        <div class="flex items-center space-x-2">
                            <input type="checkbox" v-model="channels.mattermost.thread_replies" id="mm_thread" class="rounded text-pink-500 focus:ring-pink-500" />
                            <label for="mm_thread" class="text-sm text-gray-600">{{ t('channels.threadReplies') }}</label>
                        </div>
                        <div class="flex items-center space-x-2">
                            <input type="checkbox" v-model="channels.mattermost.mention_only" id="mm_mention" class="rounded text-pink-500 focus:ring-pink-500" />
                            <label for="mm_mention" class="text-sm text-gray-600">{{ t('channels.mentionOnly') }}</label>
                        </div>
                    </div>
                </div>
                <!-- Nextcloud Talk -->
                <div v-else-if="selectedChannel === 'nextcloud_talk'" class="space-y-4">
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.baseUrl') }}</label>
                        <input v-model="channels.nextcloud_talk.base_url" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="https://cloud.example.com" />
                    </div>
                    <div class="space-y-1">
                        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.appToken') }}</label>
                        <input v-model="channels.nextcloud_talk.app_token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                    </div>
                    <div class="grid grid-cols-2 gap-4">
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.roomToken') }}</label>
                            <input v-model="channels.nextcloud_talk.room_token" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                        <div class="space-y-1">
                            <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.pollInterval') }}</label>
                            <input v-model.number="channels.nextcloud_talk.poll_interval_seconds" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                        </div>
                    </div>
                </div>
                <!-- Generic fallback -->
                <div v-else class="text-sm text-gray-500">
                    {{ t('providers.unsupportedUI') }}
                </div>
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

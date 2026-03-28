<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { LoaderCircle, MessageSquare } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { getConfigStatus, type ChannelStatusSummary } from '../../api/desktop';

const { t } = useI18n();

const props = defineProps<{
  saveChannelConfigAction: (channelName: string, channelConfig: Record<string, unknown>) => Promise<void>;
}>();

const draftChannels = ref<Record<string, any>>({});
const savedChannels = ref<Record<string, any>>({});
const channelStatuses = ref<ChannelStatusSummary[]>([]);
const selectedChannel = ref<string | null>(null);
const isInitializing = ref(true);
const isSaving = ref(false);

const cloneValue = <T>(value: T): T => JSON.parse(JSON.stringify(value));

async function loadChannels() {
  try {
    const fetchedChannels = await invoke<Record<string, any>>('get_channels');
    draftChannels.value = cloneValue(fetchedChannels);
    savedChannels.value = cloneValue(fetchedChannels);
    channelStatuses.value = (await getConfigStatus()).channels;
    if (!selectedChannel.value || !draftChannels.value[selectedChannel.value]) {
      selectedChannel.value = Object.keys(draftChannels.value)[0] ?? null;
    }
  } catch (e) {
    console.error('Failed to load channels:', e);
  } finally {
    isInitializing.value = false;
  }
}

onMounted(async () => {
  await loadChannels();
});

const channelStatusMap = computed(() => {
  return new Map(channelStatuses.value.map((item) => [item.name, item]));
});

const selectedChannelDraft = computed(() => {
  if (!selectedChannel.value) return null;
  return draftChannels.value[selectedChannel.value] ?? null;
});

const isDirty = computed(() => {
  if (!selectedChannel.value || !selectedChannelDraft.value) return false;
  return JSON.stringify(selectedChannelDraft.value) !== JSON.stringify(savedChannels.value[selectedChannel.value] ?? null);
});

const toggleChannelEnabled = (channelName: string) => {
  if (draftChannels.value[channelName]) {
    draftChannels.value[channelName].enabled = !draftChannels.value[channelName].enabled;
  }
};

const saveCurrentChannel = async () => {
  if (!selectedChannel.value || !selectedChannelDraft.value || isSaving.value || !isDirty.value) return;
  isSaving.value = true;
  try {
    const nextConfig = cloneValue(selectedChannelDraft.value);
    await props.saveChannelConfigAction(selectedChannel.value, nextConfig);
    draftChannels.value[selectedChannel.value] = cloneValue(nextConfig);
    savedChannels.value[selectedChannel.value] = cloneValue(nextConfig);
    channelStatuses.value = (await getConfigStatus()).channels;
  } finally {
    isSaving.value = false;
  }
};
</script>

<template>
  <div class="flex h-full min-h-0 fade-in">
    <div class="w-1/3 min-w-[200px] min-h-0 border-r border-gray-100 flex flex-col bg-gray-50/30">
      <div class="flex-1 overflow-y-auto p-2 space-y-1">
         <div
            v-for="(config, name) in draftChannels"
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

    <div class="flex-1 min-h-0 overflow-y-auto p-6 bg-white">
      <div v-if="selectedChannel && selectedChannelDraft" class="space-y-8">
        <div class="flex items-start justify-between gap-4">
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
                  :class="selectedChannelDraft.enabled ? 'bg-green-100 text-green-700' : 'bg-gray-200 text-gray-500'"
                >
                  {{ selectedChannelDraft.enabled ? t('channels.enabled') : t('channels.disabled') }}
                </span>
              </div>
            </div>
          </div>
          <button
            type="button"
            class="inline-flex min-w-[112px] items-center justify-center gap-2 rounded-lg bg-emerald-500 px-4 py-2 text-sm font-semibold text-white transition hover:bg-emerald-600 disabled:cursor-not-allowed disabled:bg-emerald-300"
            :disabled="isInitializing || isSaving || !isDirty"
            @click="saveCurrentChannel"
          >
            <LoaderCircle v-if="isSaving" :size="16" class="animate-spin" />
            <span>{{ isSaving ? t('console.saving') : t('console.saveConfig') }}</span>
          </button>
        </div>

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

          <div v-if="selectedChannel === 'telegram'" class="space-y-4">
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.botToken') }}</label>
              <input v-model="draftChannels.telegram.token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.telegramToken')" />
            </div>
          </div>

          <div v-else-if="selectedChannel === 'discord'" class="space-y-4">
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.botToken') }}</label>
              <input v-model="draftChannels.discord.token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.discordToken')" />
            </div>
          </div>

          <div v-else-if="selectedChannel === 'whatsapp'" class="space-y-4">
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.bridgeUrl') }}</label>
              <input v-model="draftChannels.whatsapp.bridge_url" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
            </div>
          </div>

          <div v-else-if="selectedChannel === 'feishu'" class="space-y-4">
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.appId') }}</label>
              <input v-model="draftChannels.feishu.app_id" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
            </div>
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.appSecret') }}</label>
              <input v-model="draftChannels.feishu.app_secret" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
            </div>
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.verificationToken') }}</label>
              <input v-model="draftChannels.feishu.verification_token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
            </div>
          </div>

          <div v-else-if="selectedChannel === 'dingtalk'" class="space-y-4">
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.clientId') }}</label>
              <input v-model="draftChannels.dingtalk.client_id" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
            </div>
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.clientSecret') }}</label>
              <input v-model="draftChannels.dingtalk.client_secret" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
            </div>
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.robotCode') }}</label>
              <input v-model="draftChannels.dingtalk.robot_code" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.optional')" />
            </div>
            <div class="grid grid-cols-2 gap-4">
              <div class="space-y-1">
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.dmPolicy') }}</label>
                <select v-model="draftChannels.dingtalk.dm_policy" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all text-sm">
                  <option value="open">{{ t('channels.open') }}</option>
                  <option value="allowlist">{{ t('channels.allowlist') }}</option>
                </select>
              </div>
              <div class="space-y-1">
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.groupPolicy') }}</label>
                <select v-model="draftChannels.dingtalk.group_policy" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all text-sm">
                  <option value="open">{{ t('channels.open') }}</option>
                  <option value="allowlist">{{ t('channels.allowlist') }}</option>
                </select>
              </div>
            </div>
          </div>

          <div v-else-if="selectedChannel === 'email'" class="space-y-6">
            <div class="space-y-4">
              <h4 class="font-semibold text-gray-700 text-sm border-b pb-2">{{ t('channels.receiveSettings') }}</h4>
              <div class="grid grid-cols-2 gap-4">
                <div class="space-y-1">
                  <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.imapHost') }}</label>
                  <input v-model="draftChannels.email.imap_host" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.imapHost')" />
                </div>
                <div class="space-y-1">
                  <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.imapPort') }}</label>
                  <input v-model.number="draftChannels.email.imap_port" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                </div>
              </div>
              <div class="grid grid-cols-2 gap-4">
                <div class="space-y-1">
                  <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.username') }}</label>
                  <input v-model="draftChannels.email.imap_username" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                </div>
                <div class="space-y-1">
                  <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.password') }}</label>
                  <input v-model="draftChannels.email.imap_password" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                </div>
              </div>
              <div class="flex items-center space-x-2">
                <input type="checkbox" v-model="draftChannels.email.imap_use_ssl" id="imap_ssl" class="rounded text-pink-500 focus:ring-pink-500" />
                <label for="imap_ssl" class="text-sm text-gray-600">{{ t('channels.useSsl') }}</label>
              </div>
            </div>
            <div class="space-y-4">
              <h4 class="font-semibold text-gray-700 text-sm border-b pb-2">{{ t('channels.sendSettings') }}</h4>
              <div class="grid grid-cols-2 gap-4">
                <div class="space-y-1">
                  <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.smtpHost') }}</label>
                  <input v-model="draftChannels.email.smtp_host" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.smtpHost')" />
                </div>
                <div class="space-y-1">
                  <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.smtpPort') }}</label>
                  <input v-model.number="draftChannels.email.smtp_port" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                </div>
              </div>
              <div class="grid grid-cols-2 gap-4">
                <div class="space-y-1">
                  <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.username') }}</label>
                  <input v-model="draftChannels.email.smtp_username" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                </div>
                <div class="space-y-1">
                  <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.password') }}</label>
                  <input v-model="draftChannels.email.smtp_password" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                </div>
              </div>
              <div class="grid grid-cols-2 gap-4">
                <div class="flex items-center space-x-2">
                  <input type="checkbox" v-model="draftChannels.email.smtp_use_ssl" id="smtp_ssl" class="rounded text-pink-500 focus:ring-pink-500" />
                  <label for="smtp_ssl" class="text-sm text-gray-600">{{ t('channels.useSsl') }}</label>
                </div>
                <div class="flex items-center space-x-2">
                  <input type="checkbox" v-model="draftChannels.email.smtp_use_tls" id="smtp_tls" class="rounded text-pink-500 focus:ring-pink-500" />
                  <label for="smtp_tls" class="text-sm text-gray-600">{{ t('channels.useTls') }}</label>
                </div>
              </div>
              <div class="space-y-1">
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.fromAddress') }}</label>
                <input v-model="draftChannels.email.from_address" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.senderAddress')" />
              </div>
            </div>
            <div class="space-y-4">
              <h4 class="font-semibold text-gray-700 text-sm border-b pb-2">{{ t('channels.behaviorSettings') }}</h4>
              <div class="grid grid-cols-2 gap-4">
                <div class="space-y-1">
                  <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.pollInterval') }}</label>
                  <input v-model.number="draftChannels.email.poll_interval_seconds" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                </div>
                <div class="space-y-1">
                  <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.subjectPrefix') }}</label>
                  <input v-model="draftChannels.email.subject_prefix" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                </div>
              </div>
              <div class="flex flex-col gap-3">
                <label class="text-sm text-gray-600 flex items-center space-x-2">
                  <input type="checkbox" v-model="draftChannels.email.consent_granted" />
                  <span>{{ t('channels.consentGranted') }}</span>
                </label>
                <p class="text-xs text-amber-600">{{ t('channels.emailConsentHint') }}</p>
                <label class="text-sm text-gray-600 flex items-center space-x-2">
                  <input type="checkbox" v-model="draftChannels.email.enable_auto_reply" />
                  <span>{{ t('channels.enableAutoReply') }}</span>
                </label>
                <label class="text-sm text-gray-600 flex items-center space-x-2">
                  <input type="checkbox" v-model="draftChannels.email.mark_seen" />
                  <span>{{ t('channels.markSeen') }}</span>
                </label>
              </div>
            </div>
          </div>

          <div v-else-if="selectedChannel === 'slack'" class="space-y-4">
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.botToken') }}</label>
              <input v-model="draftChannels.slack.bot_token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
            </div>
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.appToken') }}</label>
              <input v-model="draftChannels.slack.app_token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
            </div>
          </div>

          <div v-else-if="selectedChannel === 'qq'" class="space-y-4">
            <div class="grid grid-cols-2 gap-4">
              <div class="space-y-1">
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.appId') }}</label>
                <input v-model.number="draftChannels.qq.app_id" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
              </div>
              <div class="space-y-1">
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.listenPort') }}</label>
                <input v-model.number="draftChannels.qq.listen_port" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
              </div>
            </div>
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.listenHost') }}</label>
              <input v-model="draftChannels.qq.listen_host" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
            </div>
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.verificationToken') }}</label>
              <input v-model="draftChannels.qq.verification_token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
            </div>
          </div>

          <div v-else-if="selectedChannel === 'irc'" class="space-y-6">
            <div class="space-y-4">
              <h4 class="font-semibold text-gray-700 text-sm border-b pb-2">{{ t('channels.connectionSettings') }}</h4>
              <div class="grid grid-cols-2 gap-4">
                <div class="space-y-1">
                  <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.server') }}</label>
                  <input v-model="draftChannels.irc.server" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="irc.libera.chat" />
                </div>
                <div class="space-y-1">
                  <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.port') }}</label>
                  <input v-model.number="draftChannels.irc.port" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
                </div>
              </div>
              <div class="grid grid-cols-2 gap-4">
                <div class="space-y-1">
                  <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.nickname') }}</label>
                  <input v-model="draftChannels.irc.nickname" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="diva-bot" />
                </div>
                <div class="space-y-1">
                  <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.username') }}</label>
                  <input v-model="draftChannels.irc.username" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.optional')" />
                </div>
              </div>
              <div class="space-y-1">
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.ircChannels') }}</label>
                <input v-model="draftChannels.irc.channels_str" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="#channel1, #channel2" />
              </div>
              <div class="flex space-x-6">
                <div class="flex items-center space-x-2">
                  <input type="checkbox" v-model="draftChannels.irc.use_tls" id="irc_tls" class="rounded text-pink-500 focus:ring-pink-500" />
                  <label for="irc_tls" class="text-sm text-gray-600">{{ t('channels.useSsl') }}</label>
                </div>
                <div class="flex items-center space-x-2">
                  <input type="checkbox" v-model="draftChannels.irc.verify_tls" id="irc_verify_tls" class="rounded text-pink-500 focus:ring-pink-500" />
                  <label for="irc_verify_tls" class="text-sm text-gray-600">{{ t('channels.verifyTls') }}</label>
                </div>
              </div>
            </div>
            <div class="space-y-4">
              <h4 class="font-semibold text-gray-700 text-sm border-b pb-2">{{ t('channels.authSettings') }}</h4>
              <div class="space-y-1">
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.serverPassword') }}</label>
                <input v-model="draftChannels.irc.server_password" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.optional')" />
              </div>
              <div class="space-y-1">
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.nickservPassword') }}</label>
                <input v-model="draftChannels.irc.nickserv_password" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.optional')" />
              </div>
              <div class="space-y-1">
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.saslPassword') }}</label>
                <input v-model="draftChannels.irc.sasl_password" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" :placeholder="t('channels.placeholders.optional')" />
              </div>
            </div>
          </div>

          <div v-else-if="selectedChannel === 'mattermost'" class="space-y-4">
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.baseUrl') }}</label>
              <input v-model="draftChannels.mattermost.base_url" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="https://mattermost.example.com" />
            </div>
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.botToken') }}</label>
              <input v-model="draftChannels.mattermost.bot_token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
            </div>
            <div class="grid grid-cols-2 gap-4">
              <div class="space-y-1">
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.channelId') }}</label>
                <input v-model="draftChannels.mattermost.channel_id" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
              </div>
              <div class="space-y-1">
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.pollInterval') }}</label>
                <input v-model.number="draftChannels.mattermost.poll_interval_seconds" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
              </div>
            </div>
            <div class="flex space-x-6">
              <div class="flex items-center space-x-2">
                <input type="checkbox" v-model="draftChannels.mattermost.thread_replies" id="mm_thread" class="rounded text-pink-500 focus:ring-pink-500" />
                <label for="mm_thread" class="text-sm text-gray-600">{{ t('channels.threadReplies') }}</label>
              </div>
              <div class="flex items-center space-x-2">
                <input type="checkbox" v-model="draftChannels.mattermost.mention_only" id="mm_mention" class="rounded text-pink-500 focus:ring-pink-500" />
                <label for="mm_mention" class="text-sm text-gray-600">{{ t('channels.mentionOnly') }}</label>
              </div>
            </div>
          </div>

          <div v-else-if="selectedChannel === 'nextcloud_talk'" class="space-y-4">
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.baseUrl') }}</label>
              <input v-model="draftChannels.nextcloud_talk.base_url" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" placeholder="https://cloud.example.com" />
            </div>
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.appToken') }}</label>
              <input v-model="draftChannels.nextcloud_talk.app_token" type="password" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
            </div>
            <div class="grid grid-cols-2 gap-4">
              <div class="space-y-1">
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.roomToken') }}</label>
                <input v-model="draftChannels.nextcloud_talk.room_token" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
              </div>
              <div class="space-y-1">
                <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('channels.pollInterval') }}</label>
                <input v-model.number="draftChannels.nextcloud_talk.poll_interval_seconds" type="number" class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" />
              </div>
            </div>
          </div>

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

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { LoaderCircle, MessageSquare, LayoutGrid, List, Plus, RefreshCw } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import { getConfigStatus, type ChannelStatusSummary } from '../../api/desktop';
import ChannelCardView from './ChannelCardView.vue';
import ChannelEditorForm from './ChannelEditorForm.vue';
import ChannelWizardModal from './ChannelWizardModal.vue';
import { isRetiredChannel } from './channel-platforms';
import { normalizeChannelConfig } from './channel-wizard-fields';

const { t } = useI18n();

const props = defineProps<{
  saveChannelConfigAction: (channelName: string, channelConfig: Record<string, unknown>) => Promise<void>;
}>();

// 视图模式：'card' | 'list'
const viewMode = ref<'card' | 'list'>('card');
const wizardOpen = ref(false);
const editingChannel = ref<string | null>(null);
const isLoading = ref(false);

const draftChannels = ref<Record<string, any>>({});
const savedChannels = ref<Record<string, any>>({});
const channelStatuses = ref<ChannelStatusSummary[]>([]);
const selectedChannel = ref<string | null>(null);
const isInitializing = ref(true);
const isSaving = ref(false);

const cloneValue = <T>(value: T): T => JSON.parse(JSON.stringify(value));

function normalizeDiscordConfig(d: Record<string, unknown> | undefined) {
  if (!d || typeof d !== 'object') return;
  if (!Array.isArray(d.allow_from)) d.allow_from = [];
  if (d.gateway_url === undefined || d.gateway_url === '') {
    d.gateway_url = 'wss://gateway.discord.gg/?v=10&encoding=json';
  }
  if (d.intents === undefined || d.intents === null) d.intents = 37377;
  if (d.guild_id === undefined) d.guild_id = null;
  if (d.mention_only === undefined) d.mention_only = false;
  if (d.listen_to_bots === undefined) d.listen_to_bots = false;
  if (!Array.isArray(d.group_reply_allowed_sender_ids)) d.group_reply_allowed_sender_ids = [];
}

async function loadChannels() {
  isLoading.value = true;
  try {
    const fetchedChannels = await invoke<Record<string, any>>('get_channels');
    normalizeDiscordConfig(fetchedChannels.discord);
    draftChannels.value = cloneValue(fetchedChannels);
    savedChannels.value = cloneValue(fetchedChannels);
    channelStatuses.value = (await getConfigStatus()).channels;
    if (!selectedChannel.value || !draftChannels.value[selectedChannel.value] || isRetiredChannel(selectedChannel.value)) {
      selectedChannel.value = Object.keys(draftChannels.value).find((name) => !isRetiredChannel(name)) ?? null;
    }
  } catch (e) {
    console.error('Failed to load channels:', e);
  } finally {
    isInitializing.value = false;
    isLoading.value = false;
  }
}

onMounted(async () => {
  await loadChannels();
});

const channelStatusMap = computed(() => {
  return new Map(channelStatuses.value.map((item) => [item.name, item]));
});

const visibleDraftChannels = computed(() => {
  const next: Record<string, any> = {};
  for (const [name, config] of Object.entries(draftChannels.value)) {
    if (!isRetiredChannel(name)) next[name] = config;
  }
  return next;
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

const persistChannel = async (name: string, config: Record<string, any>) => {
  if (isSaving.value) return;
  isSaving.value = true;
  try {
    await props.saveChannelConfigAction(name, cloneValue(config));
    draftChannels.value[name] = cloneValue(config);
    savedChannels.value[name] = cloneValue(config);
    channelStatuses.value = (await getConfigStatus()).channels;
  } finally {
    isSaving.value = false;
  }
};

const saveCurrentChannel = async () => {
  if (!selectedChannel.value || !selectedChannelDraft.value || isSaving.value || !isDirty.value) return;
  await persistChannel(selectedChannel.value, selectedChannelDraft.value);
};

// 向导相关函数
const openWizard = () => {
  editingChannel.value = null;
  wizardOpen.value = true;
};

const handleWizardTest = async (_data: any) => {
  // TODO: 实现真实连接测试逻辑
  return { success: false, message: t('channels.testNotImplemented') };
};

const handleWizardComplete = async (data: { platform: string; credentials?: Record<string, unknown> }) => {
  try {
    const existing = draftChannels.value[data.platform] ?? {};
    const credentials = normalizeChannelConfig(data.platform, data.credentials ?? {});
    delete credentials.enabled;
    const enabled = editingChannel.value ? Boolean(existing.enabled) : true;
    await props.saveChannelConfigAction(data.platform, {
      ...cloneValue(existing),
      ...credentials,
      enabled,
    });
    await loadChannels();
  } catch (e) {
    console.error('Failed to save channel from wizard:', e);
  }
};

const handleCardEdit = (name: string) => {
  editingChannel.value = name;
  selectedChannel.value = name;
  wizardOpen.value = true;
};

const handleCardDelete = async (name: string) => {
  const confirmed = window.confirm(t('channels.deleteConfirm', { name }));
  if (!confirmed) return;
  // TODO: 调用后端删除 API
  window.alert(t('channels.deleteNotImplemented'));
};

const handleCardToggle = async (name: string) => {
  toggleChannelEnabled(name);
  await persistChannel(name, draftChannels.value[name]);
};

const handleRefresh = async () => {
  await loadChannels();
};
</script>

<template>
  <div class="flex h-full min-h-0 fade-in">
    <!-- 列表视图侧边栏 (仅在列表模式显示) -->
    <div v-if="viewMode === 'list'" class="channels-sidebar">
      <div class="channels-list">
         <div
            v-for="(config, name) in visibleDraftChannels"
            :key="name"
            @click="selectedChannel = name"
            @keydown.enter="selectedChannel = name"
            @keydown.space.prevent="selectedChannel = name"
            role="button"
            tabindex="0"
            class="channels-item"
            :class="{ selected: selectedChannel === name }"
         >
            <div class="channels-item-icon" :class="{ selected: selectedChannel === name }">
               <MessageSquare :size="16" />
            </div>
            <div class="min-w-0">
               <div class="channels-item-name">{{ name }}</div>
               <div class="channels-item-status" :class="{ enabled: config.enabled }">
                   {{ !config.enabled ? t('channels.disabled') : (channelStatusMap.get(name)?.ready ? t('channels.ready') : t('channels.needsSetup')) }}
               </div>
            </div>
         </div>
      </div>
    </div>

    <!-- 主内容区 -->
    <div class="channels-content-wrapper">
      <!-- 顶部工具栏 -->
      <div class="channels-toolbar">
        <div class="flex items-center gap-2">
          <button
            class="toolbar-btn"
            @click="handleRefresh"
            :disabled="isLoading"
            :title="t('topbar.refresh')"
          >
            <RefreshCw :size="16" :class="{ 'animate-spin': isLoading }" />
          </button>
          <div class="toolbar-divider" />
          <button
            v-if="viewMode === 'list'"
            class="toolbar-btn"
            @click="viewMode = 'card'"
            :title="t('channels.cardView')"
          >
            <LayoutGrid :size="16" />
          </button>
          <button
            v-if="viewMode === 'card'"
            class="toolbar-btn"
            @click="viewMode = 'list'"
            :title="t('channels.listView')"
          >
            <List :size="16" />
          </button>
        </div>
        <div class="flex items-center gap-2">
          <button class="btn-primary" @click="openWizard">
            <Plus :size="16" />
            {{ t('channels.addChannel') }}
          </button>
        </div>
      </div>

      <!-- 卡片视图 -->
      <div v-if="viewMode === 'card'" class="channels-main-content">
        <ChannelCardView
          :channels="visibleDraftChannels"
          :statuses="channelStatuses"
          :loading="isInitializing"
          @add="openWizard"
          @edit="handleCardEdit"
          @delete="handleCardDelete"
          @toggle="handleCardToggle"
        />
      </div>

      <!-- 列表视图（详细配置） -->
      <div v-else class="channels-content">
        <div v-if="selectedChannel && selectedChannelDraft" class="channels-detail">
          <div class="channels-detail-header">
            <div class="flex items-center space-x-4 min-w-0">
              <div class="channels-header-icon">
                <MessageSquare :size="24" />
              </div>
              <div class="min-w-0">
                <h3 class="channels-header-title capitalize">{{ selectedChannel }}</h3>
                <div class="flex flex-wrap items-center gap-2 mt-1">
                  <span class="settings-muted text-sm">{{ t('channels.status') }}:</span>
                  <span
                    class="channels-status-badge"
                    :class="selectedChannelDraft.enabled ? 'enabled' : 'disabled'"
                  >
                    {{ selectedChannelDraft.enabled ? t('channels.enabled') : t('channels.disabled') }}
                  </span>
                  <button
                    type="button"
                    role="switch"
                    :aria-checked="selectedChannelDraft.enabled"
                    :aria-label="selectedChannelDraft.enabled ? t('channels.enabled') : t('channels.disabled')"
                    :title="selectedChannelDraft.enabled ? t('channels.enabled') : t('channels.disabled')"
                    class="channels-toggle"
                    :class="{ enabled: selectedChannelDraft.enabled }"
                    @click.stop="toggleChannelEnabled(selectedChannel)"
                  >
                    <span class="channels-toggle-thumb" />
                  </button>
                </div>
              </div>
            </div>
            <button
              type="button"
              class="btn-save-config"
              :disabled="isInitializing || isSaving || !isDirty"
              @click="saveCurrentChannel"
            >
              <LoaderCircle v-if="isSaving" :size="16" class="animate-spin" />
              <span>{{ isSaving ? t('console.saving') : t('console.saveConfig') }}</span>
            </button>
          </div>

          <div class="channels-detail-body">
            <div v-if="channelStatusMap.get(selectedChannel)" class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div class="channels-status-card">
                <div class="channels-status-card-label">{{ t('channels.readiness') }}</div>
                <div class="channels-status-card-value" :class="channelStatusMap.get(selectedChannel)?.ready ? 'ready' : 'warning'">
                  {{ channelStatusMap.get(selectedChannel)?.ready ? t('channels.ready') : t('channels.needsSetup') }}
                </div>
              </div>
              <div class="channels-status-card">
                <div class="channels-status-card-label">{{ t('channels.missingFields') }}</div>
                <div class="mt-1 text-xs" style="color: var(--text);">
                  {{ channelStatusMap.get(selectedChannel)?.missing_fields.length ? channelStatusMap.get(selectedChannel)?.missing_fields.join(', ') : t('channels.none') }}
                </div>
              </div>
            </div>

            <ChannelEditorForm
              :platform="selectedChannel"
              :config="selectedChannelDraft"
            />
          </div>
        </div>

        <div v-else class="channels-empty">
          <MessageSquare :size="48" class="opacity-20" />
          <p>{{ t('channels.selectChannel') }}</p>
        </div>
      </div>
    </div>

    <!-- 配置向导模态框 -->
    <ChannelWizardModal
      v-model:open="wizardOpen"
      :initial-data="
        editingChannel
          ? { platform: editingChannel, credentials: { ...(draftChannels[editingChannel] ?? {}) } }
          : undefined
      "
      @test="handleWizardTest"
      @complete="handleWizardComplete"
    />
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

/* 内容包装器 */
.channels-content-wrapper {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.channels-sidebar {
  width: 220px;
  flex-shrink: 0;
  height: 100%;
}

.channels-detail {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  min-width: 0;
}

.channels-detail-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}

.channels-detail-body {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1rem;
  border-radius: var(--radius);
  border: 1px solid var(--line);
  background: var(--accent-bg-light);
  min-width: 0;
}

/* 工具栏 */
.channels-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem 1.5rem;
  border-bottom: 1px solid var(--line);
  background: var(--accent-bg-light);
}

.toolbar-btn {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  border: 1px solid var(--line);
  background: var(--panel);
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.toolbar-btn:hover:not(:disabled) {
  background: var(--accent-bg-light);
  color: var(--accent);
  border-color: var(--accent);
}

.toolbar-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.toolbar-divider {
  width: 1px;
  height: 24px;
  background: var(--line);
  margin: 0 0.25rem;
}

/* 主内容区 */
.channels-main-content {
  flex: 1;
  overflow-y: auto;
}

.channels-empty-wizard {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.75rem;
}

/* 按钮样式 */
.btn-primary,
.btn-secondary {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-primary {
  background: var(--accent);
  color: white;
  border: none;
}

.btn-primary:hover {
  filter: brightness(1.1);
}

.btn-secondary {
  background: var(--panel);
  color: var(--text);
  border: 1px solid var(--line);
}

.btn-secondary:hover {
  background: var(--accent-bg-light);
}
</style>

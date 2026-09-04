<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { LoaderCircle, MessageSquare, LayoutGrid, List, Plus, RefreshCw } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import { appConfirmAsync } from '../../utils/appDialog';
import { errorMessage } from '../../utils/errorMessage';
import { getConfigStatus, type ChannelStatusSummary } from '../../api/desktop';
import ChannelCardView from './ChannelCardView.vue';
import ChannelEditorForm from './ChannelEditorForm.vue';
import ChannelWizardModal from './ChannelWizardModal.vue';
import { CHANNEL_PLATFORMS } from './channel-platforms';
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
const removedChannels = ref<Set<string>>(new Set());
const channelStatuses = ref<ChannelStatusSummary[]>([]);
const selectedChannel = ref<string | null>(null);
const isInitializing = ref(true);
const busyChannels = ref<Set<string>>(new Set());
const channelErrors = ref<Record<string, string>>({});
const loadError = ref<string | null>(null);
let loadGeneration = 0;
let channelMutationGeneration = 0;
const channelMutationVersions = new Map<string, number>();

const cloneValue = <T>(value: T): T => JSON.parse(JSON.stringify(value));

interface ChannelRuntimeStatus {
  name: string;
  registered: boolean;
  lifecycle: 'starting' | 'running' | 'degraded' | 'down';
  health: 'healthy' | 'degraded' | 'down' | 'unknown';
  diagnosis?: string | null;
}

interface ChannelWizardData {
  platform: string;
  name: string;
  credentials: Record<string, unknown>;
  extra?: Record<string, unknown>;
}

interface ParsedChannelsResponse {
  channels: Record<string, Record<string, any>>;
  removed: Set<string>;
}

function isRecord(value: unknown): value is Record<string, any> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function parseRemovedNames(value: unknown): Set<string> {
  if (value === undefined) return new Set();
  if (Array.isArray(value)) {
    if (value.some((name) => typeof name !== 'string' || name.length === 0)) {
      throw new Error(t('channels.invalidResponse'));
    }
    return new Set(value as string[]);
  }
  if (typeof value === 'string' && value.length > 0) return new Set([value]);
  if (isRecord(value)) return new Set(Object.keys(value));
  throw new Error(t('channels.invalidResponse'));
}

function parseChannelsResponse(value: unknown): ParsedChannelsResponse {
  if (!isRecord(value)) throw new Error(t('channels.invalidResponse'));
  const raw = value;
  const hasNestedChannels = Object.prototype.hasOwnProperty.call(raw, 'channels');
  if (hasNestedChannels && !isRecord(raw.channels)) throw new Error(t('channels.invalidResponse'));
  const source = hasNestedChannels ? raw.channels : raw;
  const channels: Record<string, Record<string, any>> = {};
  for (const [name, config] of Object.entries(source)) {
    if (name === 'channels' || name === 'removed') continue;
    if (!isRecord(config)) throw new Error(t('channels.invalidResponse'));
    channels[name] = config;
  }
  return { channels, removed: parseRemovedNames(raw.removed) };
}

function parseRuntimeStatuses(value: unknown): ChannelRuntimeStatus[] {
  if (Array.isArray(value)) return value as ChannelRuntimeStatus[];
  if (isRecord(value) && Array.isArray(value.channels)) {
    return value.channels as ChannelRuntimeStatus[];
  }
  return [];
}

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

async function loadChannels(): Promise<boolean> {
  const requestGeneration = ++loadGeneration;
  const mutationSnapshot = new Map(channelMutationVersions);
  isLoading.value = true;
  loadError.value = null;
  try {
    const [fetchedChannels, configStatus, rawRuntimeStatuses] = await Promise.all([
      invoke<unknown>('get_channels'),
      getConfigStatus(),
      invoke<unknown>('get_channel_runtime'),
    ]);
    if (requestGeneration !== loadGeneration) return false;
    const parsed = parseChannelsResponse(fetchedChannels);
    normalizeDiscordConfig(parsed.channels.discord);
    const remoteChannels = cloneValue(parsed.channels);
    const remoteSavedChannels = cloneValue(parsed.channels);
    const nextChannels = cloneValue(remoteChannels);
    const nextSavedChannels = cloneValue(remoteSavedChannels);
    for (const [name, draft] of Object.entries(draftChannels.value)) {
      const mutationChanged = (channelMutationVersions.get(name) ?? 0) !== (mutationSnapshot.get(name) ?? 0);
      const draftIsDirty = JSON.stringify(draft) !== JSON.stringify(savedChannels.value[name] ?? null);
      if (mutationChanged) {
        nextChannels[name] = cloneValue(draft);
        if (savedChannels.value[name] === undefined) delete nextSavedChannels[name];
        else nextSavedChannels[name] = cloneValue(savedChannels.value[name]);
      } else if (isChannelBusy(name) || draftIsDirty) {
        nextChannels[name] = cloneValue(draft);
      }
    }
    const nextRemovedChannels = new Set(parsed.removed);
    for (const [name, version] of channelMutationVersions) {
      if (version === mutationSnapshot.get(name)) continue;
      if (removedChannels.value.has(name)) nextRemovedChannels.add(name);
      else nextRemovedChannels.delete(name);
    }
    draftChannels.value = nextChannels;
    savedChannels.value = nextSavedChannels;
    removedChannels.value = nextRemovedChannels;
    const runtimeStatuses = parseRuntimeStatuses(rawRuntimeStatuses);
    const configStatusMap = new Map(configStatus.channels.map((item) => [item.name, item]));
    const runtimeStatusMap = new Map(runtimeStatuses.map((item) => [item.name, item]));
    channelStatuses.value = Object.entries(nextChannels)
      .filter(([name]) => !nextRemovedChannels.has(name))
      .map(([name, channel]) => {
        const runtime = runtimeStatusMap.get(name);
        const configured = configStatusMap.get(name);
        return {
          name,
          enabled: Boolean(channel?.enabled),
          ready: Boolean(runtime?.registered && runtime.health !== 'down'),
          missing_fields: runtime?.registered ? [] : (configured?.missing_fields ?? []),
          notes: [runtime?.lifecycle, runtime?.diagnosis].filter(Boolean) as string[],
        };
      });
    const visibleNames = Object.keys(nextChannels).filter((name) => !nextRemovedChannels.has(name));
    if (!selectedChannel.value || !nextChannels[selectedChannel.value] || nextRemovedChannels.has(selectedChannel.value)) {
      selectedChannel.value = visibleNames[0] ?? null;
    }
    return true;
  } catch (e) {
    if (requestGeneration !== loadGeneration) return false;
    console.error('Failed to load channels:', e);
    loadError.value = errorMessage(e, t('channels.loadFailed'));
    return false;
  } finally {
    if (requestGeneration === loadGeneration) {
      isInitializing.value = false;
      isLoading.value = false;
    }
  }
}

onMounted(async () => {
  await loadChannels();
});

const channelStatusMap = computed(() => {
  return new Map(channelStatuses.value.map((item) => [item.name, item]));
});

const visibleDraftChannels = computed(() => {
  const visible: Record<string, Record<string, any>> = {};
  for (const [name, config] of Object.entries(draftChannels.value)) {
    if (!removedChannels.value.has(name)) visible[name] = config;
  }
  return visible;
});

const recoverablePlatforms = computed(() =>
  Array.from(removedChannels.value).filter((platform) => Boolean(CHANNEL_PLATFORMS[platform])),
);

const canAddChannel = computed(() => recoverablePlatforms.value.length > 0);

const busyChannelNames = computed(() => Array.from(busyChannels.value));

const isChannelBusy = (channelName: string) => busyChannels.value.has(channelName);

const setChannelBusy = (channelName: string, busy: boolean) => {
  const next = new Set(busyChannels.value);
  if (busy) next.add(channelName);
  else next.delete(channelName);
  busyChannels.value = next;
};

const setChannelError = (channelName: string, error: unknown) => {
  const next = { ...channelErrors.value };
  if (error) next[channelName] = errorMessage(error, t('channels.saveFailed'));
  else delete next[channelName];
  channelErrors.value = next;
};

const markChannelMutation = (channelName: string) => {
  const version = ++channelMutationGeneration;
  channelMutationVersions.set(channelName, version);
  return version;
};

const valuesEqual = (left: unknown, right: unknown) => JSON.stringify(left) === JSON.stringify(right);

const commitLocalChannelSave = (name: string, submittedConfig: Record<string, any>) => {
  const submitted = cloneValue(submittedConfig);
  const currentDraft = draftChannels.value[name];
  const nextDraftChannels = { ...draftChannels.value };
  if (currentDraft === undefined || valuesEqual(currentDraft, submitted)) {
    nextDraftChannels[name] = cloneValue(submitted);
  }
  draftChannels.value = nextDraftChannels;
  savedChannels.value = { ...savedChannels.value, [name]: cloneValue(submitted) };

  const nextRemovedChannels = new Set(removedChannels.value);
  nextRemovedChannels.delete(name);
  removedChannels.value = nextRemovedChannels;

  const existingStatus = channelStatuses.value.find((status) => status.name === name);
  channelStatuses.value = existingStatus
    ? channelStatuses.value.map((status) =>
        status.name === name ? { ...status, enabled: Boolean(submitted.enabled) } : status,
      )
    : [
        ...channelStatuses.value,
        { name, enabled: Boolean(submitted.enabled), ready: false, missing_fields: [], notes: [] },
      ];
  if (!selectedChannel.value) selectedChannel.value = name;
};

const commitLocalChannelDelete = (name: string) => {
  const nextDraftChannels = { ...draftChannels.value };
  const nextSavedChannels = { ...savedChannels.value };
  delete nextDraftChannels[name];
  delete nextSavedChannels[name];
  draftChannels.value = nextDraftChannels;
  savedChannels.value = nextSavedChannels;

  const nextRemovedChannels = new Set(removedChannels.value);
  nextRemovedChannels.add(name);
  removedChannels.value = nextRemovedChannels;
  channelStatuses.value = channelStatuses.value.filter((status) => status.name !== name);
  setChannelError(name, null);

  if (selectedChannel.value === name) {
    selectedChannel.value = Object.keys(nextDraftChannels).find((channelName) => !nextRemovedChannels.has(channelName)) ?? null;
  }
};

const selectedChannelDraft = computed(() => {
  if (!selectedChannel.value) return null;
  return visibleDraftChannels.value[selectedChannel.value] ?? null;
});

const selectedChannelError = computed(() =>
  selectedChannel.value ? channelErrors.value[selectedChannel.value] ?? null : null,
);

const isSaving = computed(() => Boolean(selectedChannel.value && isChannelBusy(selectedChannel.value)));

const isDirty = computed(() => {
  if (!selectedChannel.value || !selectedChannelDraft.value) return false;
  return JSON.stringify(selectedChannelDraft.value) !== JSON.stringify(savedChannels.value[selectedChannel.value] ?? null);
});

const toggleChannelEnabled = (channelName: string) => {
  if (isChannelBusy(channelName)) return;
  if (draftChannels.value[channelName]) {
    draftChannels.value[channelName].enabled = !draftChannels.value[channelName].enabled;
  }
};

const persistChannel = async (name: string, config: Record<string, any>) => {
  if (isChannelBusy(name)) return;
  const submitted = cloneValue(config);
  markChannelMutation(name);
  setChannelError(name, null);
  setChannelBusy(name, true);
  try {
    await props.saveChannelConfigAction(name, submitted);
    commitLocalChannelSave(name, submitted);
  } catch (error) {
    setChannelError(name, error);
    throw error;
  } finally {
    setChannelBusy(name, false);
  }
};

const saveCurrentChannel = async () => {
  if (!selectedChannel.value || !selectedChannelDraft.value || isSaving.value || !isDirty.value) return;
  try {
    await persistChannel(selectedChannel.value, selectedChannelDraft.value);
  } catch {
    // The inline error remains next to the active editor and the draft stays editable.
  }
};

// 向导相关函数
const openWizard = () => {
  if (!canAddChannel.value) return;
  editingChannel.value = null;
  wizardOpen.value = true;
};

const handleWizardTest = async (data: ChannelWizardData) => {
  const config = normalizeChannelConfig(data.platform, cloneValue(data.credentials ?? {}));
  delete config.enabled;
  const result = await invoke<unknown>('probe_channel', {
    name: data.platform,
    config,
  });
  const raw = isRecord(result) ? result : {};
  const status = typeof raw.status === 'string' ? raw.status.toLowerCase() : '';
  const success =
    typeof raw.success === 'boolean'
      ? raw.success
      : typeof raw.ok === 'boolean'
        ? raw.ok
        : status === 'ok' || status === 'success' || raw.healthy === true;
  const message =
    (typeof raw.message === 'string' && raw.message) ||
    (typeof raw.error === 'string' && raw.error) ||
    (success ? t('channels.testSuccess') : t('channels.testFailed', { error: t('channels.testUnknownError') }));
  return { success, message };
};

const handleWizardComplete = async (data: ChannelWizardData) => {
  const existing = draftChannels.value[data.platform] ?? {};
  const credentials = normalizeChannelConfig(data.platform, cloneValue(data.credentials ?? {}));
  delete credentials.enabled;
  const enabled = editingChannel.value ? Boolean(existing.enabled) : true;
  await persistChannel(data.platform, {
    ...cloneValue(existing),
    ...credentials,
    enabled,
  });
};

const handleCardEdit = (name: string) => {
  editingChannel.value = name;
  selectedChannel.value = name;
  wizardOpen.value = true;
};

const handleCardDelete = async (name: string) => {
  if (isChannelBusy(name)) return false;
  return appConfirmAsync(
    t('channels.deleteConfirm', { name }),
    async () => {
      markChannelMutation(name);
      setChannelError(name, null);
      setChannelBusy(name, true);
      try {
        const result = await invoke<unknown>('delete_channel', { name });
        if (isRecord(result) && typeof result.status === 'string' && result.status.toLowerCase() === 'error') {
          throw new Error(typeof result.message === 'string' ? result.message : t('channels.deleteFailed'));
        }
        commitLocalChannelDelete(name);
      } catch (error) {
        setChannelError(name, error);
        throw error;
      } finally {
        setChannelBusy(name, false);
      }
    },
    {
      title: t('channels.deleteTitle'),
      confirmLabel: t('channels.delete'),
    },
  );
};

const handleCardToggle = async (name: string) => {
  if (isChannelBusy(name)) return;
  toggleChannelEnabled(name);
  try {
    await persistChannel(name, draftChannels.value[name]);
  } catch {
    // The failed draft and its inline error remain available for retry.
  }
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
          <button v-if="canAddChannel" class="btn-primary" @click="openWizard">
            <Plus :size="16" />
            {{ t('channels.addChannel') }}
          </button>
        </div>
      </div>

      <div v-if="loadError" class="channels-load-error" role="alert">
        <span class="channels-load-error-message">{{ loadError }}</span>
        <button
          type="button"
          class="channels-load-retry"
          :disabled="isLoading"
          @click="handleRefresh"
        >
          <RefreshCw :size="14" :class="{ 'animate-spin': isLoading }" />
          {{ isLoading ? t('channels.loading') : t('channels.retry') }}
        </button>
      </div>

      <!-- 卡片视图 -->
      <div v-if="viewMode === 'card'" class="channels-main-content">
        <ChannelCardView
          :channels="visibleDraftChannels"
          :statuses="channelStatuses"
          :loading="isInitializing"
          :can-add="canAddChannel"
          :busy-channels="busyChannelNames"
          :errors="channelErrors"
          :error="loadError"
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
                    :disabled="isChannelBusy(selectedChannel)"
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
            <p v-if="selectedChannelError" class="channel-operation-error" role="alert">
              {{ selectedChannelError }}
            </p>
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
              :disabled="isChannelBusy(selectedChannel)"
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
      :available-platforms="recoverablePlatforms"
      :on-test="handleWizardTest"
      :on-complete="handleWizardComplete"
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

.channel-operation-error {
  margin: 0;
  padding: 0.75rem 1rem;
  border: 1px solid var(--danger);
  border-radius: var(--radius-sm);
  color: var(--danger);
  font-size: 0.875rem;
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

.channels-load-error {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.75rem 1.5rem;
  border-bottom: 1px solid var(--danger);
  background: var(--accent-bg-light);
  color: var(--danger);
}

.channels-load-error-message {
  min-width: 0;
  font-size: 0.875rem;
}

.channels-load-retry {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  flex-shrink: 0;
  padding: 0.375rem 0.75rem;
  border: 1px solid var(--danger);
  border-radius: var(--radius-sm);
  background: var(--panel);
  color: var(--danger);
  cursor: pointer;
  font-size: 0.75rem;
  font-weight: 500;
}

.channels-load-retry:hover:not(:disabled) {
  background: var(--accent-bg-light);
}

.channels-load-retry:disabled {
  cursor: not-allowed;
  opacity: 0.5;
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

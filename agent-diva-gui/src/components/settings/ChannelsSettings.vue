<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { ChevronDown, ChevronRight, RefreshCw, Save } from '@lucide/vue';
import { useI18n } from 'vue-i18n';

import { isTauriRuntime } from '../../api/desktop';
import {
  loadChannelsState,
  saveChannel,
  type ChannelView,
} from '../../api/settings';

const { t } = useI18n();

const previewMode = computed(() => !isTauriRuntime());
const views = ref<ChannelView[]>([]);
const loading = ref(false);
const saving = ref('');
const error = ref('');
const expanded = ref<string | null>(null);

// Per-channel editor drafts, keyed by channel name.
const allowFromDraft = ref<Record<string, string>>({});
const tokenEnvDraft = ref<Record<string, string>>({});

async function refresh() {
  if (previewMode.value) {
    views.value = [];
    return;
  }
  loading.value = true;
  error.value = '';
  try {
    views.value = await loadChannelsState();
    for (const view of views.value) {
      if (!(view.name in allowFromDraft.value)) {
        allowFromDraft.value[view.name] = view.envelope.allow_from.join('\n');
      }
      if (!(view.name in tokenEnvDraft.value)) {
        tokenEnvDraft.value[view.name] = view.envelope.token_env;
      }
    }
  } catch (err) {
    error.value = String(err);
  } finally {
    loading.value = false;
  }
}

function statusLabel(view: ChannelView): string {
  const status = view.status;
  if (!view.envelope.configured && !status?.configured) return t('settings.unconfigured');
  if (status?.started) return t('settings.running');
  if (view.envelope.enabled) return t('settings.needsRestart');
  return t('channels.disabled');
}

function statusClass(view: ChannelView): string {
  if (view.status?.started) return 'skills-status-badge active';
  return 'skills-status-badge available';
}

function allowFromFor(name: string): string[] {
  return (allowFromDraft.value[name] ?? '')
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
}

function draftDirty(view: ChannelView): boolean {
  return (
    JSON.stringify(allowFromFor(view.name)) !==
      JSON.stringify(view.envelope.allow_from) ||
    (tokenEnvDraft.value[view.name] ?? '') !== view.envelope.token_env
  );
}

async function toggleEnabled(view: ChannelView) {
  if (saving.value) return;
  saving.value = view.name;
  error.value = '';
  try {
    await saveChannel({ name: view.name, enabled: !view.envelope.enabled });
    await refresh();
  } catch (err) {
    error.value = String(err);
  } finally {
    saving.value = '';
  }
}

async function saveEditor(view: ChannelView) {
  if (saving.value) return;
  saving.value = view.name;
  error.value = '';
  try {
    await saveChannel({
      name: view.name,
      allow_from: allowFromFor(view.name),
      token_env: (tokenEnvDraft.value[view.name] ?? '').trim(),
    });
    await refresh();
  } catch (err) {
    error.value = String(err);
  } finally {
    saving.value = '';
  }
}

function toggleExpand(name: string) {
  expanded.value = expanded.value === name ? null : name;
}

onMounted(refresh);
</script>

<template>
  <div class="space-y-4">
    <div class="flex flex-wrap items-center gap-3">
      <button class="ui-button ui-button--ghost skills-btn" :disabled="loading" @click="refresh">
        <RefreshCw :size="14" :class="{ 'animate-spin': loading }" />
        {{ t('settings.refreshChannels') }}
      </button>
      <p class="text-xs" style="color: var(--muted-foreground);">
        {{ t('settings.restartHint') }}
      </p>
    </div>

    <p v-if="error" class="text-xs" style="color: var(--destructive); break-words;">{{ error }}</p>
    <div v-if="loading && views.length === 0" class="text-sm" style="color: var(--muted-foreground);">
      {{ t('settings.loadingChannels') }}
    </div>
    <div v-else-if="views.length === 0" class="text-sm" style="color: var(--muted-foreground);">
      {{ previewMode ? t('settings.channelsPreviewOnly') : t('settings.empty') }}
    </div>

    <div v-else class="space-y-3">
      <div v-for="view in views" :key="view.name" class="skills-list-item">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div class="flex flex-wrap items-center gap-2 min-w-0">
            <button class="ui-button ui-button--ghost ui-button--compact toolbar-btn" @click="toggleExpand(view.name)">
              <ChevronDown v-if="expanded === view.name" :size="14" />
              <ChevronRight v-else :size="14" />
            </button>
            <div class="skills-item-name">{{ view.name }}</div>
            <span :class="statusClass(view)">{{ statusLabel(view) }}</span>
            <span v-if="view.pendingRestart" class="skills-status-badge available">
              {{ t('settings.pendingRestart') }}
            </span>
          </div>
          <button
            class="ui-button ui-button--ghost skills-btn"
            :disabled="Boolean(saving) || previewMode"
            @click="toggleEnabled(view)"
          >
            {{ view.envelope.enabled ? t('settings.disableChannel') : t('settings.enableChannel') }}
          </button>
        </div>

        <div
          v-if="view.status"
          class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-[11px]"
          style="color: var(--muted-foreground);"
        >
          <span>
            {{ t('settings.tokenEnv') }}:
            <code>{{ view.status.token_env || '—' }}</code>
            <span :style="{ color: view.status.token_env_set ? 'var(--success)' : 'var(--destructive)' }">
              {{ view.status.token_env_set ? t('network.envConfigured') : t('network.envMissing') }}
            </span>
          </span>
          <span v-if="view.status.health">
            {{ t('settings.health') }}:
            {{ view.status.health.ok ? 'ok' : (view.status.health.class || 'unknown') }}
            <template v-if="view.status.health.detail">— {{ view.status.health.detail }}</template>
          </span>
          <span v-if="view.status.note" style="color: var(--warning);">{{ view.status.note }}</span>
        </div>

        <div v-if="expanded === view.name" class="mt-3 space-y-3">
          <div>
            <label class="welcome-label">{{ t('settings.allowFrom') }}</label>
            <p class="text-[11px] mb-1" style="color: var(--muted-foreground);">{{ t('settings.allowFromHint') }}</p>
            <textarea
              v-model="allowFromDraft[view.name]"
              class="ui-input skills-search-input"
              rows="3"
              style="width: 100%; font-family: monospace; resize: vertical;"
              :disabled="previewMode"
            />
          </div>
          <div>
            <label class="welcome-label">{{ t('settings.tokenEnv') }}</label>
            <p class="text-[11px] mb-1" style="color: var(--muted-foreground);">{{ t('settings.tokenEnvHint') }}</p>
            <input
              v-model="tokenEnvDraft[view.name]"
              type="text"
              class="ui-input skills-search-input"
              style="width: 100%; font-family: monospace;"
              :disabled="previewMode"
            />
          </div>
          <div class="flex items-center gap-2">
            <button
              class="ui-button ui-button--primary skills-btn skills-btn-primary"
              :disabled="Boolean(saving) || !draftDirty(view)"
              @click="saveEditor(view)"
            >
              <Save :size="14" />
              {{ t('settings.save') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

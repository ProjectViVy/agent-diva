<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Mic, LoaderCircle, Trash2 } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import {
  NativeUnavailableError,
  speechConfig,
  speechCredentials,
  voiceAssets,
  type SpeechConfigReadback,
  type VoiceAssetDescriptor,
} from '../../api/speech';
import { showAppToast } from '../../utils/appToast';
import { recordGuiDiagnostic } from '../../state/gui-diagnostics';

const { t } = useI18n();

interface SpeechPreferences {
  stt: { provider: string; base_url: string; model: string };
  tts: {
    provider: string;
    siliconflow?: { base_url: string; model: string; voice: string; speed: number; reference?: unknown };
    minimax?: { base_url: string; model: string; voice_id: string; speed: number; volume: number };
  };
  auto_read_replies: boolean;
}

const config = ref<SpeechConfigReadback | null>(null);
const draft = ref<SpeechPreferences | null>(null);
const assets = ref<VoiceAssetDescriptor[]>([]);
const loading = ref(true);
const saving = ref(false);
const unavailable = ref(false);
const loadError = ref('');
const credentialDrafts = ref<{ siliconflow: string; minimax: string }>({ siliconflow: '', minimax: '' });
const credentialBusy = ref<Record<string, boolean>>({});
const assetBusy = ref<Record<string, boolean>>({});

const isDirty = computed(() =>
  config.value !== null && JSON.stringify(draft.value) !== JSON.stringify(config.value.preferences),
);

async function refresh() {
  loading.value = true;
  loadError.value = '';
  try {
    const readback = await speechConfig.get();
    config.value = readback;
    draft.value = JSON.parse(JSON.stringify(readback.preferences ?? {}));
    assets.value = await voiceAssets.list().catch(() => []);
  } catch (err) {
    if (err instanceof NativeUnavailableError) {
      unavailable.value = true;
    } else {
      loadError.value = err instanceof Error ? err.message : String(err);
      recordGuiDiagnostic({ level: 'warn', component: 'speech/settings', message: 'speech config load failed' });
    }
  } finally {
    loading.value = false;
  }
}

async function save() {
  if (saving.value || !isDirty.value || !config.value || !draft.value) return;
  saving.value = true;
  try {
    const reply = await speechConfig.update({
      base_revision: config.value.revision,
      preferences: draft.value,
    });
    config.value = reply;
    showAppToast(t('console.saved'), 'success');
  } catch (err) {
    // CAS conflicts surface through the error string; refresh gives the user
    // the authoritative document instead of a silent overwrite.
    loadError.value = err instanceof Error ? err.message : String(err);
    await refresh();
  } finally {
    saving.value = false;
  }
}

async function setCredential(provider: 'siliconflow' | 'minimax') {
  const key = credentialDrafts.value[provider].trim();
  if (!key || credentialBusy.value[provider]) return;
  credentialBusy.value = { ...credentialBusy.value, [provider]: true };
  try {
    await speechCredentials.set(provider, key);
    // The key field clears immediately — provider credentials never persist
    // in renderer state.
    credentialDrafts.value = { ...credentialDrafts.value, [provider]: '' };
    showAppToast(t('console.saved'), 'success');
    await refresh();
  } catch (err) {
    loadError.value = err instanceof Error ? err.message : String(err);
  } finally {
    credentialBusy.value = { ...credentialBusy.value, [provider]: false };
  }
}

async function deleteCredential(provider: 'siliconflow' | 'minimax') {
  if (credentialBusy.value[provider]) return;
  credentialBusy.value = { ...credentialBusy.value, [provider]: true };
  try {
    await speechCredentials.delete(provider);
    await refresh();
  } catch (err) {
    loadError.value = err instanceof Error ? err.message : String(err);
  } finally {
    credentialBusy.value = { ...credentialBusy.value, [provider]: false };
  }
}

async function importAsset(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  try {
    await voiceAssets.import(await file.arrayBuffer(), file.name, file.type || 'audio/wav');
    await refresh();
  } catch (err) {
    loadError.value = err instanceof Error ? err.message : String(err);
  } finally {
    input.value = '';
  }
}

async function deleteAsset(assetId: string) {
  if (assetBusy.value[assetId]) return;
  assetBusy.value = { ...assetBusy.value, [assetId]: true };
  try {
    await voiceAssets.delete(assetId);
    await refresh();
  } catch (err) {
    loadError.value = err instanceof Error ? err.message : String(err);
  } finally {
    assetBusy.value = { ...assetBusy.value, [assetId]: false };
  }
}

const credentialStateText = (state?: string) =>
  state === 'configured' ? t('speechSettings.credentialSet')
    : state === 'unavailable' ? t('speechSettings.credentialUnavailable')
    : t('speechSettings.credentialMissing');

onMounted(refresh);
</script>

<template>
  <div class="p-6 space-y-6 fade-in">
    <div class="flex items-start justify-between gap-4">
      <div class="flex items-center space-x-3">
        <div class="settings-dashboard-icon">
          <Mic :size="20" />
        </div>
        <div>
          <h3 class="settings-dashboard-title">{{ t('speechSettings.title') }}</h3>
          <p class="settings-dashboard-desc">{{ t('speechSettings.desc') }}</p>
        </div>
      </div>
      <button
        type="button"
        class="btn-save-config settings-btn inline-flex min-w-[112px] items-center justify-center gap-2"
        :disabled="saving || !isDirty || loading || unavailable"
        @click="save"
      >
        <LoaderCircle v-if="saving" :size="16" class="animate-spin" />
        <span>{{ saving ? t('console.saving') : t('console.saveConfig') }}</span>
      </button>
    </div>

    <p v-if="unavailable" class="settings-section settings-muted text-sm">
      {{ t('speechSettings.nativeUnavailable') }}
    </p>
    <p v-if="loadError" class="text-xs" style="color: var(--danger);">{{ loadError }}</p>
    <p v-if="loading && !unavailable" class="settings-muted text-sm">{{ t('console.loading') }}</p>

    <template v-if="draft && !unavailable">
      <div class="settings-section">
        <h4 class="settings-muted text-xs font-medium uppercase tracking-wider mb-3">{{ t('speechSettings.sttTitle') }}</h4>
        <div class="grid grid-cols-3 gap-4">
          <div class="space-y-1">
            <label class="block text-xs font-medium settings-muted uppercase tracking-wider">{{ t('speechSettings.provider') }}</label>
            <select v-model="draft.stt.provider" class="settings-input">
              <option value="siliconflow">SiliconFlow</option>
            </select>
          </div>
          <div class="space-y-1">
            <label class="block text-xs font-medium settings-muted uppercase tracking-wider">{{ t('speechSettings.baseUrl') }}</label>
            <input v-model="draft.stt.base_url" class="settings-input" :placeholder="t('speechSettings.baseUrlPlaceholder')" />
          </div>
          <div class="space-y-1">
            <label class="block text-xs font-medium settings-muted uppercase tracking-wider">{{ t('speechSettings.model') }}</label>
            <input v-model="draft.stt.model" class="settings-input" />
          </div>
        </div>
      </div>

      <div class="settings-section">
        <h4 class="settings-muted text-xs font-medium uppercase tracking-wider mb-3">{{ t('speechSettings.ttsTitle') }}</h4>
        <div class="grid grid-cols-3 gap-4">
          <div class="space-y-1">
            <label class="block text-xs font-medium settings-muted uppercase tracking-wider">{{ t('speechSettings.provider') }}</label>
            <select v-model="draft.tts.provider" class="settings-input">
              <option value="siliconflow">SiliconFlow</option>
              <option value="minimax">MiniMax</option>
            </select>
          </div>
          <template v-if="draft.tts.provider === 'siliconflow' && draft.tts.siliconflow">
            <div class="space-y-1">
              <label class="block text-xs font-medium settings-muted uppercase tracking-wider">{{ t('speechSettings.model') }}</label>
              <input v-model="draft.tts.siliconflow.model" class="settings-input" />
            </div>
            <div class="space-y-1">
              <label class="block text-xs font-medium settings-muted uppercase tracking-wider">{{ t('speechSettings.voice') }}</label>
              <input v-model="draft.tts.siliconflow.voice" class="settings-input" />
            </div>
          </template>
          <template v-if="draft.tts.provider === 'minimax' && draft.tts.minimax">
            <div class="space-y-1">
              <label class="block text-xs font-medium settings-muted uppercase tracking-wider">{{ t('speechSettings.model') }}</label>
              <input v-model="draft.tts.minimax.model" class="settings-input" />
            </div>
            <div class="space-y-1">
              <label class="block text-xs font-medium settings-muted uppercase tracking-wider">{{ t('speechSettings.voiceId') }}</label>
              <input v-model="draft.tts.minimax.voice_id" class="settings-input" />
            </div>
          </template>
        </div>
        <label class="mt-4 flex items-center gap-2 text-sm settings-muted">
          <input type="checkbox" v-model="draft.auto_read_replies" />
          {{ t('speechSettings.autoRead') }}
        </label>
      </div>

      <div class="settings-section" v-if="config">
        <h4 class="settings-muted text-xs font-medium uppercase tracking-wider mb-3">{{ t('speechSettings.credentialsTitle') }}</h4>
        <div
          v-for="provider in (['siliconflow', 'minimax'] as const)"
          :key="provider"
          class="grid grid-cols-[1fr_auto_auto] items-center gap-3 py-2"
        >
          <div>
            <span class="text-sm font-medium">{{ provider === 'siliconflow' ? 'SiliconFlow' : 'MiniMax' }}</span>
            <span class="settings-muted text-xs ml-2">{{ credentialStateText(config.credential_state?.[provider]) }}</span>
          </div>
          <input
            v-model="credentialDrafts[provider]"
            type="password"
            class="settings-input w-64"
            :placeholder="t('speechSettings.credentialPlaceholder')"
            autocomplete="off"
          />
          <div class="flex gap-2">
            <button class="settings-btn text-xs px-3 py-1" :disabled="credentialBusy[provider] || !credentialDrafts[provider].trim()" @click="setCredential(provider)">
              {{ t('speechSettings.credentialSave') }}
            </button>
            <button class="settings-btn text-xs px-3 py-1" :disabled="credentialBusy[provider] || config.credential_state?.[provider] !== 'configured'" @click="deleteCredential(provider)">
              {{ t('speechSettings.credentialDelete') }}
            </button>
          </div>
        </div>
      </div>

      <div class="settings-section">
        <h4 class="settings-muted text-xs font-medium uppercase tracking-wider mb-3">{{ t('speechSettings.assetsTitle') }}</h4>
        <div class="flex items-center gap-3 mb-3">
          <input type="file" accept="audio/*" class="text-xs" @change="importAsset" />
        </div>
        <div v-if="assets.length === 0" class="settings-muted text-sm">{{ t('speechSettings.assetsEmpty') }}</div>
        <div
          v-for="asset in assets"
          :key="asset.asset_id"
          class="flex items-center justify-between py-1 text-sm"
        >
          <span>{{ asset.display_name }} <span class="settings-muted text-xs">({{ asset.mime_type }}, {{ asset.size_bytes }}B)</span></span>
          <button class="settings-btn text-xs px-2 py-1" :disabled="assetBusy[asset.asset_id]" @click="deleteAsset(asset.asset_id)">
            <Trash2 :size="12" />
          </button>
        </div>
      </div>
    </template>
  </div>
</template>

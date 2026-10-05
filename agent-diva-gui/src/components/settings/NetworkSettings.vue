<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Globe, LoaderCircle } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import {
  loadNetworkToolsState,
  saveNetworkToolsState,
  type NetworkToolsState,
} from '../../api/settings';
import { showAppToast } from '../../utils/appToast';

const { t } = useI18n();

const state = ref<NetworkToolsState | null>(null);
const draft = ref<{ provider: string; searchEnabled: boolean; fetchEnabled: boolean }>({
  provider: '',
  searchEnabled: false,
  fetchEnabled: false,
});
const loading = ref(false);
const saving = ref(false);
const loadError = ref('');

const isDirty = computed(() => {
  if (!state.value) return false;
  return (
    draft.value.provider !== state.value.searchProvider ||
    draft.value.searchEnabled !== state.value.searchEnabled ||
    draft.value.fetchEnabled !== state.value.fetchEnabled
  );
});

const selectedProvider = computed(() =>
  state.value?.providers.find((p) => p.name === draft.value.provider) ?? null
);

async function refresh() {
  loading.value = true;
  loadError.value = '';
  try {
    state.value = await loadNetworkToolsState();
    draft.value = {
      provider: state.value.searchProvider,
      searchEnabled: state.value.searchEnabled,
      fetchEnabled: state.value.fetchEnabled,
    };
  } catch (err) {
    loadError.value = String(err);
  } finally {
    loading.value = false;
  }
}

async function save() {
  if (saving.value || !isDirty.value) return;
  saving.value = true;
  try {
    await saveNetworkToolsState({
      searchProvider: draft.value.provider,
      searchEnabled: draft.value.searchEnabled,
      fetchEnabled: draft.value.fetchEnabled,
    });
    showAppToast(t('console.saved'), 'success');
    await refresh();
  } catch (err) {
    loadError.value = String(err);
    showAppToast(t('app.configUpdateError', { error: err }), 'error');
  } finally {
    saving.value = false;
  }
}

onMounted(refresh);
</script>

<template>
  <div class="p-6 space-y-6 fade-in">
    <div class="flex items-start justify-between gap-4">
      <div class="flex items-center space-x-3">
        <div class="settings-dashboard-icon">
          <Globe :size="20" />
        </div>
        <div>
          <h3 class="settings-dashboard-title">{{ t('network.title') }}</h3>
          <p class="settings-dashboard-desc">{{ t('network.desc') }}</p>
        </div>
      </div>
      <button
        type="button"
        class="btn-save-config settings-btn inline-flex min-w-[112px] items-center justify-center gap-2"
        :disabled="saving || !isDirty || loading"
        @click="save"
      >
        <LoaderCircle v-if="saving" :size="16" class="animate-spin" />
        <span>{{ saving ? t('console.saving') : t('console.saveConfig') }}</span>
      </button>
    </div>

    <p v-if="loadError" class="text-xs" style="color: var(--danger);">{{ loadError }}</p>

    <div v-if="state" class="settings-section">
      <div class="grid grid-cols-2 gap-4">
        <div class="space-y-1">
          <label class="block text-xs font-medium settings-muted uppercase tracking-wider">{{ t('network.provider') }}</label>
          <select v-model="draft.provider" class="settings-input">
            <option value="">{{ t('network.providerAutomatic') }}</option>
            <option
              v-for="provider in state.providers"
              :key="provider.name"
              :value="provider.name"
            >
              {{ provider.name }}
            </option>
          </select>
        </div>
        <div class="space-y-1" v-if="selectedProvider">
          <label class="block text-xs font-medium settings-muted uppercase tracking-wider">{{ t('network.credential') }}</label>
          <div class="settings-input flex items-center justify-between">
            <span v-if="selectedProvider.keyless" class="settings-muted text-sm">{{ t('network.keylessProvider') }}</span>
            <template v-else>
              <code class="text-xs font-mono">{{ selectedProvider.env_key }}</code>
              <span
                class="text-xs font-medium"
                :style="{ color: selectedProvider.configured ? 'var(--success, #16a34a)' : 'var(--warning, #d97706)' }"
              >
                {{ selectedProvider.configured ? t('network.envConfigured') : t('network.envMissing') }}
              </span>
            </template>
          </div>
        </div>
      </div>

      <div class="flex space-x-6 mt-4">
        <label class="settings-label flex items-center space-x-2 cursor-pointer">
          <input type="checkbox" v-model="draft.searchEnabled" class="settings-checkbox" />
          <span>{{ t('network.enableSearch') }}</span>
        </label>
        <label class="settings-label flex items-center space-x-2 cursor-pointer">
          <input type="checkbox" v-model="draft.fetchEnabled" class="settings-checkbox" />
          <span>{{ t('network.enableFetch') }}</span>
        </label>
      </div>
    </div>
  </div>
</template>

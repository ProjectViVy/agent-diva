<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { Server, Check, Cpu, ShieldCheck, ShieldAlert, RefreshCcw, Plus, Trash2, PlugZap, LoaderCircle, CircleAlert, Eye, EyeOff } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import {
  type ConfigStatusReport,
  getConfigStatus,
} from '../../api/desktop';
import {
  addProviderModel,
  createCustomProvider,
  deleteCustomProvider,
  getProviderModels,
  removeProviderModel,
  testProviderModel,
  type ProviderModelCatalog,
  type ProviderModelTestResult
} from '../../api/providers';
import { invoke } from '@tauri-apps/api/core';
import { appConfirm } from '../../utils/appDialog';

const { t } = useI18n();

interface ProviderSpec {
  name: string;
  api_type: string;
  source?: string;
  display_name: string;
  default_model?: string | null;
  default_api_base: string;
  models: string[];
  custom_models: string[];
}

interface SavedModel {
  id: string;
  provider: string;
  model: string;
  apiBase: string;
  apiKey: string;
  displayName: string;
}

interface ProviderConfigEntry {
  apiKey: string;
  apiBase: string;
  source: 'providers' | 'custom_providers';
}

interface NewProviderForm {
  id: string;
  displayName: string;
  apiBase: string;
  apiKey: string;
  defaultModel: string;
}

type ModelTestState = 'idle' | 'testing' | 'success' | 'failed';

interface ModelTestStatus {
  state: ModelTestState;
  message: string;
  latencyMs?: number;
}

const props = defineProps<{
  config: {
    provider: string;
    apiBase: string;
    apiKey: string;
    model: string;
  };
  providerConfigs?: Record<string, ProviderConfigEntry>;
  savedModels?: SavedModel[];
  saveConfigAction: (config: {
    provider: string;
    apiBase: string;
    apiKey: string;
    model: string;
  }) => Promise<void>;
}>();

const emit = defineEmits<{
  (e: 'update-saved-models', models: SavedModel[]): void;
}>();

const providers = ref<ProviderSpec[]>([]);
const statusReport = ref<ConfigStatusReport | null>(null);
const localConfig = ref({ ...props.config });
const localSavedModels = ref<SavedModel[]>(JSON.parse(JSON.stringify(props.savedModels || [])));
const selectedProvider = ref<ProviderSpec | null>(null);
const searchTerm = ref('');
const modelSearchTerm = ref('');
const manualModelName = ref('');
const isManualModelDialogOpen = ref(false);
const isCreateProviderDialogOpen = ref(false);
const isSavingProvider = ref(false);
const isDeletingCustomProvider = ref<string | null>(null);
const providerApiKeys = ref<Record<string, string>>({});
const providerApiBases = ref<Record<string, string>>({});
const providerApiKeyVisibility = ref<Record<string, boolean>>({});
const isRefreshing = ref(false);
const runtimeCatalogs = ref<Record<string, ProviderModelCatalog>>({});
const modelTestStatuses = ref<Record<string, ModelTestStatus>>({});
const modelTestTimers = new Map<string, ReturnType<typeof setTimeout>>();
const isSavingConfig = ref(false);
const lastSavedConfigSnapshot = ref(JSON.stringify(props.config));
const lastSavedModelsSnapshot = ref(JSON.stringify(props.savedModels || []));
const newProviderForm = ref<NewProviderForm>({
  id: '',
  displayName: '',
  apiBase: '',
  apiKey: '',
  defaultModel: '',
});
const isCreateProviderApiKeyVisible = ref(false);

const dedupeProviders = (items: ProviderSpec[]) => {
  const seen = new Map<string, ProviderSpec>();
  for (const provider of items) {
    if (!seen.has(provider.name)) {
      seen.set(provider.name, provider);
      continue;
    }

    const existing = seen.get(provider.name)!;
    if (existing.source !== 'builtin' && provider.source === 'builtin') {
      seen.set(provider.name, provider);
    }
  }
  return Array.from(seen.values());
};

const cloneSavedModels = (models: SavedModel[]) => JSON.parse(JSON.stringify(models)) as SavedModel[];

const buildProviderStateFromDraft = () => {
  providerApiKeys.value = {};
  providerApiBases.value = {};

  Object.entries(props.providerConfigs || {}).forEach(([providerName, entry]) => {
    if (entry.apiKey) {
      providerApiKeys.value[providerName] = entry.apiKey;
    }
    if (entry.apiBase) {
      providerApiBases.value[providerName] = entry.apiBase;
    }
  });

  if (localSavedModels.value) {
    localSavedModels.value.forEach((model) => {
      if (model.apiKey) {
        providerApiKeys.value[model.provider] = model.apiKey;
      }
      if (model.apiBase) {
        providerApiBases.value[model.provider] = model.apiBase;
      }
    });
  }

  if (localConfig.value.provider) {
    if (localConfig.value.apiKey) {
      providerApiKeys.value[localConfig.value.provider] = localConfig.value.apiKey;
    }
    if (localConfig.value.apiBase) {
      providerApiBases.value[localConfig.value.provider] = localConfig.value.apiBase;
    }
  }
};

const isProviderApiKeyVisible = (providerName: string) =>
  providerApiKeyVisibility.value[providerName] ?? false;

const toggleProviderApiKeyVisibility = (providerName: string) => {
  providerApiKeyVisibility.value = {
    ...providerApiKeyVisibility.value,
    [providerName]: !isProviderApiKeyVisible(providerName),
  };
};

const providerModelsFor = (provider: ProviderSpec | null) => {
  if (!provider) return [];
  return runtimeCatalogs.value[provider.name]?.models ?? provider.models;
};

const refreshProviderState = async () => {
  isRefreshing.value = true;
  try {
    providers.value = dedupeProviders(await invoke('get_providers'));
    statusReport.value = await getConfigStatus();
    buildProviderStateFromDraft();

    if (providers.value.length > 0) {
      let found = providers.value.find(p => p.name === localConfig.value.provider);
      if (!found) {
        found = providers.value.find(p => p.default_api_base === localConfig.value.apiBase);
      }
      const initialProvider =
        providers.value.find((provider) => provider.name === selectedProvider.value?.name) ||
        found ||
        providers.value[0];
      await setSelectedProvider(initialProvider);
    }
  } catch (e) {
    console.error('Failed to load providers:', e);
  } finally {
    isRefreshing.value = false;
  }
};

onMounted(async () => {
  await refreshProviderState();
});

const providerStatusMap = computed(() => {
  const items = statusReport.value?.providers ?? [];
  return new Map(items.map((item) => [item.name, item]));
});

const currentProviderLabel = computed(() => {
  return statusReport.value?.default_provider || t('providers.unresolved');
});

const doctorTone = computed(() => {
  if (!statusReport.value) return 'text-muted';
  return statusReport.value.doctor.ready
    ? 'text-success'
    : 'text-warning';
});

const filteredProviders = computed(() => {
  if (!searchTerm.value) return providers.value;
  const lower = searchTerm.value.toLowerCase();
  return providers.value.filter(p => 
    p.display_name.toLowerCase().includes(lower) || 
    p.name.toLowerCase().includes(lower)
  );
});

const currentConfigSnapshot = computed(() => JSON.stringify(localConfig.value));
const currentSavedModelsSnapshot = computed(() => JSON.stringify(localSavedModels.value));
const isDirty = computed(() =>
  currentConfigSnapshot.value !== lastSavedConfigSnapshot.value ||
  currentSavedModelsSnapshot.value !== lastSavedModelsSnapshot.value
);

const buildSavedModelEntry = (provider: ProviderSpec, modelName: string): SavedModel => {
  const trimmedModelName = modelName.trim();
  const providerKey = providerApiKeys.value[provider.name] || '';
  const providerApiBase = providerApiBases.value[provider.name] || provider.default_api_base;
  return {
    id: `${provider.name}:${trimmedModelName}`,
    provider: provider.name,
    model: trimmedModelName,
    apiBase: providerApiBase,
    apiKey: providerKey,
    displayName: `${provider.display_name} - ${trimmedModelName}`
  };
};

const upsertSavedModel = (entry: SavedModel) => {
  const existingIndex = localSavedModels.value.findIndex(
    (model) => model.provider === entry.provider && model.model === entry.model
  );
  const newSavedModels = cloneSavedModels(localSavedModels.value);

  if (existingIndex >= 0) {
    newSavedModels.splice(existingIndex, 1, {
      ...newSavedModels[existingIndex],
      ...entry
    });
  } else {
    newSavedModels.push(entry);
  }

  localSavedModels.value = newSavedModels;
};

const providerModels = computed(() => {
  const models = providerModelsFor(selectedProvider.value);
  const keyword = modelSearchTerm.value.trim().toLowerCase();
  if (!keyword) {
    return models;
  }
  return models.filter((model) => model.toLowerCase().includes(keyword));
});

const clearSelectedProvider = () => {
  selectedProvider.value = null;
  manualModelName.value = '';
  isManualModelDialogOpen.value = false;
};

const setSelectedProvider = async (provider: ProviderSpec) => {
  selectedProvider.value = provider;
  modelSearchTerm.value = '';
  manualModelName.value = '';
};

const openManualModelDialog = () => {
  if (!selectedProvider.value) return;
  manualModelName.value = '';
  isManualModelDialogOpen.value = true;
};

const closeManualModelDialog = () => {
  isManualModelDialogOpen.value = false;
  manualModelName.value = '';
};

const resetNewProviderForm = () => {
  newProviderForm.value = {
    id: '',
    displayName: '',
    apiBase: '',
    apiKey: '',
    defaultModel: '',
  };
};

const openCreateProviderDialog = () => {
  resetNewProviderForm();
  isCreateProviderDialogOpen.value = true;
};

const closeCreateProviderDialog = () => {
  isCreateProviderDialogOpen.value = false;
  isSavingProvider.value = false;
  isCreateProviderApiKeyVisible.value = false;
  resetNewProviderForm();
};

const createProvider = async () => {
  const id = newProviderForm.value.id.trim();
  const displayName = newProviderForm.value.displayName.trim();
  const apiBase = newProviderForm.value.apiBase.trim();
  const apiKey = newProviderForm.value.apiKey.trim();
  const defaultModel = newProviderForm.value.defaultModel.trim();

  if (!id || !displayName || !apiBase || !defaultModel) {
    return;
  }

  isSavingProvider.value = true;
  try {
    const provider = await createCustomProvider({
      id,
      displayName,
      apiKey,
      apiBase,
      defaultModel,
      models: [defaultModel],
    });

    providerApiKeys.value[id] = apiKey;
    providerApiBases.value[id] = apiBase;
    providers.value = dedupeProviders([...providers.value, provider]);
    const createdProvider =
      providers.value.find((item) => item.name === id) || provider;
    await setSelectedProvider(createdProvider);

    localConfig.value.provider = id;
    localConfig.value.model = defaultModel;
    localConfig.value.apiBase = apiBase;
    localConfig.value.apiKey = apiKey;

    upsertSavedModel(buildSavedModelEntry(createdProvider, defaultModel));
    statusReport.value = await getConfigStatus();
    closeCreateProviderDialog();
  } catch (error) {
    console.error('Failed to create custom provider:', error);
  } finally {
    isSavingProvider.value = false;
  }
};

const refreshSelectedProviderModels = async () => {
  if (!selectedProvider.value) return;

  const provider = selectedProvider.value;
  const apiBase = providerApiBases.value[provider.name] || provider.default_api_base || null;
  const apiKey = providerApiKeys.value[provider.name] || null;

  isRefreshing.value = true;
  try {
    const catalog = await getProviderModels(provider.name, apiBase, apiKey);
    runtimeCatalogs.value = {
      ...runtimeCatalogs.value,
      [provider.name]: catalog,
    };

    const refreshedProvider = providers.value.find((item) => item.name === provider.name);
    if (refreshedProvider) {
      selectedProvider.value = refreshedProvider;
    }
  } catch (error) {
    console.error('Failed to refresh provider models:', error);
  } finally {
    isRefreshing.value = false;
  }
};

const isCustomProviderSpec = (p: ProviderSpec) => p.source === 'custom';

const requestDeleteCustomProvider = async (provider: ProviderSpec) => {
  if (!isCustomProviderSpec(provider)) return;
  if (
    !(await appConfirm(
      t('providers.deleteProviderConfirm', { name: provider.display_name }),
    ))
  ) {
    return;
  }
  const id = provider.name;
  isDeletingCustomProvider.value = id;
  try {
    await deleteCustomProvider(id);

    const nextKeys = { ...providerApiKeys.value };
    delete nextKeys[id];
    providerApiKeys.value = nextKeys;
    const nextBases = { ...providerApiBases.value };
    delete nextBases[id];
    providerApiBases.value = nextBases;
    const nextCat = { ...runtimeCatalogs.value };
    delete nextCat[id];
    runtimeCatalogs.value = nextCat;

    localSavedModels.value = localSavedModels.value.filter((m) => m.provider !== id);

    await refreshProviderState();

    if (!providers.value.some((p) => p.name === localConfig.value.provider)) {
      const next = providers.value[0];
      if (next) {
        localConfig.value.provider = next.name;
        localConfig.value.model = next.default_model || next.models[0] || '';
        localConfig.value.apiBase =
          providerApiBases.value[next.name] || next.default_api_base;
        localConfig.value.apiKey = providerApiKeys.value[next.name] || '';
      }
    }
  } catch (error) {
    console.error('Failed to delete custom provider:', error);
  } finally {
    isDeletingCustomProvider.value = null;
  }
};

const selectProvider = async (provider: ProviderSpec) => {
  if (selectedProvider.value?.name === provider.name) {
    clearSelectedProvider();
    return;
  }

  await setSelectedProvider(provider);
};

const isModelSaved = (providerName: string, modelName: string) => {
  return localSavedModels.value.some(m => m.provider === providerName && m.model === modelName);
};

const isModelDeletable = (provider: ProviderSpec, modelName: string) => {
  const runtimeCustomModels = runtimeCatalogs.value[provider.name]?.custom_models ?? [];
  const customModels = runtimeCustomModels.length > 0 ? runtimeCustomModels : provider.custom_models;
  return customModels.includes(modelName);
};

const fallbackModelForProvider = (
  provider: ProviderSpec,
  excludedModel?: string,
  catalogOverride?: ProviderModelCatalog
) => {
  const candidates = (catalogOverride?.models ?? providerModelsFor(provider)).filter(
    (model) => model.trim() && model !== excludedModel
  );
  if (provider.default_model && provider.default_model !== excludedModel) {
    return provider.default_model;
  }
  return candidates[0] ?? '';
};

const modelTestKey = (providerName: string, modelName: string) => `${providerName}:${modelName}`;

const modelTestStatusFor = (providerName: string, modelName: string): ModelTestStatus => {
  return modelTestStatuses.value[modelTestKey(providerName, modelName)] ?? {
    state: 'idle',
    message: '',
  };
};

const setModelTestStatus = (
  providerName: string,
  modelName: string,
  status: ModelTestStatus,
  resetMs?: number,
) => {
  const key = modelTestKey(providerName, modelName);
  if (modelTestTimers.has(key)) {
    clearTimeout(modelTestTimers.get(key)!);
    modelTestTimers.delete(key);
  }
  modelTestStatuses.value = {
    ...modelTestStatuses.value,
    [key]: status,
  };
  if (resetMs) {
    modelTestTimers.set(
      key,
      setTimeout(() => {
        modelTestStatuses.value = {
          ...modelTestStatuses.value,
          [key]: { state: 'idle', message: '' },
        };
        modelTestTimers.delete(key);
      }, resetMs)
    );
  }
};

const testStatusTone = (providerName: string, modelName: string) => {
  const status = modelTestStatusFor(providerName, modelName);
  if (status.state === 'success') return 'text-success';
  if (status.state === 'failed') return 'text-danger';
  return 'text-muted';
};

const testStatusLabel = (providerName: string, modelName: string) => {
  const status = modelTestStatusFor(providerName, modelName);
  if (status.state === 'testing') return t('providers.testingConnection');
  if (status.state === 'success') {
    return status.latencyMs != null
      ? t('providers.testSuccessWithLatency', { ms: status.latencyMs })
      : t('providers.testSuccess');
  }
  if (status.state === 'failed') return status.message || t('providers.testFailed');
  return '';
};

const shouldShowTestAction = (providerName: string, modelName: string) => {
  return modelTestStatusFor(providerName, modelName).state !== 'idle';
};

const testModelConnection = async (modelName: string) => {
  if (!selectedProvider.value) return;
  const provider = selectedProvider.value;
  const providerName = provider.name;
  const apiBase = providerApiBases.value[providerName] || provider.default_api_base || null;
  const apiKey = providerApiKeys.value[providerName] || null;

  setModelTestStatus(providerName, modelName, {
    state: 'testing',
    message: '',
  });

  try {
    const result: ProviderModelTestResult = await testProviderModel(
      providerName,
      modelName,
      apiBase,
      apiKey
    );
    setModelTestStatus(
      providerName,
      modelName,
      {
        state: result.ok ? 'success' : 'failed',
        message: result.message,
        latencyMs: result.latency_ms,
      },
      3000
    );
  } catch (error) {
    setModelTestStatus(
      providerName,
      modelName,
      {
        state: 'failed',
        message: error instanceof Error ? error.message : String(error),
      },
      3000
    );
  }
};

const toggleModel = (modelName: string) => {
  if (!selectedProvider.value) return;
  const provider = selectedProvider.value;
  const trimmedModelName = modelName.trim();

  if (
    localConfig.value.provider === provider.name &&
    localConfig.value.model === trimmedModelName
  ) {
    removeCurrentModel();
    return;
  }

  const entry = buildSavedModelEntry(provider, trimmedModelName);
  
  localConfig.value.model = trimmedModelName;
  localConfig.value.provider = provider.name;
  localConfig.value.apiBase = providerApiBases.value[provider.name] || provider.default_api_base;
  localConfig.value.apiKey = entry.apiKey;

  upsertSavedModel(entry);
};

const removeCurrentModel = () => {
  const providerName = localConfig.value.provider?.trim();
  const modelName = localConfig.value.model?.trim();
  if (!providerName || !modelName) return;

  const newSavedModels = localSavedModels.value.filter(
    (savedModel) => !(savedModel.provider === providerName && savedModel.model === modelName)
  );

  localSavedModels.value = newSavedModels;
};

const addManualModel = async () => {
  if (!selectedProvider.value) return;
  const trimmedModelName = manualModelName.value.trim();
  if (!trimmedModelName) return;

  const provider = selectedProvider.value;
  const entry = buildSavedModelEntry(provider, trimmedModelName);

  try {
    const catalog = await addProviderModel(provider.name, trimmedModelName);
    runtimeCatalogs.value = {
      ...runtimeCatalogs.value,
      [provider.name]: catalog,
    };
    providers.value = providers.value.map((item) =>
      item.name === provider.name
        ? {
            ...item,
            models: catalog.models,
            custom_models: catalog.custom_models,
          }
        : item
    );

    localConfig.value.model = trimmedModelName;
    localConfig.value.provider = provider.name;
    localConfig.value.apiBase = providerApiBases.value[provider.name] || provider.default_api_base;
    localConfig.value.apiKey = entry.apiKey;

    upsertSavedModel(entry);
    closeManualModelDialog();
  } catch (error) {
    console.error('Failed to add manual model:', error);
  }
};

const deleteProviderModel = async (modelName: string) => {
  if (!selectedProvider.value) return;
  const provider = selectedProvider.value;
  const trimmedModelName = modelName.trim();
  if (!trimmedModelName || !isModelDeletable(provider, trimmedModelName)) {
    return;
  }
  if (
    localConfig.value.provider === provider.name &&
    localConfig.value.model === trimmedModelName &&
    !fallbackModelForProvider(provider, trimmedModelName)
  ) {
    console.warn('Refusing to delete the active model because no fallback model is available.');
    return;
  }

  try {
    const catalog = await removeProviderModel(provider.name, trimmedModelName);
    runtimeCatalogs.value = {
      ...runtimeCatalogs.value,
      [provider.name]: catalog,
    };
    providers.value = providers.value.map((item) =>
      item.name === provider.name
        ? {
            ...item,
            models: catalog.models,
            custom_models: catalog.custom_models,
          }
        : item
    );

    const newSavedModels = localSavedModels.value.filter(
      (savedModel) => !(savedModel.provider === provider.name && savedModel.model === trimmedModelName)
    );
    if (newSavedModels.length !== localSavedModels.value.length) {
      localSavedModels.value = newSavedModels;
    }

    if (
      localConfig.value.provider === provider.name &&
      localConfig.value.model === trimmedModelName
    ) {
      const nextModel = fallbackModelForProvider(provider, trimmedModelName, catalog);
      if (nextModel) {
        localConfig.value.provider = provider.name;
        localConfig.value.model = nextModel;
        localConfig.value.apiBase =
          providerApiBases.value[provider.name] || provider.default_api_base;
        localConfig.value.apiKey = providerApiKeys.value[provider.name] || '';
      }
    }
  } catch (error) {
    console.error('Failed to delete provider model:', error);
  }
};

const updateProviderKey = (key: string) => {
  if (!selectedProvider.value) return;
  const providerName = selectedProvider.value.name;
  providerApiKeys.value[providerName] = key;
  localConfig.value.apiKey = key;
  
  const newModels = localSavedModels.value.map(m => {
    if (m.provider === providerName) {
      return { ...m, apiKey: key };
    }
    return m;
  });
  if (JSON.stringify(newModels) !== JSON.stringify(localSavedModels.value)) {
    localSavedModels.value = newModels;
  }
};

const updateProviderApiBase = (apiBase: string) => {
  if (!selectedProvider.value) return;
  const providerName = selectedProvider.value.name;
  const normalized = apiBase.trim();
  providerApiBases.value[providerName] = normalized;
  localConfig.value.apiBase = normalized;

  const newModels = localSavedModels.value.map((model) => {
    if (model.provider === providerName) {
      return { ...model, apiBase: normalized };
    }
    return model;
  });
  if (JSON.stringify(newModels) !== JSON.stringify(localSavedModels.value)) {
    localSavedModels.value = newModels;
  }
};

const saveProviderConfig = async () => {
  if (isSavingConfig.value || !isDirty.value) return;
  isSavingConfig.value = true;
  try {
    await props.saveConfigAction({ ...localConfig.value });
    emit('update-saved-models', cloneSavedModels(localSavedModels.value));
    lastSavedConfigSnapshot.value = JSON.stringify(localConfig.value);
    lastSavedModelsSnapshot.value = JSON.stringify(localSavedModels.value);
    statusReport.value = await getConfigStatus();
  } finally {
    isSavingConfig.value = false;
  }
};

watch(() => props.config, async (newVal) => {
  localConfig.value = { ...newVal };
  lastSavedConfigSnapshot.value = JSON.stringify(newVal);
  buildProviderStateFromDraft();
  await refreshProviderState();
}, { deep: true });

watch(() => props.providerConfigs, () => {
  buildProviderStateFromDraft();
}, { deep: true });

watch(() => props.savedModels, (newVal) => {
  localSavedModels.value = cloneSavedModels(newVal || []);
  lastSavedModelsSnapshot.value = JSON.stringify(newVal || []);
  buildProviderStateFromDraft();
}, { deep: true });
</script>

<template>
  <div class="flex h-full min-h-0 fade-in">
    <!-- Sidebar: List of Providers -->
    <div class="providers-sidebar w-1/3 min-w-[200px] min-h-0 flex flex-col">
      <div class="providers-sidebar-header">
        <div class="space-y-3">
          <input
            v-model="searchTerm"
            :placeholder="t('providers.search')"
            class="providers-input"
          />
          <button
            type="button"
            class="providers-add-btn"
            @click="openCreateProviderDialog"
          >
            <Plus :size="14" />
            <span>{{ t('providers.createProviderAction') }}</span>
          </button>
        </div>
      </div>

      <div class="providers-list">
          <div
            v-for="provider in filteredProviders"
            :key="provider.name"
            class="providers-list-item"
            :class="{ selected: selectedProvider?.name === provider.name }"
          >
            <button
              type="button"
              class="flex min-w-0 items-center px-4 py-3 text-left"
              @click="selectProvider(provider)"
            >
              <div class="flex min-w-0 flex-1 items-center">
                <div class="providers-item-icon" :class="{ selected: selectedProvider?.name === provider.name }">
                  <Server :size="16" />
                </div>
                <div class="min-w-0">
                  <div class="font-medium flex items-center gap-2">
                    <span class="truncate">{{ provider.display_name }}</span>
                  </div>
                  <div class="text-[10px] uppercase tracking-wider opacity-70 flex flex-wrap items-center gap-1 providers-tag api-type">
                    <span>{{ provider.api_type || t('providers.standardApi') }}</span>
                    <span v-if="providerStatusMap.get(provider.name)?.current" class="providers-tag current">{{ t('providers.currentTag') }}</span>
                  </div>
                </div>
              </div>
            </button>
            <div
              v-if="provider.source === 'custom'"
              class="relative z-10 flex shrink-0 items-center pr-2"
            >
              <button
                type="button"
                class="providers-delete-btn"
                :title="t('providers.deleteProvider')"
                :disabled="isDeletingCustomProvider === provider.name"
                @click.stop="requestDeleteCustomProvider(provider)"
              >
                <LoaderCircle
                  v-if="isDeletingCustomProvider === provider.name"
                  :size="14"
                  class="animate-spin"
                />
                <Trash2 v-else :size="14" />
              </button>
            </div>
          </div>
      </div>
    </div>

    <!-- Main Area -->
    <div class="providers-main flex-1 min-h-0 overflow-y-auto p-6">
      <div v-if="selectedProvider" class="space-y-8">
        <!-- Header -->
        <div class="flex items-start justify-between gap-4">
            <div class="flex items-center space-x-4">
                <div class="providers-header-icon">
                    <Server :size="24" />
                </div>
                <div>
                    <h3 class="text-xl font-bold settings-label">{{ selectedProvider.display_name }}</h3>
                    <p class="text-sm settings-muted">{{ selectedProvider.default_api_base || t('providers.customApi') }}</p>
                </div>
            </div>
            <div class="flex items-center gap-3">
              <div
                v-if="statusReport"
                class="providers-status-badge"
                :class="statusReport.doctor.ready ? 'ready' : 'warning'"
              >
                <ShieldCheck v-if="statusReport.doctor.ready" :size="14" />
                <ShieldAlert v-else :size="14" />
                <span>{{ statusReport.doctor.ready ? t('providers.healthReady') : t('providers.healthAttention') }}</span>
              </div>
              <button
                type="button"
                class="btn-save-config settings-btn"
                :disabled="isSavingConfig || !isDirty"
                @click="saveProviderConfig"
              >
                <LoaderCircle v-if="isSavingConfig" :size="16" class="animate-spin" />
                <span>{{ isSavingConfig ? t('console.saving') : t('console.saveConfig') }}</span>
              </button>
            </div>
        </div>

        <div
          v-if="statusReport"
          class="grid grid-cols-1 md:grid-cols-2 gap-3"
        >
          <div class="providers-config-card">
            <div class="text-[11px] uppercase tracking-wider settings-muted">{{ t('providers.currentProvider') }}</div>
            <div class="mt-1 text-sm font-semibold settings-label">{{ currentProviderLabel }}</div>
            <div class="mt-1 text-xs settings-muted">{{ statusReport.default_model }}</div>
          </div>
          <div class="providers-config-card">
            <div class="text-[11px] uppercase tracking-wider settings-muted">{{ t('providers.resolvedWorkspace') }}</div>
            <div class="mt-1 text-xs font-mono settings-label break-all">{{ statusReport.config.workspace }}</div>
          </div>
        </div>

        <!-- Configuration -->
        <div class="providers-config-card">
           <h4 class="font-semibold settings-label text-sm mb-4">{{ t('providers.connectConfig') }}</h4>

           <div
             v-if="providerStatusMap.get(selectedProvider.name)"
             class="grid grid-cols-1 md:grid-cols-3 gap-3 mb-4"
           >
             <div class="providers-config-card">
               <div class="text-[11px] uppercase tracking-wider settings-muted">{{ t('providers.readiness') }}</div>
               <div class="mt-1 text-sm font-semibold" :class="providerStatusMap.get(selectedProvider.name)?.ready ? 'text-emerald-600' : 'text-amber-600'">
                 {{ providerStatusMap.get(selectedProvider.name)?.ready ? t('providers.ready') : t('providers.missingConfig') }}
               </div>
             </div>
             <div class="providers-config-card">
               <div class="text-[11px] uppercase tracking-wider settings-muted">{{ t('providers.currentModel') }}</div>
               <div class="mt-1 text-sm font-semibold settings-label">
                 {{ providerStatusMap.get(selectedProvider.name)?.model || providerStatusMap.get(selectedProvider.name)?.default_model || '-' }}
               </div>
             </div>
             <div class="providers-config-card">
               <div class="text-[11px] uppercase tracking-wider settings-muted">{{ t('providers.missingFields') }}</div>
               <div class="mt-1 text-xs settings-muted">
                 {{ providerStatusMap.get(selectedProvider.name)?.missing_fields.length ? providerStatusMap.get(selectedProvider.name)?.missing_fields.join(', ') : t('providers.none') }}
               </div>
            </div>
           </div>

           <!-- API Key -->
           <div class="space-y-1">
             <label class="block text-xs font-medium settings-muted uppercase tracking-wider">{{ t('providers.apiKey') }}</label>
             <div class="relative">
               <input
                 :value="providerApiKeys[selectedProvider.name]"
                 @input="e => updateProviderKey((e.target as HTMLInputElement).value)"
                 :type="isProviderApiKeyVisible(selectedProvider.name) ? 'text' : 'password'"
                 :placeholder="`${t('providers.enterApiKey')} (${selectedProvider.display_name})`"
                 class="providers-input mono pr-11"
               />
               <button
                 type="button"
                 class="absolute inset-y-0 right-0 inline-flex w-10 items-center justify-center rounded-r-lg settings-muted transition"
                 :title="isProviderApiKeyVisible(selectedProvider.name) ? t('providers.hideApiKey') : t('providers.showApiKey')"
                 @click="toggleProviderApiKeyVisibility(selectedProvider.name)"
               >
                 <EyeOff v-if="isProviderApiKeyVisible(selectedProvider.name)" :size="16" />
                 <Eye v-else :size="16" />
               </button>
             </div>
           </div>

           <!-- API Base -->
           <div class="space-y-1">
             <label class="block text-xs font-medium settings-muted uppercase tracking-wider">{{ t('providers.apiBaseUrl') }}</label>
             <input
               :value="providerApiBases[selectedProvider.name] || selectedProvider.default_api_base || ''"
               @input="e => updateProviderApiBase((e.target as HTMLInputElement).value)"
               :placeholder="selectedProvider.default_api_base || t('providers.placeholderLocalCustom')"
               class="providers-input mono"
             />
           </div>
        </div>

        <!-- Model Selection -->
        <div class="space-y-4">
          <div class="flex items-center justify-between gap-3">
            <h4 class="font-semibold settings-label text-sm">
              {{ t('providers.availableModels') }}
            </h4>
            <div class="flex items-center gap-2">
              <input
                v-model="modelSearchTerm"
                type="text"
                :placeholder="t('providers.searchModelPlaceholder')"
                class="providers-input h-8 w-52 text-xs"
              />
              <button
                type="button"
                class="settings-btn settings-btn-secondary h-8 px-2"
                :title="t('providers.manualModelTitle')"
                @click="openManualModelDialog"
              >
                <Plus :size="14" />
              </button>
              <button
                type="button"
                class="settings-btn settings-btn-secondary text-xs h-8"
                :disabled="isRefreshing"
                @click="refreshSelectedProviderModels"
              >
                <RefreshCcw :size="14" :class="isRefreshing ? 'animate-spin' : ''" />
                <span>{{ t('providers.refreshModels') }}</span>
              </button>
            </div>
          </div>
          <div class="text-xs settings-muted">
            {{ t('providers.checkToAdd') }}
          </div>

          <div class="grid grid-cols-1 gap-2">
            <div
              v-for="model in providerModels"
              :key="model"
              @click="toggleModel(model)"
              class="providers-model-card"
              :class="{ selected: isModelSaved(selectedProvider.name, model) }"
            >
              <div class="flex items-center justify-between">
                <div class="flex items-center space-x-3">
                    <div class="providers-checkbox" :class="{ checked: isModelSaved(selectedProvider.name, model) }">
                        <Check v-if="isModelSaved(selectedProvider.name, model)" :size="12" />
                    </div>
                    <span class="font-medium text-sm settings-label">{{ model }}</span>
                </div>

                <div class="flex items-center gap-2">
                  <span v-if="localConfig.model === model && localConfig.provider === selectedProvider.name" class="providers-tag current">
                    {{ t('providers.currentTag') }}
                  </span>
                  <span
                    v-if="testStatusLabel(selectedProvider.name, model)"
                    class="max-w-56 truncate rounded-full px-2 py-0.5 text-[10px] font-semibold"
                    :class="testStatusTone(selectedProvider.name, model)"
                    :title="modelTestStatusFor(selectedProvider.name, model).message"
                  >
                    {{ testStatusLabel(selectedProvider.name, model) }}
                  </span>
                  <button
                    type="button"
                    class="settings-btn settings-btn-secondary h-7 w-7 !p-0"
                    :class="shouldShowTestAction(selectedProvider.name, model) ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'"
                    :disabled="modelTestStatusFor(selectedProvider.name, model).state === 'testing'"
                    :title="t('providers.testConnection')"
                    @click.stop="testModelConnection(model)"
                  >
                    <LoaderCircle
                      v-if="modelTestStatusFor(selectedProvider.name, model).state === 'testing'"
                      :size="14"
                      class="animate-spin"
                    />
                    <Check
                      v-else-if="modelTestStatusFor(selectedProvider.name, model).state === 'success'"
                      :size="14"
                    />
                    <CircleAlert
                      v-else-if="modelTestStatusFor(selectedProvider.name, model).state === 'failed'"
                      :size="14"
                    />
                    <PlugZap v-else :size="14" />
                  </button>
                  <button
                    v-if="isModelDeletable(selectedProvider, model)"
                    type="button"
                    class="providers-delete-btn"
                    :title="t('providers.deleteModel')"
                    @click.stop="deleteProviderModel(model)"
                  >
                    <Trash2 :size="14" />
                  </button>
                </div>
              </div>
            </div>
          </div>

          <div v-if="providerModels.length === 0" class="text-center py-8 settings-muted text-sm">
              {{ modelSearchTerm.trim() ? t('providers.noSearchResults') : t('providers.noModels') }}
          </div>
        </div>
      </div>
      <div v-else class="h-full flex flex-col items-center justify-center settings-muted space-y-4">
        <Cpu :size="48" class="opacity-20" />
        <p>{{ t('providers.selectProvider') }}</p>
      </div>

      <!-- Manual Model Dialog -->
      <div
        v-if="selectedProvider && isManualModelDialogOpen"
        class="providers-dialog-overlay"
        @click.self="closeManualModelDialog"
      >
        <div class="providers-dialog max-w-md">
          <div>
            <div class="text-base font-semibold settings-label">{{ t('providers.manualModelTitle') }}</div>
            <div class="mt-1 text-sm settings-muted">{{ t('providers.manualModelHint') }}</div>
          </div>
          <div class="mt-4">
            <input
              v-model="manualModelName"
              type="text"
              :placeholder="t('providers.manualModelPlaceholder')"
              class="providers-input"
              @keydown.enter.prevent="addManualModel"
            />
          </div>
          <div class="mt-5 flex justify-end gap-2">
            <button
              type="button"
              class="settings-btn settings-btn-secondary"
              @click="closeManualModelDialog"
            >
              {{ t('mcp.cancel') }}
            </button>
            <button
              type="button"
              class="settings-btn settings-btn-primary"
              :disabled="manualModelName.trim().length === 0"
              @click="addManualModel"
            >
              {{ t('providers.manualModelAdd') }}
            </button>
          </div>
        </div>
      </div>

      <!-- Create Provider Dialog -->
      <div
        v-if="isCreateProviderDialogOpen"
        class="providers-dialog-overlay"
        @click.self="closeCreateProviderDialog"
      >
        <div class="providers-dialog max-w-lg">
          <div>
            <div class="text-base font-semibold settings-label">{{ t('providers.createProviderTitle') }}</div>
            <div class="mt-1 text-sm settings-muted">{{ t('providers.createProviderHint') }}</div>
          </div>
          <div class="mt-4 grid grid-cols-1 gap-4 md:grid-cols-2">
            <label class="space-y-1 md:col-span-1">
              <div class="text-xs font-medium uppercase tracking-wider settings-muted">{{ t('providers.providerId') }}</div>
              <input
                v-model="newProviderForm.id"
                type="text"
                :placeholder="t('providers.providerIdPlaceholder')"
                class="providers-input"
              />
            </label>
            <label class="space-y-1 md:col-span-1">
              <div class="text-xs font-medium uppercase tracking-wider settings-muted">{{ t('providers.providerDisplayName') }}</div>
              <input
                v-model="newProviderForm.displayName"
                type="text"
                :placeholder="t('providers.providerDisplayNamePlaceholder')"
                class="providers-input"
              />
            </label>
            <label class="space-y-1 md:col-span-2">
              <div class="text-xs font-medium uppercase tracking-wider settings-muted">{{ t('providers.apiBaseUrl') }}</div>
              <input
                v-model="newProviderForm.apiBase"
                type="text"
                :placeholder="t('providers.createProviderApiBasePlaceholder')"
                class="providers-input"
              />
            </label>
            <label class="space-y-1 md:col-span-2">
              <div class="text-xs font-medium uppercase tracking-wider settings-muted">{{ t('providers.apiKey') }}</div>
              <div class="relative">
                <input
                  v-model="newProviderForm.apiKey"
                  :type="isCreateProviderApiKeyVisible ? 'text' : 'password'"
                  :placeholder="t('providers.enterApiKey')"
                  class="providers-input pr-11"
                />
                <button
                  type="button"
                  class="absolute inset-y-0 right-0 inline-flex w-10 items-center justify-center rounded-r-lg settings-muted transition"
                  :title="isCreateProviderApiKeyVisible ? t('providers.hideApiKey') : t('providers.showApiKey')"
                  @click="isCreateProviderApiKeyVisible = !isCreateProviderApiKeyVisible"
                >
                  <EyeOff v-if="isCreateProviderApiKeyVisible" :size="16" />
                  <Eye v-else :size="16" />
                </button>
              </div>
            </label>
            <label class="space-y-1 md:col-span-2">
              <div class="text-xs font-medium uppercase tracking-wider settings-muted">{{ t('providers.defaultModel') }}</div>
              <input
                v-model="newProviderForm.defaultModel"
                type="text"
                :placeholder="t('providers.defaultModelPlaceholder')"
                class="providers-input"
                @keydown.enter.prevent="createProvider"
              />
            </label>
          </div>
          <div class="mt-5 flex justify-end gap-2">
            <button
              type="button"
              class="settings-btn settings-btn-secondary"
              @click="closeCreateProviderDialog"
            >
              {{ t('mcp.cancel') }}
            </button>
            <button
              type="button"
              class="settings-btn settings-btn-primary"
              :disabled="isSavingProvider || !newProviderForm.id.trim() || !newProviderForm.displayName.trim() || !newProviderForm.apiBase.trim() || !newProviderForm.defaultModel.trim()"
              @click="createProvider"
            >
              {{ isSavingProvider ? t('providers.saving') : t('providers.createProviderAction') }}
            </button>
          </div>
        </div>
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

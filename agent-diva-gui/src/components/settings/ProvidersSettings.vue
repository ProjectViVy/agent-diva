<script setup lang="ts">
import { ref, onMounted, computed, watch, onBeforeUnmount } from 'vue';
import { Server, Check, Cpu, ShieldCheck, ShieldAlert, RefreshCcw, Plus, Trash2, PlugZap, LoaderCircle, CircleAlert } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import {
  type ConfigStatusReport,
  getConfigStatus,
} from '../../api/desktop';
import {
  addProviderModel,
  createCustomProvider,
  deleteCustomProvider,
  getProviderAuthStatus,
  getProviderLoginStatus,
  getProviders,
  loginProvider,
  logoutProvider,
  getProviderModels,
  removeProviderModel,
  refreshProviderAuth,
  testProviderModel,
  type PendingProviderLoginStatusDto,
  type ProviderAuthStatusDto,
  type ProviderModelCatalog,
  type ProviderModelTestResult,
  type ProviderSpecDto,
  useProviderProfile,
} from '../../api/providers';
import { appConfirm } from '../../utils/appDialog';
import { openExternalUrl } from '../../utils/openExternal';

const { t } = useI18n();

type ProviderSpec = ProviderSpecDto;

interface SavedModel {
  id: string;
  provider: string;
  model: string;
  apiBase: string;
  apiKey: string;
  displayName: string;
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
  savedModels?: SavedModel[];
}>();

const emit = defineEmits<{
  (e: 'save', config: typeof props.config): void;
  (e: 'update-saved-models', models: SavedModel[]): void;
}>();

const providers = ref<ProviderSpec[]>([]);
const statusReport = ref<ConfigStatusReport | null>(null);
const localConfig = ref({ ...props.config });
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
const isRefreshing = ref(false);
const runtimeCatalogs = ref<Record<string, ProviderModelCatalog>>({});
const modelTestStatuses = ref<Record<string, ModelTestStatus>>({});
const providerAuthStatuses = ref<Record<string, ProviderAuthStatusDto>>({});
const providerAuthBusy = ref<Record<string, string>>({});
const providerAuthMessages = ref<Record<string, string>>({});
const providerAuthErrors = ref<Record<string, string>>({});
const providerProfileDrafts = ref<Record<string, string>>({});
const providerRedirectInputs = ref<Record<string, string>>({});
const modelTestTimers = new Map<string, ReturnType<typeof setTimeout>>();
const providerLoginTimers = new Map<string, ReturnType<typeof setTimeout>>();
const newProviderForm = ref<NewProviderForm>({
  id: '',
  displayName: '',
  apiBase: '',
  apiKey: '',
  defaultModel: '',
});

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

const buildProviderStateFromProps = () => {
  if (props.savedModels) {
    props.savedModels.forEach((model) => {
      if (model.apiKey) {
        providerApiKeys.value[model.provider] = model.apiKey;
      }
      if (model.apiBase) {
        providerApiBases.value[model.provider] = model.apiBase;
      }
    });
  }

  if (props.config.provider) {
    if (props.config.apiKey) {
      providerApiKeys.value[props.config.provider] = props.config.apiKey;
    }
    if (props.config.apiBase) {
      providerApiBases.value[props.config.provider] = props.config.apiBase;
    }
  }
};

const providerModelsFor = (provider: ProviderSpec | null) => {
  if (!provider) return [];
  return runtimeCatalogs.value[provider.name]?.models ?? provider.models;
};

const providerAuthStatusFor = (providerName: string) =>
  providerAuthStatuses.value[providerName] ?? null;

const selectedProviderAuthStatus = computed(() => {
  if (!selectedProvider.value) return null;
  return providerAuthStatusFor(selectedProvider.value.name);
});

const selectedProviderUsesExternalAuth = computed(() => {
  return (
    !!selectedProvider.value?.login_supported &&
    selectedProvider.value?.credential_store === 'external_secure_store'
  );
});

const selectedProviderProfileDraft = computed({
  get: () => {
    if (!selectedProvider.value) return 'default';
    return providerProfileDrafts.value[selectedProvider.value.name] || 'default';
  },
  set: (value: string) => {
    if (!selectedProvider.value) return;
    providerProfileDrafts.value = {
      ...providerProfileDrafts.value,
      [selectedProvider.value.name]: value.trim() || 'default',
    };
  },
});

const refreshProviderState = async () => {
  isRefreshing.value = true;
  try {
    providers.value = dedupeProviders(await getProviders());
    statusReport.value = await getConfigStatus();
    buildProviderStateFromProps();

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

const updateProviderAuthSnapshot = (providerName: string, status: ProviderAuthStatusDto) => {
  providerAuthStatuses.value = {
    ...providerAuthStatuses.value,
    [providerName]: status,
  };
  providers.value = providers.value.map((provider) =>
    provider.name === providerName
      ? {
          ...provider,
          authenticated: status.authenticated,
          active_profile: status.active_profile,
          expires_at: status.expires_at,
          profiles: status.profiles,
        }
      : provider
  );
  if (selectedProvider.value?.name === providerName) {
    selectedProvider.value = {
      ...selectedProvider.value,
      authenticated: status.authenticated,
      active_profile: status.active_profile,
      expires_at: status.expires_at,
      profiles: status.profiles,
    };
  }
  if (!providerProfileDrafts.value[providerName]) {
    providerProfileDrafts.value = {
      ...providerProfileDrafts.value,
      [providerName]: status.active_profile || status.profiles[0]?.profile_name || 'default',
    };
  }
};

const setProviderAuthBusy = (providerName: string, action: string | null) => {
  providerAuthBusy.value = {
    ...providerAuthBusy.value,
    [providerName]: action || '',
  };
};

const clearProviderFeedback = (providerName: string) => {
  providerAuthMessages.value = { ...providerAuthMessages.value, [providerName]: '' };
  providerAuthErrors.value = { ...providerAuthErrors.value, [providerName]: '' };
};

const refreshProviderAuthState = async (providerName: string) => {
  const provider = providers.value.find((item) => item.name === providerName);
  if (!provider?.login_supported) return;
  const status = await getProviderAuthStatus(providerName);
  updateProviderAuthSnapshot(providerName, status);
};

const stopProviderLoginPolling = (providerName: string) => {
  const timer = providerLoginTimers.get(providerName);
  if (timer) {
    clearTimeout(timer);
    providerLoginTimers.delete(providerName);
  }
};

const scheduleProviderLoginPolling = (providerName: string, profileName: string) => {
  stopProviderLoginPolling(providerName);
  providerLoginTimers.set(
    providerName,
    setTimeout(async () => {
      try {
        const status: PendingProviderLoginStatusDto = await getProviderLoginStatus(
          providerName,
          profileName
        );
        if (status.status === 'pending') {
          scheduleProviderLoginPolling(providerName, profileName);
          return;
        }
        if (status.status === 'completed') {
          await refreshProviderAuthState(providerName);
          providerAuthMessages.value = {
            ...providerAuthMessages.value,
            [providerName]: `Logged in as ${profileName}.`,
          };
          providerRedirectInputs.value = {
            ...providerRedirectInputs.value,
            [providerName]: '',
          };
          return;
        }
        if (status.status === 'failed') {
          providerAuthErrors.value = {
            ...providerAuthErrors.value,
            [providerName]: status.message || 'Provider login failed.',
          };
        }
      } catch (error) {
        providerAuthErrors.value = {
          ...providerAuthErrors.value,
          [providerName]: error instanceof Error ? error.message : String(error),
        };
      }
    }, 1500)
  );
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
  if (!statusReport.value) return 'bg-gray-100 text-gray-600';
  return statusReport.value.doctor.ready
    ? 'bg-emerald-100 text-emerald-700'
    : 'bg-amber-100 text-amber-700';
});

const filteredProviders = computed(() => {
  if (!searchTerm.value) return providers.value;
  const lower = searchTerm.value.toLowerCase();
  return providers.value.filter(p => 
    p.display_name.toLowerCase().includes(lower) || 
    p.name.toLowerCase().includes(lower)
  );
});

const emitConfigSave = () => {
  emit('save', { ...localConfig.value });
};

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
  const existingIndex = props.savedModels?.findIndex(
    (model) => model.provider === entry.provider && model.model === entry.model
  ) ?? -1;
  const newSavedModels = [...(props.savedModels || [])];

  if (existingIndex >= 0) {
    newSavedModels.splice(existingIndex, 1, {
      ...newSavedModels[existingIndex],
      ...entry
    });
  } else {
    newSavedModels.push(entry);
  }

  emit('update-saved-models', newSavedModels);
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
  if (provider.login_supported) {
    await refreshProviderAuthState(provider.name);
  }
};

const startBrowserProviderLogin = async () => {
  if (!selectedProvider.value) return;
  const providerName = selectedProvider.value.name;
  const profileName = selectedProviderProfileDraft.value.trim() || 'default';
  clearProviderFeedback(providerName);
  setProviderAuthBusy(providerName, 'login');
  try {
    const response = await loginProvider(providerName, profileName, 'browser');
    if (response.authorize_url) {
      await openExternalUrl(response.authorize_url);
    }
    providerAuthMessages.value = {
      ...providerAuthMessages.value,
      [providerName]:
        'Browser authorization started. If loopback fails, paste the final redirect URL below.',
    };
    scheduleProviderLoginPolling(providerName, profileName);
  } catch (error) {
    providerAuthErrors.value = {
      ...providerAuthErrors.value,
      [providerName]: error instanceof Error ? error.message : String(error),
    };
  } finally {
    setProviderAuthBusy(providerName, null);
  }
};

const completeProviderLoginWithRedirect = async () => {
  if (!selectedProvider.value) return;
  const providerName = selectedProvider.value.name;
  const profileName = selectedProviderProfileDraft.value.trim() || 'default';
  const redirectUrl = providerRedirectInputs.value[providerName]?.trim();
  if (!redirectUrl) return;
  clearProviderFeedback(providerName);
  setProviderAuthBusy(providerName, 'complete');
  try {
    await loginProvider(providerName, profileName, 'paste_redirect', redirectUrl);
    await refreshProviderAuthState(providerName);
    providerAuthMessages.value = {
      ...providerAuthMessages.value,
      [providerName]: `Logged in as ${profileName}.`,
    };
    providerRedirectInputs.value = {
      ...providerRedirectInputs.value,
      [providerName]: '',
    };
  } catch (error) {
    providerAuthErrors.value = {
      ...providerAuthErrors.value,
      [providerName]: error instanceof Error ? error.message : String(error),
    };
  } finally {
    setProviderAuthBusy(providerName, null);
  }
};

const activateProviderProfile = async (profileName: string) => {
  if (!selectedProvider.value) return;
  const providerName = selectedProvider.value.name;
  clearProviderFeedback(providerName);
  setProviderAuthBusy(providerName, 'use');
  try {
    const status = await useProviderProfile(providerName, profileName);
    updateProviderAuthSnapshot(providerName, status);
    providerAuthMessages.value = {
      ...providerAuthMessages.value,
      [providerName]: `Active profile set to ${profileName}.`,
    };
  } catch (error) {
    providerAuthErrors.value = {
      ...providerAuthErrors.value,
      [providerName]: error instanceof Error ? error.message : String(error),
    };
  } finally {
    setProviderAuthBusy(providerName, null);
  }
};

const refreshSelectedProviderAuth = async () => {
  if (!selectedProvider.value) return;
  const providerName = selectedProvider.value.name;
  clearProviderFeedback(providerName);
  setProviderAuthBusy(providerName, 'refresh');
  try {
    const status = await refreshProviderAuth(providerName, selectedProviderProfileDraft.value);
    updateProviderAuthSnapshot(providerName, status);
    providerAuthMessages.value = {
      ...providerAuthMessages.value,
      [providerName]: 'OAuth token refreshed.',
    };
  } catch (error) {
    providerAuthErrors.value = {
      ...providerAuthErrors.value,
      [providerName]: error instanceof Error ? error.message : String(error),
    };
  } finally {
    setProviderAuthBusy(providerName, null);
  }
};

const logoutSelectedProviderAuth = async () => {
  if (!selectedProvider.value) return;
  const providerName = selectedProvider.value.name;
  clearProviderFeedback(providerName);
  setProviderAuthBusy(providerName, 'logout');
  try {
    const status = await logoutProvider(providerName, selectedProviderProfileDraft.value);
    updateProviderAuthSnapshot(providerName, status);
    providerAuthMessages.value = {
      ...providerAuthMessages.value,
      [providerName]: 'Provider profile removed from local auth store.',
    };
  } catch (error) {
    providerAuthErrors.value = {
      ...providerAuthErrors.value,
      [providerName]: error instanceof Error ? error.message : String(error),
    };
  } finally {
    setProviderAuthBusy(providerName, null);
  }
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
    emitConfigSave();
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

    emit(
      'update-saved-models',
      (props.savedModels || []).filter((m) => m.provider !== id),
    );

    await refreshProviderState();

    if (!providers.value.some((p) => p.name === localConfig.value.provider)) {
      const next = providers.value[0];
      if (next) {
        localConfig.value.provider = next.name;
        localConfig.value.model = next.default_model || next.models[0] || '';
        localConfig.value.apiBase =
          providerApiBases.value[next.name] || next.default_api_base;
        localConfig.value.apiKey = providerApiKeys.value[next.name] || '';
        emitConfigSave();
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
  return props.savedModels?.some(m => m.provider === providerName && m.model === modelName) ?? false;
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
  if (status.state === 'success') return 'text-emerald-700 bg-emerald-50';
  if (status.state === 'failed') return 'text-rose-700 bg-rose-50';
  return 'text-gray-500';
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
  emitConfigSave();
};

const removeCurrentModel = () => {
  const providerName = localConfig.value.provider?.trim();
  const modelName = localConfig.value.model?.trim();
  if (!providerName || !modelName) return;

  const newSavedModels = (props.savedModels || []).filter(
    (savedModel) => !(savedModel.provider === providerName && savedModel.model === modelName)
  );

  emit('update-saved-models', newSavedModels);
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
    emitConfigSave();
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

    const newSavedModels = (props.savedModels || []).filter(
      (savedModel) => !(savedModel.provider === provider.name && savedModel.model === trimmedModelName)
    );
    if (newSavedModels.length !== (props.savedModels || []).length) {
      emit('update-saved-models', newSavedModels);
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
      emitConfigSave();
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
  emitConfigSave();
};

const updateProviderApiBase = (apiBase: string) => {
  if (!selectedProvider.value) return;
  const providerName = selectedProvider.value.name;
  const normalized = apiBase.trim();
  providerApiBases.value[providerName] = normalized;
  localConfig.value.apiBase = normalized;

  if (props.savedModels) {
    const newModels = props.savedModels.map((model) => {
      if (model.provider === providerName) {
        return { ...model, apiBase: normalized };
      }
      return model;
    });
    if (JSON.stringify(newModels) !== JSON.stringify(props.savedModels)) {
      emit('update-saved-models', newModels);
    }
  }

  emitConfigSave();
};

watch(() => props.config, async (newVal) => {
  localConfig.value = { ...newVal };
  buildProviderStateFromProps();
  await refreshProviderState();
}, { deep: true });

onBeforeUnmount(() => {
  for (const timer of providerLoginTimers.values()) {
    clearTimeout(timer);
  }
  providerLoginTimers.clear();
});
</script>

<template>
  <div class="flex h-full min-h-0 fade-in">
    <!-- Sidebar: List of Providers -->
    <div class="w-1/3 min-w-[200px] min-h-0 border-r border-gray-100 flex flex-col bg-gray-50/30">
      <div class="p-4 border-b border-gray-100">
        <div class="space-y-3">
          <input 
            v-model="searchTerm" 
            :placeholder="t('providers.search')" 
            class="w-full px-4 py-2 bg-white border border-gray-200 rounded-lg text-sm focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all"
          />
          <button
            type="button"
            class="flex w-full items-center justify-center gap-2 rounded-lg border border-dashed border-pink-300 bg-pink-50 px-4 py-2 text-sm font-semibold text-pink-700 transition hover:border-pink-400 hover:bg-pink-100"
            @click="openCreateProviderDialog"
          >
            <Plus :size="14" />
            <span>{{ t('providers.createProviderAction') }}</span>
          </button>
        </div>
      </div>
      
      <div class="flex-1 overflow-y-auto p-2 space-y-1">
          <div
            v-for="provider in filteredProviders"
            :key="provider.name"
            class="grid w-full min-w-0 grid-cols-[minmax(0,1fr)_auto] items-stretch rounded-xl transition-all group"
            :class="selectedProvider?.name === provider.name ? 'bg-white shadow-sm border-l-4 border-pink-500 text-pink-700' : 'hover:bg-gray-100 text-gray-600 border-l-4 border-transparent'"
          >
            <button
              type="button"
              class="flex min-w-0 items-center px-4 py-3 text-left"
              @click="selectProvider(provider)"
            >
              <div class="flex min-w-0 flex-1 items-center">
                <div class="w-8 h-8 shrink-0 rounded-lg flex items-center justify-center mr-3" :class="selectedProvider?.name === provider.name ? 'bg-pink-100 text-pink-600' : 'bg-gray-200 text-gray-500'">
                  <Server :size="16" />
                </div>
                <div class="min-w-0">
                  <div class="font-medium flex items-center gap-2">
                    <span class="truncate">{{ provider.display_name }}</span>
                  </div>
                  <div class="text-[10px] uppercase tracking-wider opacity-70 flex flex-wrap items-center gap-1 text-blue-600">
                    <span>{{ provider.api_type || t('providers.standardApi') }}</span>
                    <span v-if="providerStatusMap.get(provider.name)?.current" class="text-pink-600">{{ t('providers.currentTag') }}</span>
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
                class="inline-flex h-8 w-8 items-center justify-center rounded-lg border border-transparent text-gray-400 transition hover:border-red-200 hover:bg-red-50 hover:text-red-600 disabled:cursor-not-allowed disabled:opacity-50"
                :title="t('providers.deleteProvider')"
                :disabled="isDeletingCustomProvider === provider.name"
                @click.stop="requestDeleteCustomProvider(provider)"
              >
                <LoaderCircle
                  v-if="isDeletingCustomProvider === provider.name"
                  :size="14"
                  class="animate-spin text-rose-500"
                />
                <Trash2 v-else :size="14" />
              </button>
            </div>
          </div>
      </div>
    </div>

    <!-- Main Area -->
    <div class="flex-1 min-h-0 overflow-y-auto p-6 bg-white">
      <div v-if="selectedProvider" class="space-y-8">
        <!-- Header -->
        <div class="flex items-start justify-between gap-4">
            <div class="flex items-center space-x-4">
                <div class="w-12 h-12 bg-gradient-to-br from-pink-500 to-purple-600 rounded-xl flex items-center justify-center text-white shadow-lg shadow-pink-500/30">
                    <Server :size="24" />
                </div>
                <div>
                    <h3 class="text-xl font-bold text-gray-800">{{ selectedProvider.display_name }}</h3>
                    <p class="text-sm text-gray-500">{{ selectedProvider.default_api_base || t('providers.customApi') }}</p>
                </div>
            </div>
            <div
              v-if="statusReport"
              class="px-3 py-2 rounded-xl text-xs font-semibold flex items-center gap-2"
              :class="doctorTone"
            >
              <ShieldCheck v-if="statusReport.doctor.ready" :size="14" />
              <ShieldAlert v-else :size="14" />
              <span>{{ statusReport.doctor.ready ? t('providers.healthReady') : t('providers.healthAttention') }}</span>
            </div>
        </div>

        <div
          v-if="statusReport"
          class="grid grid-cols-1 md:grid-cols-2 gap-3"
        >
          <div class="rounded-xl border border-gray-100 bg-gray-50 p-4">
            <div class="text-[11px] uppercase tracking-wider text-gray-400">{{ t('providers.currentProvider') }}</div>
            <div class="mt-1 text-sm font-semibold text-gray-800">{{ currentProviderLabel }}</div>
            <div class="mt-1 text-xs text-gray-500">{{ statusReport.default_model }}</div>
          </div>
          <div class="rounded-xl border border-gray-100 bg-gray-50 p-4">
            <div class="text-[11px] uppercase tracking-wider text-gray-400">{{ t('providers.resolvedWorkspace') }}</div>
            <div class="mt-1 text-xs font-mono text-gray-700 break-all">{{ statusReport.config.workspace }}</div>
          </div>
        </div>

        <!-- Configuration -->
        <div class="space-y-4 p-4 bg-gray-50 rounded-xl border border-gray-100">
           <h4 class="font-semibold text-gray-700 text-sm">{{ t('providers.connectConfig') }}</h4>

           <div
             v-if="providerStatusMap.get(selectedProvider.name)"
             class="grid grid-cols-1 md:grid-cols-3 gap-3"
           >
             <div class="rounded-lg bg-white border border-gray-200 px-3 py-2">
               <div class="text-[11px] uppercase tracking-wider text-gray-400">{{ t('providers.readiness') }}</div>
               <div
                 class="mt-1 text-sm font-semibold"
                 :class="
                   selectedProvider.login_supported
                     ? (selectedProviderAuthStatus?.authenticated ? 'text-emerald-700' : 'text-amber-700')
                     : (providerStatusMap.get(selectedProvider.name)?.ready ? 'text-emerald-700' : 'text-amber-700')
                 "
               >
                 {{
                   selectedProvider.login_supported
                     ? (selectedProviderAuthStatus?.authenticated ? 'Authenticated' : 'Login required')
                     : (providerStatusMap.get(selectedProvider.name)?.ready ? t('providers.ready') : t('providers.missingConfig'))
                 }}
               </div>
             </div>
             <div class="rounded-lg bg-white border border-gray-200 px-3 py-2">
               <div class="text-[11px] uppercase tracking-wider text-gray-400">
                 {{ selectedProvider.login_supported ? 'Active profile' : t('providers.currentModel') }}
               </div>
               <div class="mt-1 text-sm font-semibold text-gray-800">
                 {{
                   selectedProvider.login_supported
                     ? (selectedProviderAuthStatus?.active_profile || '-')
                     : (providerStatusMap.get(selectedProvider.name)?.model || providerStatusMap.get(selectedProvider.name)?.default_model || '-')
                 }}
               </div>
             </div>
             <div class="rounded-lg bg-white border border-gray-200 px-3 py-2">
               <div class="text-[11px] uppercase tracking-wider text-gray-400">
                 {{ selectedProvider.login_supported ? 'Expires at' : t('providers.missingFields') }}
               </div>
               <div class="mt-1 text-xs text-gray-600">
                 {{
                   selectedProvider.login_supported
                     ? (selectedProviderAuthStatus?.expires_at || '-')
                     : (providerStatusMap.get(selectedProvider.name)?.missing_fields.length ? providerStatusMap.get(selectedProvider.name)?.missing_fields.join(', ') : t('providers.none'))
                 }}
               </div>
            </div>
           </div>

           <div v-if="selectedProviderUsesExternalAuth" class="space-y-4 rounded-xl border border-sky-100 bg-sky-50/60 p-4">
             <div class="flex items-center justify-between gap-3">
               <div>
                 <div class="text-sm font-semibold text-sky-900">Provider Authentication</div>
                 <div class="mt-1 text-xs text-sky-700">
                   {{ selectedProvider.auth_mode }} via {{ selectedProvider.credential_store }}. Credentials are stored outside config.json.
                 </div>
               </div>
               <div
                 class="rounded-full px-2.5 py-1 text-[11px] font-semibold"
                 :class="selectedProviderAuthStatus?.authenticated ? 'bg-emerald-100 text-emerald-700' : 'bg-amber-100 text-amber-700'"
               >
                 {{ selectedProviderAuthStatus?.authenticated ? 'Authenticated' : 'Not logged in' }}
               </div>
             </div>

             <div class="grid grid-cols-1 gap-3 md:grid-cols-[minmax(0,1fr)_auto_auto_auto]">
               <input
                 v-model="selectedProviderProfileDraft"
                 type="text"
                 placeholder="Profile name"
                 class="rounded-lg border border-sky-200 bg-white px-3 py-2 text-sm text-gray-700 outline-none transition focus:border-sky-400 focus:ring-2 focus:ring-sky-500/20"
               />
               <button
                 type="button"
                 class="rounded-lg bg-sky-600 px-3 py-2 text-sm font-semibold text-white transition hover:bg-sky-700 disabled:cursor-not-allowed disabled:opacity-60"
                 :disabled="providerAuthBusy[selectedProvider.name] === 'login'"
                 @click="startBrowserProviderLogin"
               >
                 {{ providerAuthBusy[selectedProvider.name] === 'login' ? 'Starting...' : 'Login' }}
               </button>
               <button
                 type="button"
                 class="rounded-lg border border-sky-200 bg-white px-3 py-2 text-sm font-semibold text-sky-700 transition hover:bg-sky-50 disabled:cursor-not-allowed disabled:opacity-60"
                 :disabled="!selectedProviderAuthStatus?.authenticated || providerAuthBusy[selectedProvider.name] === 'refresh'"
                 @click="refreshSelectedProviderAuth"
               >
                 {{ providerAuthBusy[selectedProvider.name] === 'refresh' ? 'Refreshing...' : 'Refresh' }}
               </button>
               <button
                 type="button"
                 class="rounded-lg border border-rose-200 bg-white px-3 py-2 text-sm font-semibold text-rose-600 transition hover:bg-rose-50 disabled:cursor-not-allowed disabled:opacity-60"
                 :disabled="!selectedProviderAuthStatus?.authenticated || providerAuthBusy[selectedProvider.name] === 'logout'"
                 @click="logoutSelectedProviderAuth"
               >
                 {{ providerAuthBusy[selectedProvider.name] === 'logout' ? 'Removing...' : 'Logout' }}
               </button>
             </div>

             <div class="grid grid-cols-1 gap-2 md:grid-cols-[minmax(0,1fr)_auto]">
               <input
                 v-model="providerRedirectInputs[selectedProvider.name]"
                 type="text"
                 placeholder="Paste redirect URL here if browser callback does not return"
                 class="rounded-lg border border-sky-200 bg-white px-3 py-2 text-sm text-gray-700 outline-none transition focus:border-sky-400 focus:ring-2 focus:ring-sky-500/20"
               />
               <button
                 type="button"
                 class="rounded-lg border border-sky-200 bg-white px-3 py-2 text-sm font-semibold text-sky-700 transition hover:bg-sky-50 disabled:cursor-not-allowed disabled:opacity-60"
                 :disabled="!providerRedirectInputs[selectedProvider.name]?.trim() || providerAuthBusy[selectedProvider.name] === 'complete'"
                 @click="completeProviderLoginWithRedirect"
               >
                 {{ providerAuthBusy[selectedProvider.name] === 'complete' ? 'Completing...' : 'Complete Login' }}
               </button>
             </div>

             <div v-if="selectedProviderAuthStatus?.profiles?.length" class="space-y-2">
               <div class="text-[11px] uppercase tracking-wider text-sky-700">Profiles</div>
               <div class="flex flex-wrap gap-2">
                 <button
                   v-for="profile in selectedProviderAuthStatus.profiles"
                   :key="profile.id"
                   type="button"
                   class="rounded-full border px-3 py-1 text-xs font-medium transition"
                   :class="profile.is_active ? 'border-emerald-200 bg-emerald-100 text-emerald-700' : 'border-sky-200 bg-white text-sky-700 hover:bg-sky-50'"
                   @click="activateProviderProfile(profile.profile_name)"
                 >
                   {{ profile.profile_name }}
                 </button>
               </div>
             </div>

             <div v-if="providerAuthMessages[selectedProvider.name]" class="rounded-lg bg-emerald-50 px-3 py-2 text-xs text-emerald-700">
               {{ providerAuthMessages[selectedProvider.name] }}
             </div>
             <div v-if="providerAuthErrors[selectedProvider.name]" class="rounded-lg bg-rose-50 px-3 py-2 text-xs text-rose-700">
               {{ providerAuthErrors[selectedProvider.name] }}
             </div>
           </div>

           <template v-else>
             <div class="space-y-1">
               <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('providers.apiKey') }}</label>
               <input 
                 :value="providerApiKeys[selectedProvider.name]"
                 @input="e => updateProviderKey((e.target as HTMLInputElement).value)"
                 type="password" 
                 :placeholder="`${t('providers.enterApiKey')} (${selectedProvider.display_name})`"
                 class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" 
               />
             </div>
             
             <div class="space-y-1">
               <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{ t('providers.apiBaseUrl') }}</label>
               <input 
                 :value="providerApiBases[selectedProvider.name] || selectedProvider.default_api_base || ''"
                 @input="e => updateProviderApiBase((e.target as HTMLInputElement).value)"
                 :placeholder="selectedProvider.default_api_base || t('providers.placeholderLocalCustom')"
                 class="w-full px-3 py-2 bg-white border border-gray-200 rounded-lg focus:ring-2 focus:ring-pink-500/20 focus:border-pink-500 outline-none transition-all font-mono text-sm" 
               />
             </div>
           </template>
        </div>

        <!-- Model Selection -->
        <div class="space-y-4">
          <div class="flex items-center justify-between gap-3">
            <h4 class="font-semibold text-gray-700 text-sm">
              {{ t('providers.availableModels') }}
            </h4>
            <div class="flex items-center gap-2">
              <input
                v-model="modelSearchTerm"
                type="text"
                :placeholder="t('providers.searchModelPlaceholder')"
                class="h-8 w-52 rounded-lg border border-gray-200 bg-white px-3 text-xs text-gray-700 outline-none transition focus:border-pink-400 focus:ring-2 focus:ring-pink-500/20"
              />
              <button
                type="button"
                class="inline-flex h-8 w-8 items-center justify-center rounded-lg border border-gray-200 bg-white text-gray-700 transition hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-60"
                :title="t('providers.manualModelTitle')"
                @click="openManualModelDialog"
              >
                <Plus :size="14" />
              </button>
              <button
                type="button"
                class="inline-flex items-center gap-2 rounded-lg border border-gray-200 bg-white px-3 py-1.5 text-xs font-semibold text-gray-700 transition hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-60"
                :disabled="isRefreshing"
                @click="refreshSelectedProviderModels"
              >
                <RefreshCcw :size="14" :class="isRefreshing ? 'animate-spin' : ''" />
                <span>{{ t('providers.refreshModels') }}</span>
              </button>
            </div>
          </div>
          <div class="flex items-center justify-between gap-3">
            <div class="text-xs text-gray-500">
              {{ t('providers.checkToAdd') }}
            </div>
          </div>
          
          <div class="grid grid-cols-1 gap-2">
            <div
              v-for="model in providerModels"
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

              <div class="flex items-center gap-2">
                <span v-if="localConfig.model === model && localConfig.provider === selectedProvider.name" class="text-[10px] bg-green-100 text-green-700 px-2 py-0.5 rounded-full font-bold">
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
                  class="inline-flex h-7 w-7 items-center justify-center rounded-lg border border-transparent text-gray-400 transition hover:border-sky-200 hover:bg-sky-50 hover:text-sky-600"
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
                  class="inline-flex h-7 w-7 items-center justify-center rounded-lg border border-transparent text-gray-400 opacity-0 transition hover:border-red-200 hover:bg-red-50 hover:text-red-600 group-hover:opacity-100"
                  :title="t('providers.deleteModel')"
                  @click.stop="deleteProviderModel(model)"
                >
                  <Trash2 :size="14" />
                </button>
              </div>
            </div>
          </div>

          <div v-if="providerModels.length === 0" class="text-center py-8 text-gray-400 text-sm">
              {{ modelSearchTerm.trim() ? t('providers.noSearchResults') : t('providers.noModels') }}
          </div>
        </div>
      </div>
      <div v-else class="h-full flex flex-col items-center justify-center text-gray-400 space-y-4">
        <Cpu :size="48" class="opacity-20" />
        <p>{{ t('providers.selectProvider') }}</p>
      </div>

      <div
        v-if="selectedProvider && isManualModelDialogOpen"
        class="fixed inset-0 z-40 flex items-center justify-center bg-gray-900/35 px-4"
        @click.self="closeManualModelDialog"
      >
        <div class="w-full max-w-md rounded-2xl border border-gray-200 bg-white p-5 shadow-2xl">
          <div>
            <div class="text-base font-semibold text-gray-800">{{ t('providers.manualModelTitle') }}</div>
            <div class="mt-1 text-sm text-gray-500">{{ t('providers.manualModelHint') }}</div>
          </div>
          <div class="mt-4">
            <input
              v-model="manualModelName"
              type="text"
              :placeholder="t('providers.manualModelPlaceholder')"
              class="w-full rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm outline-none transition focus:border-pink-400 focus:ring-2 focus:ring-pink-500/20"
              @keydown.enter.prevent="addManualModel"
            />
          </div>
          <div class="mt-5 flex justify-end gap-2">
            <button
              type="button"
              class="rounded-lg border border-gray-200 bg-white px-4 py-2 text-sm font-semibold text-gray-700 transition hover:bg-gray-50"
              @click="closeManualModelDialog"
            >
              {{ t('mcp.cancel') }}
            </button>
            <button
              type="button"
              class="rounded-lg bg-pink-500 px-4 py-2 text-sm font-semibold text-white transition hover:bg-pink-600 disabled:cursor-not-allowed disabled:bg-pink-300"
              :disabled="manualModelName.trim().length === 0"
              @click="addManualModel"
            >
              {{ t('providers.manualModelAdd') }}
            </button>
          </div>
        </div>
      </div>

      <div
        v-if="isCreateProviderDialogOpen"
        class="fixed inset-0 z-40 flex items-center justify-center bg-gray-900/35 px-4"
        @click.self="closeCreateProviderDialog"
      >
        <div class="w-full max-w-lg rounded-2xl border border-gray-200 bg-white p-5 shadow-2xl">
          <div>
            <div class="text-base font-semibold text-gray-800">{{ t('providers.createProviderTitle') }}</div>
            <div class="mt-1 text-sm text-gray-500">{{ t('providers.createProviderHint') }}</div>
          </div>
          <div class="mt-4 grid grid-cols-1 gap-4 md:grid-cols-2">
            <label class="space-y-1 md:col-span-1">
              <div class="text-xs font-medium uppercase tracking-wider text-gray-500">{{ t('providers.providerId') }}</div>
              <input
                v-model="newProviderForm.id"
                type="text"
                :placeholder="t('providers.providerIdPlaceholder')"
                class="w-full rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm outline-none transition focus:border-pink-400 focus:ring-2 focus:ring-pink-500/20"
              />
            </label>
            <label class="space-y-1 md:col-span-1">
              <div class="text-xs font-medium uppercase tracking-wider text-gray-500">{{ t('providers.providerDisplayName') }}</div>
              <input
                v-model="newProviderForm.displayName"
                type="text"
                :placeholder="t('providers.providerDisplayNamePlaceholder')"
                class="w-full rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm outline-none transition focus:border-pink-400 focus:ring-2 focus:ring-pink-500/20"
              />
            </label>
            <label class="space-y-1 md:col-span-2">
              <div class="text-xs font-medium uppercase tracking-wider text-gray-500">{{ t('providers.apiBaseUrl') }}</div>
              <input
                v-model="newProviderForm.apiBase"
                type="text"
                :placeholder="t('providers.createProviderApiBasePlaceholder')"
                class="w-full rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm outline-none transition focus:border-pink-400 focus:ring-2 focus:ring-pink-500/20"
              />
            </label>
            <label class="space-y-1 md:col-span-2">
              <div class="text-xs font-medium uppercase tracking-wider text-gray-500">{{ t('providers.apiKey') }}</div>
              <input
                v-model="newProviderForm.apiKey"
                type="password"
                :placeholder="t('providers.enterApiKey')"
                class="w-full rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm outline-none transition focus:border-pink-400 focus:ring-2 focus:ring-pink-500/20"
              />
            </label>
            <label class="space-y-1 md:col-span-2">
              <div class="text-xs font-medium uppercase tracking-wider text-gray-500">{{ t('providers.defaultModel') }}</div>
              <input
                v-model="newProviderForm.defaultModel"
                type="text"
                :placeholder="t('providers.defaultModelPlaceholder')"
                class="w-full rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm outline-none transition focus:border-pink-400 focus:ring-2 focus:ring-pink-500/20"
                @keydown.enter.prevent="createProvider"
              />
            </label>
          </div>
          <div class="mt-5 flex justify-end gap-2">
            <button
              type="button"
              class="rounded-lg border border-gray-200 bg-white px-4 py-2 text-sm font-semibold text-gray-700 transition hover:bg-gray-50"
              @click="closeCreateProviderDialog"
            >
              {{ t('mcp.cancel') }}
            </button>
            <button
              type="button"
              class="rounded-lg bg-pink-500 px-4 py-2 text-sm font-semibold text-white transition hover:bg-pink-600 disabled:cursor-not-allowed disabled:bg-pink-300"
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

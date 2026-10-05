import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ProvidersSettings from './ProvidersSettings.vue';
import { deleteCustomProvider, getProviderModels, loadProviderState } from '../../api/settings';
import { appConfirm } from '../../utils/appDialog';

const provider = {
  name: 'deepseek',
  api_type: 'openai',
  source: 'builtin',
  display_name: 'DeepSeek',
  default_model: 'deepseek-chat',
  default_api_base: 'https://api.deepseek.com/v1',
  models: ['deepseek-chat', 'deepseek-reasoner'],
  custom_models: [],
};

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}));

vi.mock('@lucide/vue', () => {
  const icon = (name: string) => ({ name, template: `<span class="${name}" />` });
  return {
    Server: icon('Server'),
    Check: icon('Check'),
    Cpu: icon('Cpu'),
    ShieldCheck: icon('ShieldCheck'),
    ShieldAlert: icon('ShieldAlert'),
    RefreshCcw: icon('RefreshCcw'),
    Plus: icon('Plus'),
    Trash2: icon('Trash2'),
    PlugZap: icon('PlugZap'),
    LoaderCircle: icon('LoaderCircle'),
    CircleAlert: icon('CircleAlert'),
    Eye: icon('Eye'),
    EyeOff: icon('EyeOff'),
    MoreHorizontal: icon('MoreHorizontal'),
    ChevronDown: icon('ChevronDown'),
    ChevronRight: icon('ChevronRight'),
  };
});

vi.mock('../../api/desktop', () => ({}));

vi.mock('../../api/settings', () => ({
  loadProviderState: vi.fn(() => Promise.resolve({
    providers: [provider],
    statusReport: {
      config: {},
      default_provider: 'deepseek',
      default_model: 'deepseek-chat',
      logging: {},
      providers: [],
      channels: [],
      cron_jobs: 0,
      mcp_servers: { configured: 0, disabled: 0 },
      doctor: { valid: true, ready: true, errors: [], warnings: [] },
    },
    providerConfigs: {},
    runtime: { provider: 'deepseek', apiBase: 'https://api.deepseek.com/v1', model: 'deepseek-chat', apiKeySet: true },
    entryByName: {},
    keySetByName: { deepseek: true },
    readOnly: false,
    frozen: false,
  })),
  saveActiveProvider: vi.fn(() => Promise.resolve()),
  addProviderModel: vi.fn(),
  createCustomProvider: vi.fn(),
  deleteCustomProvider: vi.fn(),
  getProviderModels: vi.fn(),
  removeProviderModel: vi.fn(),
  testProviderModel: vi.fn(),
}));

vi.mock('../../utils/appDialog', () => ({ appConfirm: vi.fn() }));
vi.mock('../../utils/appToast', () => ({ showAppToast: vi.fn() }));
vi.mock('./ProviderWizardModal.vue', () => ({ default: { template: '<div />' } }));
vi.mock('./ProviderListItem.vue', () => ({
  default: {
    props: ['provider'],
    emits: ['select', 'delete'],
    template: '<div><button @click="$emit(\'select\', provider)">{{ provider.display_name }}</button><button v-if="provider.source === \'custom\'" :aria-label="\'delete \' + provider.name" @click="$emit(\'delete\', provider)">delete</button></div>',
  },
}));

describe('ProvidersSettings', () => {
  it('reconciles a deleted provider while its model refresh is still pending', async () => {
    const custom = { ...provider, name: 'custom', display_name: 'Custom', source: 'custom', default_api_base: 'http://localhost:8080/v1' };
    const originalSnapshot = await loadProviderState();
    vi.mocked(loadProviderState)
      .mockResolvedValueOnce({ ...originalSnapshot, providers: [custom, provider] })
      .mockResolvedValueOnce(originalSnapshot);
    vi.mocked(appConfirm).mockResolvedValueOnce(true);
    vi.mocked(deleteCustomProvider).mockResolvedValueOnce(undefined);
    let finishRefresh!: (value: any) => void;
    vi.mocked(getProviderModels).mockImplementationOnce(() => new Promise(resolve => { finishRefresh = resolve; }));
    const wrapper = mount(ProvidersSettings, {
      props: {
        config: { provider: 'custom', apiBase: custom.default_api_base, apiKey: '', model: 'deepseek-chat' },
        saveConfigAction: vi.fn(() => Promise.resolve()),
      },
    });
    await flushPromises();
    await wrapper.findAll('button').find(button => button.text() === 'providers.refreshModels')!.trigger('click');
    await wrapper.find('button[aria-label="delete custom"]').trigger('click');
    await flushPromises();
    expect(deleteCustomProvider).toHaveBeenCalledWith('custom');
    expect(wrapper.find('button[aria-label="delete custom"]').exists()).toBe(false);
    expect(wrapper.find('.providers-detail-header').text()).toContain('DeepSeek');
    finishRefresh({ ...custom, provider: 'custom' });
    await flushPromises();
    expect(wrapper.find('.providers-detail-header').text()).toContain('DeepSeek');
    wrapper.unmount();
  });
  it('shows a recoverable load error instead of asking to select a missing provider', async () => {
    vi.mocked(loadProviderState).mockRejectedValueOnce(new Error('service unavailable'));
    const wrapper = mount(ProvidersSettings, {
      props: {
        config: { provider: 'deepseek', apiBase: 'https://api.deepseek.com/v1', apiKey: '', model: 'deepseek-chat' },
        saveConfigAction: vi.fn(() => Promise.resolve()),
      },
    });
    await flushPromises();
    expect(wrapper.find('[role="alert"]').text()).toContain('providers.loadError');
    expect(wrapper.text()).not.toContain('providers.selectProvider');
    const retry = wrapper.findAll('button').find(button => button.text() === 'providers.retry');
    await retry!.trigger('click');
    await flushPromises();
    expect(wrapper.find('[role="alert"]').exists()).toBe(false);
    expect(wrapper.findAll('.providers-model-card')).toHaveLength(2);
    wrapper.unmount();
  });
  it('persists the selected model immediately when its card is clicked', async () => {
    const saveConfigAction = vi.fn(() => Promise.resolve());
    const wrapper = mount(ProvidersSettings, {
      props: {
        config: {
          provider: 'deepseek',
          apiBase: 'https://api.deepseek.com/v1',
          apiKey: 'test-key',
          model: 'deepseek-chat',
        },
        providerConfigs: {},
        savedModels: [],
        saveConfigAction,
      },
    });

    await flushPromises();
    const modelCard = wrapper.findAll('.providers-model-card').find((card) =>
      card.text().includes('deepseek-reasoner')
    );
    expect(modelCard).toBeDefined();

    await modelCard!.trigger('click');
    await flushPromises();

    expect(saveConfigAction).toHaveBeenCalledWith({
      provider: 'deepseek',
      apiBase: 'https://api.deepseek.com/v1',
      apiKey: 'test-key',
      model: 'deepseek-reasoner',
    });
    expect(wrapper.emitted('update-saved-models')?.[0]?.[0]).toEqual([
      expect.objectContaining({
        provider: 'deepseek',
        model: 'deepseek-reasoner',
      }),
    ]);
  });
});

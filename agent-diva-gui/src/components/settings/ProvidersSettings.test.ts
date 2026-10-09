import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ProvidersSettings from './ProvidersSettings.vue';
import { getProviderModels, loadProviderState, testProviderModel } from '../../api/settings';

const provider = {
  name: 'deepseek',
  api_type: 'openai',
  source: 'builtin',
  display_name: 'DeepSeek',
  default_model: 'deepseek-chat',
  default_api_base: 'https://api.deepseek.com/v1',
  models: ['deepseek-chat', 'deepseek-reasoner'],
  custom_models: [],
  executable: true,
  capability_state: 'READY',
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
describe('ProvidersSettings', () => {
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

  it('keeps deferred providers visible and deletable while blocking execution actions', async () => {
    const deferredProvider = {
      ...provider,
      name: 'custom-responses',
      api_type: 'openai-responses',
      source: 'custom',
      display_name: 'Deferred Responses',
      default_model: 'response-model',
      default_api_base: 'https://responses.example/v1',
      models: ['response-model'],
      executable: false,
      capability_state: 'DEFERRED-INDEFINITE',
    };
    vi.mocked(loadProviderState).mockResolvedValueOnce({
      providers: [deferredProvider],
      statusReport: {
        config: {},
        default_provider: 'custom-responses',
        default_model: 'response-model',
        logging: {},
        providers: [],
        channels: [],
        cron_jobs: 0,
        mcp_servers: { configured: 0, disabled: 0 },
        doctor: { valid: true, ready: false, errors: [], warnings: [] },
      },
      providerConfigs: {},
      runtime: { provider: 'custom-responses', apiBase: deferredProvider.default_api_base, model: 'response-model', apiKeySet: true },
      entryByName: {},
      keySetByName: { 'custom-responses': true },
      readOnly: false,
      frozen: false,
    } as never);

    const saveConfigAction = vi.fn(() => Promise.resolve());
    const wrapper = mount(ProvidersSettings, {
      props: {
        config: {
          provider: 'another-provider',
          apiBase: '',
          apiKey: '',
          model: 'other-model',
        },
        providerConfigs: {},
        savedModels: [],
        saveConfigAction,
      },
    });

    await flushPromises();
    const row = wrapper.find('.providers-list-item');
    expect(row.text()).toContain('Deferred Responses');
    expect(row.find('[title="DEFERRED-INDEFINITE"]').exists()).toBe(true);
    expect(row.find('[title="providers.deleteProvider"]').exists()).toBe(true);

    const modelCard = wrapper.find('.providers-model-card');
    expect(modelCard.text()).toContain('response-model');
    await modelCard.trigger('click');
    expect(saveConfigAction).not.toHaveBeenCalled();

    expect(wrapper.find('button[title="providers.testConnection"]').attributes('disabled')).toBeDefined();
    const refreshButton = wrapper.findAll('button').find((button) => button.text().includes('providers.refreshModels'));
    expect(refreshButton?.attributes('disabled')).toBeDefined();
    expect(wrapper.find('button[title="providers.manualModelTitle"]').attributes('disabled')).toBeDefined();
    expect(wrapper.find('.btn-save-config').attributes('disabled')).toBeDefined();
    expect(getProviderModels).not.toHaveBeenCalled();
    expect(testProviderModel).not.toHaveBeenCalled();
  });
});

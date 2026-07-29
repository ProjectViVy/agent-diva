import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ProvidersSettings from './ProvidersSettings.vue';

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

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(() => Promise.resolve([provider])),
}));

vi.mock('../../api/desktop', () => ({
  getConfigStatus: vi.fn(() => Promise.resolve({
    config: {},
    default_provider: 'deepseek',
    default_model: 'deepseek-chat',
    logging: {},
    providers: [],
    channels: [],
    cron_jobs: 0,
    mcp_servers: { configured: 0, disabled: 0 },
    doctor: { valid: true, ready: true, errors: [], warnings: [] },
  })),
}));

vi.mock('../../api/providers', () => ({
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
    template: '<button @click="$emit(\'select\', provider)">{{ provider.display_name }}</button>',
  },
}));

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
});

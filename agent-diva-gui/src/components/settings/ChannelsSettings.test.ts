import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ChannelsSettings from './ChannelsSettings.vue';
import ChannelCardView from './ChannelCardView.vue';
import ChannelWizardModal from './ChannelWizardModal.vue';

const rawChannels = {
  telegram: { enabled: true, token: 'abc' },
  slack: { enabled: true, bot_token: 'retired' },
  feishu: { enabled: false, app_id: '', app_secret: '', verification_token: '' },
};

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}));

vi.mock('@lucide/vue', () => {
  const icon = (name: string) => ({ name, template: `<span class="${name}" />` });
  return {
    LoaderCircle: icon('LoaderCircle'),
    MessageSquare: icon('MessageSquare'),
    LayoutGrid: icon('LayoutGrid'),
    List: icon('List'),
    Plus: icon('Plus'),
    RefreshCw: icon('RefreshCw'),
  };
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === 'get_channels') return Promise.resolve(structuredClone(rawChannels));
    return Promise.resolve(null);
  }),
}));

vi.mock('../../api/desktop', () => ({
  getConfigStatus: vi.fn(() => Promise.resolve({ channels: [] })),
}));

vi.mock('./ChannelCardView.vue', () => ({
  default: {
    props: ['channels', 'statuses', 'loading'],
    emits: ['add', 'edit', 'delete', 'toggle'],
    template: `<div class="card-stub">
      <button
        v-for="(cfg, name) in channels"
        :key="name"
        :class="'toggle-' + name"
        @click="$emit('toggle', name)"
      >{{ name }}:{{ cfg.enabled }}</button>
    </div>`,
  },
}));

vi.mock('./ChannelWizardModal.vue', () => ({
  default: {
    props: ['open', 'initialData'],
    emits: ['update:open', 'test', 'complete'],
    template: '<div class="wizard-stub" />',
  },
}));

vi.mock('./ChannelEditorForm.vue', () => ({
  default: {
    props: ['platform', 'config'],
    template: '<div class="editor-stub">{{ platform }}</div>',
  },
}));

const mountSettings = () => {
  const saveChannelConfigAction = vi.fn(() => Promise.resolve());
  const wrapper = mount(ChannelsSettings, {
    props: { saveChannelConfigAction },
    global: { mocks: { $t: (key: string) => key } },
  });
  return { wrapper, saveChannelConfigAction };
};

describe('ChannelsSettings', () => {
  it('persists the toggled card even when it is not the selected channel', async () => {
    const { wrapper, saveChannelConfigAction } = mountSettings();
    await flushPromises();

    // telegram is auto-selected on load; toggle the non-selected feishu card.
    await wrapper.find('.toggle-feishu').trigger('click');
    await flushPromises();

    expect(saveChannelConfigAction).toHaveBeenCalledWith('feishu', {
      enabled: true,
      app_id: '',
      app_secret: '',
      verification_token: '',
    });
  });

  it('merges wizard credentials over the existing channel config on completion', async () => {
    const { wrapper, saveChannelConfigAction } = mountSettings();
    await flushPromises();

    wrapper.findComponent(ChannelWizardModal).vm.$emit('complete', {
      platform: 'feishu',
      credentials: { app_id: 'cli_x', app_secret: 'sec' },
    });
    await flushPromises();

    expect(saveChannelConfigAction).toHaveBeenCalledWith('feishu', {
      enabled: true,
      app_id: 'cli_x',
      app_secret: 'sec',
      verification_token: '',
    });
  });

  it('normalizes email wizard select strings into booleans before saving', async () => {
    const { wrapper, saveChannelConfigAction } = mountSettings();
    await flushPromises();

    wrapper.findComponent(ChannelWizardModal).vm.$emit('complete', {
      platform: 'email',
      credentials: { imap_host: 'imap.example.com', imap_use_ssl: 'false', smtp_use_ssl: 'true' },
    });
    await flushPromises();

    expect(saveChannelConfigAction).toHaveBeenCalledWith(
      'email',
      expect.objectContaining({
        imap_host: 'imap.example.com',
        imap_use_ssl: false,
        smtp_use_ssl: true,
        enabled: true,
      }),
    );
  });

  it('opens the wizard in card mode when a card is edited', async () => {
    const { wrapper } = mountSettings();
    await flushPromises();

    wrapper.findComponent(ChannelCardView).vm.$emit('edit', 'feishu');
    await flushPromises();

    const wizard = wrapper.findComponent(ChannelWizardModal);
    expect(wizard.props('open')).toBe(true);
    expect(wizard.props('initialData')).toEqual({
      platform: 'feishu',
      credentials: { enabled: false, app_id: '', app_secret: '', verification_token: '' },
    });
    expect(wrapper.find('.card-stub').exists()).toBe(true);
    expect(wrapper.find('.channels-sidebar').exists()).toBe(false);
  });

  it('renders an inline editor for every visible channel in list view', async () => {
    const { wrapper } = mountSettings();
    await flushPromises();

    await wrapper.find('button[title="channels.listView"]').trigger('click');
    await flushPromises();

    expect(wrapper.text()).not.toContain('providers.unsupportedUI');
    expect(wrapper.text()).not.toContain('channels.editViaWizardHint');
    expect(wrapper.find('.editor-stub').text()).toContain('telegram');

    const feishuItem = wrapper.findAll('.channels-item').find((item) => item.text().includes('feishu'));
    expect(feishuItem).toBeTruthy();
    await feishuItem!.trigger('click');
    await flushPromises();

    expect(wrapper.find('.editor-stub').text()).toContain('feishu');
  });

  it('hides retired channels from the list-view sidebar', async () => {
    const { wrapper } = mountSettings();
    await flushPromises();

    await wrapper.find('button[title="channels.listView"]').trigger('click');
    const sidebar = wrapper.find('.channels-sidebar').text();
    expect(sidebar).toContain('telegram');
    expect(sidebar).toContain('feishu');
    expect(sidebar).not.toContain('slack');
  });
});

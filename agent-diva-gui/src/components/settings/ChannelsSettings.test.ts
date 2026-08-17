import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ChannelsSettings from './ChannelsSettings.vue';
import ChannelCardView from './ChannelCardView.vue';
import ChannelWizardModal from './ChannelWizardModal.vue';

const rawChannels = {
  telegram: { enabled: true, token: 'abc' },
  slack: { enabled: false, bot_token: 'y' },
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

    // telegram is auto-selected on load; toggle the non-selected slack card.
    await wrapper.find('.toggle-slack').trigger('click');
    await flushPromises();

    expect(saveChannelConfigAction).toHaveBeenCalledWith('slack', {
      enabled: true,
      bot_token: 'y',
    });
  });

  it('merges wizard credentials over the existing channel config on completion', async () => {
    const { wrapper, saveChannelConfigAction } = mountSettings();
    await flushPromises();

    wrapper.findComponent(ChannelWizardModal).vm.$emit('complete', {
      platform: 'slack',
      credentials: { bot_token: 'new-token' },
    });
    await flushPromises();

    expect(saveChannelConfigAction).toHaveBeenCalledWith('slack', {
      enabled: true,
      bot_token: 'new-token',
    });
  });

  it('normalizes irc wizard credentials before saving', async () => {
    const { wrapper, saveChannelConfigAction } = mountSettings();
    await flushPromises();

    wrapper.findComponent(ChannelWizardModal).vm.$emit('complete', {
      platform: 'irc',
      credentials: { server: 'irc.example.com', channels_str: '#a, #b', use_tls: 'true' },
    });
    await flushPromises();

    expect(saveChannelConfigAction).toHaveBeenCalledWith(
      'irc',
      expect.objectContaining({
        server: 'irc.example.com',
        channels: ['#a', '#b'],
        use_tls: true,
        enabled: true,
      }),
    );
    const saved = saveChannelConfigAction.mock.calls.at(-1)![1] as Record<string, unknown>;
    expect(saved).not.toHaveProperty('channels_str');
  });

  it('prefills wizard credentials when editing an existing channel', async () => {
    const { wrapper } = mountSettings();
    await flushPromises();

    wrapper.findComponent(ChannelCardView).vm.$emit('edit', 'slack');
    await flushPromises();

    const wizard = wrapper.findComponent(ChannelWizardModal);
    expect(wizard.props('initialData')).toEqual({
      platform: 'slack',
      credentials: { enabled: false, bot_token: 'y' },
    });
  });
});

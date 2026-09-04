import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import ChannelsSettings from './ChannelsSettings.vue';
import ChannelCardView from './ChannelCardView.vue';
import ChannelWizardModal from './ChannelWizardModal.vue';

const rawChannels = {
  telegram: { enabled: true, token: 'abc' },
  discord: { enabled: false, token: '' },
  feishu: { enabled: false, app_id: '', app_secret: '', verification_token: '' },
  dingtalk: { enabled: false, client_id: '', client_secret: '' },
  email: { enabled: false, imap_host: '', smtp_host: '' },
  qq: { enabled: false, app_id: '', client_secret: '' },
};

const runtimeChannels = [
  {
    name: 'telegram',
    registered: true,
    lifecycle: 'running',
    health: 'healthy',
    diagnosis: null,
  },
];

let channelsResponse: unknown = rawChannels;
let channelLoadError: Error | null = null;
let channelLoadQueue: Promise<unknown>[] = [];

const appDialogMock = vi.hoisted(() => ({
  appConfirmAsync: vi.fn(async (_message: string, action: () => Promise<void>) => {
    await action();
    return true;
  }),
}));

vi.mock('../../utils/appDialog', () => appDialogMock);

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
    if (cmd === 'get_channels') {
      if (channelLoadError) return Promise.reject(channelLoadError);
      const queued = channelLoadQueue.shift();
      if (queued) return queued;
      return Promise.resolve(structuredClone(channelsResponse));
    }
    if (cmd === 'get_channel_runtime') return Promise.resolve(structuredClone(runtimeChannels));
    if (cmd === 'delete_channel') return Promise.resolve(null);
    if (cmd === 'probe_channel') return Promise.resolve({ success: true, message: 'ok' });
    return Promise.resolve(null);
  }),
}));

vi.mock('../../api/desktop', () => ({
  getConfigStatus: vi.fn(() => Promise.resolve({ channels: [] })),
}));

vi.mock('./ChannelCardView.vue', () => ({
  default: {
    props: ['channels', 'statuses', 'loading', 'canAdd', 'busyChannels', 'errors', 'error'],
    emits: ['add', 'edit', 'delete', 'toggle'],
    template: `<div class="card-stub">
      <button
        v-for="(cfg, name) in channels"
        :key="name"
        :class="'toggle-' + name"
        @click="$emit('toggle', name)"
      >{{ name }}:{{ cfg.enabled }}</button>
      <p v-for="(message, name) in errors" :key="name" :class="'channel-error-' + name">{{ message }}</p>
    </div>`,
  },
}));

vi.mock('./ChannelWizardModal.vue', () => ({
  default: {
    props: ['open', 'initialData', 'availablePlatforms', 'onTest', 'onComplete'],
    emits: ['update:open'],
    template: '<div class="wizard-stub" />',
  },
}));

vi.mock('./ChannelEditorForm.vue', () => ({
  default: {
    props: ['platform', 'config', 'disabled'],
    template: `<div class="editor-stub">
      {{ platform }}
      <input
        class="editor-input"
        :disabled="disabled"
        :value="String(config.token ?? config.app_id ?? '')"
        @input="config.token !== undefined ? config.token = $event.target.value : config.app_id = $event.target.value"
      />
    </div>`,
  },
}));

const mountSettings = (saveChannelConfigAction = vi.fn(() => Promise.resolve())) => {
  const wrapper = mount(ChannelsSettings, {
    props: { saveChannelConfigAction },
    global: { mocks: { $t: (key: string) => key } },
  });
  return { wrapper, saveChannelConfigAction };
};

describe('ChannelsSettings', () => {
  beforeEach(() => {
    channelsResponse = rawChannels;
    channelLoadError = null;
    channelLoadQueue = [];
    appDialogMock.appConfirmAsync.mockClear();
  });

  it('shows a load error and retry while preserving the current channel content', async () => {
    const { wrapper } = mountSettings();
    await flushPromises();

    channelLoadError = new Error('temporary load error');
    await wrapper.find('button[title="topbar.refresh"]').trigger('click');
    await flushPromises();

    expect(wrapper.find('.channels-load-error').text()).toContain('temporary load error');
    expect(wrapper.find('.channels-load-retry').exists()).toBe(true);
    expect(wrapper.find('.toggle-telegram').exists()).toBe(true);

    channelLoadError = null;
    await wrapper.find('.channels-load-retry').trigger('click');
    await flushPromises();
    expect(wrapper.find('.channels-load-error').exists()).toBe(false);
    wrapper.unmount();
  });

  it('treats a malformed channels response as an error instead of an empty collection', async () => {
    channelsResponse = { telegram: 'not-a-channel-config' };
    const { wrapper } = mountSettings();
    await flushPromises();

    expect(wrapper.find('.channels-load-error').text()).toContain('channels.invalidResponse');
    expect(wrapper.find('.toggle-telegram').exists()).toBe(false);
    wrapper.unmount();
  });

  it('preserves a new edit made during submission and commits only the submitted snapshot', async () => {
    const { wrapper, saveChannelConfigAction } = mountSettings();
    await flushPromises();
    await wrapper.find('button[title="channels.listView"]').trigger('click');

    await wrapper.find('.editor-input').setValue('submitted-token');
    let releaseSave!: () => void;
    saveChannelConfigAction.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          releaseSave = resolve;
        }),
    );
    const saveClick = wrapper.find('.btn-save-config').trigger('click');
    await flushPromises();

    expect(wrapper.find('.btn-save-config').attributes('disabled')).toBeDefined();
    expect(wrapper.find('.editor-input').attributes('disabled')).toBeDefined();
    (wrapper.vm as any).draftChannels.telegram.token = 'edited-during-save';

    releaseSave();
    await saveClick;
    await flushPromises();

    const state = wrapper.vm as any;
    expect(state.savedChannels.telegram.token).toBe('submitted-token');
    expect(state.draftChannels.telegram.token).toBe('edited-during-save');
    expect(wrapper.find('.btn-save-config').attributes('disabled')).toBeUndefined();
    wrapper.unmount();
  });

  it('keeps another channel draft when saving the selected channel', async () => {
    const { wrapper } = mountSettings();
    await flushPromises();
    await wrapper.find('button[title="channels.listView"]').trigger('click');

    const feishuItem = wrapper.findAll('.channels-item').find((item) => item.text().includes('feishu'));
    expect(feishuItem).toBeTruthy();
    await feishuItem!.trigger('click');
    await wrapper.find('.editor-input').setValue('local-feishu-draft');

    const telegramItem = wrapper.findAll('.channels-item').find((item) => item.text().includes('telegram'));
    expect(telegramItem).toBeTruthy();
    await telegramItem!.trigger('click');
    await wrapper.find('.editor-input').setValue('local-telegram-save');
    await wrapper.find('.btn-save-config').trigger('click');
    await flushPromises();

    await feishuItem!.trigger('click');
    expect((wrapper.find('.editor-input').element as HTMLInputElement).value).toBe('local-feishu-draft');
    wrapper.unmount();
  });

  it('ignores an older explicit load response when a newer load has completed', async () => {
    const { wrapper } = mountSettings();
    await flushPromises();

    let resolveOlder!: (value: unknown) => void;
    let resolveNewer!: (value: unknown) => void;
    channelLoadQueue.push(
      new Promise((resolve) => {
        resolveOlder = resolve;
      }),
      new Promise((resolve) => {
        resolveNewer = resolve;
      }),
    );

    const refreshOlder = (wrapper.vm as any).handleRefresh();
    const refreshNewer = (wrapper.vm as any).handleRefresh();
    resolveNewer({ ...rawChannels, telegram: { enabled: false, token: 'newer' } });
    await refreshNewer;
    resolveOlder({ ...rawChannels, telegram: { enabled: true, token: 'older' } });
    await refreshOlder;
    await flushPromises();

    expect(wrapper.find('.toggle-telegram').text()).toContain('telegram:false');
    wrapper.unmount();
  });

  it('shows a toggle failure on the affected channel while keeping its draft', async () => {
    const { wrapper, saveChannelConfigAction } = mountSettings();
    await flushPromises();
    saveChannelConfigAction.mockRejectedValue(new Error('toggle failed'));

    await wrapper.find('.toggle-feishu').trigger('click');
    await flushPromises();

    expect(wrapper.find('.channel-error-feishu').text()).toContain('toggle failed');
    expect(wrapper.find('.toggle-feishu').text()).toContain('feishu:true');
    wrapper.unmount();
  });

  it('filters removed channels and exposes only recoverable platforms to the wizard', async () => {
    channelsResponse = { ...rawChannels, removed: ['discord', 'retired'] };
    const { wrapper } = mountSettings();
    await flushPromises();

    expect(wrapper.find('.toggle-telegram').exists()).toBe(true);
    expect(wrapper.find('.toggle-discord').exists()).toBe(false);
    expect(wrapper.find('button.btn-primary').exists()).toBe(true);
    expect(wrapper.findComponent(ChannelWizardModal).props('availablePlatforms')).toEqual(['discord']);
  });

  it('hides add actions when no platform can be recovered', async () => {
    channelsResponse = { ...rawChannels, removed: [] };
    const { wrapper } = mountSettings();
    await flushPromises();

    expect(wrapper.find('button.btn-primary').exists()).toBe(false);
    expect(wrapper.findComponent(ChannelWizardModal).props('availablePlatforms')).toEqual([]);
  });

  it('probes a normalized credential payload through the backend command', async () => {
    const { wrapper } = mountSettings();
    await flushPromises();

    const test = wrapper.findComponent(ChannelWizardModal).props('onTest') as (data: {
      platform: string;
      name: string;
      credentials: Record<string, unknown>;
    }) => Promise<{ success: boolean; message: string }>;
    await test({
      platform: 'email',
      name: 'Email',
      credentials: { imap_use_ssl: 'false', smtp_use_ssl: 'true' },
    });

    expect(invoke).toHaveBeenCalledWith('probe_channel', {
      name: 'email',
      config: { imap_use_ssl: false, smtp_use_ssl: true },
    });
  });

  it('deletes through the backend and commits the local removal before closing confirmation', async () => {
    const { wrapper } = mountSettings();
    await flushPromises();
    const getChannelsCallsBefore = vi.mocked(invoke).mock.calls.filter(([command]) => command === 'get_channels').length;

    wrapper.findComponent(ChannelCardView).vm.$emit('delete', 'telegram');
    await flushPromises();

    expect(invoke).toHaveBeenCalledWith('delete_channel', { name: 'telegram' });
    expect(wrapper.find('.toggle-telegram').exists()).toBe(false);
    expect(vi.mocked(invoke).mock.calls.filter(([command]) => command === 'get_channels').length).toBe(getChannelsCallsBefore);
    expect(appDialogMock.appConfirmAsync).toHaveBeenCalledWith(
      'channels.deleteConfirm',
      expect.any(Function),
      expect.objectContaining({ title: 'channels.deleteTitle' }),
    );
  });

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

    const complete = wrapper.findComponent(ChannelWizardModal).props('onComplete') as (data: {
      platform: string;
      name: string;
      credentials: Record<string, unknown>;
    }) => Promise<void>;
    await complete({
      platform: 'feishu',
      name: '飞书',
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

    const complete = wrapper.findComponent(ChannelWizardModal).props('onComplete') as (data: {
      platform: string;
      name: string;
      credentials: Record<string, unknown>;
    }) => Promise<void>;
    await complete({
      platform: 'email',
      name: 'Email',
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

  it('renders exactly the six production channels in the list-view sidebar', async () => {
    const { wrapper } = mountSettings();
    await flushPromises();

    await wrapper.find('button[title="channels.listView"]').trigger('click');
    const sidebar = wrapper.find('.channels-sidebar').text();
    expect(sidebar).toContain('telegram');
    expect(sidebar).toContain('discord');
    expect(sidebar).toContain('feishu');
    expect(sidebar).toContain('dingtalk');
    expect(sidebar).toContain('email');
    expect(sidebar).toContain('qq');
  });
});

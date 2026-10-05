import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ChannelsSettings from './ChannelsSettings.vue';
import type { ChannelView } from '../../api/settings';

const { loadChannelsState, saveChannel, runtime } = vi.hoisted(() => ({
  loadChannelsState: vi.fn(),
  saveChannel: vi.fn(),
  runtime: { tauri: true },
}));

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('../../api/desktop', () => ({
  isTauriRuntime: () => runtime.tauri,
}));

vi.mock('../../api/settings', () => ({
  loadChannelsState: (...args: unknown[]) => loadChannelsState(...args),
  saveChannel: (...args: unknown[]) => saveChannel(...args),
}));

function caps() {
  return {
    typing: false, edit: false, delete: false, reaction: false,
    placeholder: false, media: false, media_store: false, webhook: false,
    listen: false, stream: false, health: true,
  };
}

function viewFixture(overrides: Partial<ChannelView> = {}): ChannelView {
  return {
    name: 'telegram',
    envelope: {
      name: 'telegram',
      enabled: true,
      allow_from: ['123'],
      token_env: 'TELEGRAM_BOT_TOKEN',
      configured: true,
    },
    status: {
      name: 'telegram',
      capabilities: caps(),
      health: { ok: true },
      configured: true,
      enabled: false,
      allow_from: ['123'],
      started: true,
      token_env: 'TELEGRAM_BOT_TOKEN',
      token_env_set: true,
      note: '',
    },
    pendingRestart: true,
    ...overrides,
  };
}

describe('ChannelsSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    runtime.tauri = true;
    loadChannelsState.mockResolvedValue([]);
    saveChannel.mockResolvedValue({});
  });

  it('lists channels with status and pending-restart badge', async () => {
    loadChannelsState.mockResolvedValue([viewFixture()]);
    const wrapper = mount(ChannelsSettings);
    await flushPromises();

    expect(wrapper.text()).toContain('telegram');
    expect(wrapper.text()).toContain('settings.running');
    expect(wrapper.text()).toContain('settings.pendingRestart');
    expect(wrapper.text()).toContain('TELEGRAM_BOT_TOKEN');
  });

  it('toggles enabled via channel/update', async () => {
    loadChannelsState.mockResolvedValue([viewFixture()]);
    const wrapper = mount(ChannelsSettings);
    await flushPromises();

    const toggle = wrapper.findAll('.skills-btn').find((b) => b.text().includes('settings.disableChannel'));
    expect(toggle).toBeDefined();
    await toggle!.trigger('click');
    await flushPromises();

    expect(saveChannel).toHaveBeenCalledWith({ name: 'telegram', enabled: false });
    expect(loadChannelsState.mock.calls.length).toBeGreaterThanOrEqual(2);
  });

  it('saves allow_from and token_env edits through channel/update', async () => {
    loadChannelsState.mockResolvedValue([viewFixture({ pendingRestart: false })]);
    const wrapper = mount(ChannelsSettings);
    await flushPromises();

    await wrapper.find('.toolbar-btn').trigger('click');
    await flushPromises();

    const textarea = wrapper.find('textarea');
    await textarea.setValue('111\n222');
    const inputs = wrapper.findAll('input.skills-search-input');
    await inputs[inputs.length - 1].setValue('TG_TOKEN');

    const saveBtn = wrapper.findAll('.skills-btn').find((b) => b.text().includes('settings.save'));
    await saveBtn!.trigger('click');
    await flushPromises();

    expect(saveChannel).toHaveBeenCalledWith({
      name: 'telegram',
      allow_from: ['111', '222'],
      token_env: 'TG_TOKEN',
    });
  });

  it('surfaces backend errors without hiding them', async () => {
    loadChannelsState.mockResolvedValue([viewFixture({ pendingRestart: false })]);
    saveChannel.mockRejectedValue(new Error('conflict (-32009): read-only'));
    const wrapper = mount(ChannelsSettings);
    await flushPromises();

    const toggle = wrapper.findAll('.skills-btn').find((b) => b.text().includes('settings.disableChannel'));
    await toggle!.trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('read-only');
  });

  it('uses settings translations in browser preview mode', async () => {
    runtime.tauri = false;
    const wrapper = mount(ChannelsSettings);
    await flushPromises();
    expect(wrapper.text()).toContain('settings.restartHint');
    expect(wrapper.text()).toContain('settings.channelsPreviewOnly');
    expect(wrapper.text()).not.toContain('channels.restartHint');
    expect(wrapper.text()).not.toContain('general.skillsPreviewOnly');
    wrapper.unmount();
  });
});

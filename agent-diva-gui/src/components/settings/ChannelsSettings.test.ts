import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ChannelsSettings from './ChannelsSettings.vue';
import type { ChannelView } from '../../api/settings';

const loadChannelsState = vi.fn();
const saveChannel = vi.fn();

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('../../api/desktop', () => ({
  isTauriRuntime: () => true,
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
    loadChannelsState.mockResolvedValue([]);
    saveChannel.mockResolvedValue({});
  });

  it('lists channels with status and pending-restart badge', async () => {
    loadChannelsState.mockResolvedValue([viewFixture()]);
    const wrapper = mount(ChannelsSettings);
    await flushPromises();

    expect(wrapper.text()).toContain('telegram');
    expect(wrapper.text()).toContain('channels.running');
    expect(wrapper.text()).toContain('channels.pendingRestart');
    expect(wrapper.text()).toContain('TELEGRAM_BOT_TOKEN');
  });

  it('toggles enabled via channel/update', async () => {
    loadChannelsState.mockResolvedValue([viewFixture()]);
    const wrapper = mount(ChannelsSettings);
    await flushPromises();

    const toggle = wrapper.findAll('.skills-btn').find((b) => b.text().includes('general.disableSkill'));
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

    const saveBtn = wrapper.findAll('.skills-btn').find((b) => b.text().includes('general.save'));
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

    const toggle = wrapper.findAll('.skills-btn').find((b) => b.text().includes('general.disableSkill'));
    await toggle!.trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('read-only');
  });
});

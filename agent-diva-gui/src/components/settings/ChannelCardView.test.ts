import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ChannelCardView from './ChannelCardView.vue';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}));

vi.mock('@lucide/vue', () => {
  const icon = (name: string) => ({ name, template: `<span class="${name}" />` });
  return {
    Plus: icon('Plus'),
    MessageSquarePlus: icon('MessageSquarePlus'),
    Pencil: icon('Pencil'),
    Trash2: icon('Trash2'),
    Power: icon('Power'),
    MessageSquare: icon('MessageSquare'),
    Mail: icon('Mail'),
    Globe: icon('Globe'),
  };
});

const rawChannels = {
  telegram: { enabled: true, token: 'abc' },
  'neuro-link': { enabled: false, host: '0.0.0.0', port: 9100 },
  slack: { enabled: true, bot_token: 'xoxb-retired' },
};

const mountView = (channels: Record<string, any>, statuses: any[] = []) =>
  mount(ChannelCardView, {
    props: { channels, statuses },
    global: { mocks: { $t: (key: string) => key } },
  });

describe('ChannelCardView', () => {
  it('normalizes the raw config map into named, enabled cards', () => {
    const wrapper = mountView(rawChannels);
    const text = wrapper.text();
    expect(text).toContain('Telegram');
    expect(text).toContain('Neuro-Link');
    expect(text).toContain('channels.enabled');
    expect(text).toContain('channels.disabled');
    expect(wrapper.findAll('.channel-card')).toHaveLength(2);
  });

  it('hides retired channels even when they are enabled in the config', () => {
    const wrapper = mountView(rawChannels);
    expect(wrapper.text()).not.toContain('Slack');
    expect(wrapper.findAll('.channel-card')).toHaveLength(2);
  });

  it('renders the empty state when there are no channels', () => {
    const wrapper = mountView({});
    expect(wrapper.find('.channel-empty-state').exists()).toBe(true);
  });

  it('renders the empty state when only retired channels remain', () => {
    const wrapper = mountView({ slack: { enabled: true }, irc: { enabled: false } });
    expect(wrapper.find('.channel-empty-state').exists()).toBe(true);
  });

  it('bubbles toggle events with the config map key', async () => {
    const wrapper = mountView(rawChannels);
    const toggles = wrapper.findAll('.action-btn').filter((b) => b.attributes('title') === 'channels.deactivate' || b.attributes('title') === 'channels.activate');
    await toggles[0].trigger('click');
    expect(wrapper.emitted('toggle')?.[0]).toEqual(['telegram']);
  });

  it('matches status summaries by channel name', () => {
    const wrapper = mountView(rawChannels, [
      { name: 'telegram', enabled: true, ready: true, missing_fields: [], notes: [] },
    ]);
    expect(wrapper.text()).toContain('channels.activated');
  });
});

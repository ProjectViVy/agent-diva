import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ChannelCardView from './ChannelCardView.vue';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}));

vi.mock('@lucide/vue', () => {
  const icon = (name: string) => ({ name, template: `<span class="${name}" />` });
  return {
    CircleAlert: icon('CircleAlert'),
    LoaderCircle: icon('LoaderCircle'),
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
  discord: { enabled: false, token: '' },
};

const mountView = (
  channels: Record<string, any>,
  statuses: any[] = [],
  canAdd?: boolean,
  busyChannels: string[] = [],
  loading = false,
  error: string | null = null,
  errors: Record<string, string> = {},
) =>
  mount(ChannelCardView, {
    props: { channels, statuses, canAdd, busyChannels, loading, error, errors },
    global: { mocks: { $t: (key: string) => key } },
  });

describe('ChannelCardView', () => {
  it('normalizes the raw config map into named, enabled cards', () => {
    const wrapper = mountView(rawChannels);
    const text = wrapper.text();
    expect(text).toContain('Telegram');
    expect(text).toContain('Discord');
    expect(text).toContain('channels.enabled');
    expect(text).toContain('channels.disabled');
    expect(wrapper.findAll('.channel-card')).toHaveLength(2);
  });

  it('renders the empty state when there are no channels', () => {
    const wrapper = mountView({});
    expect(wrapper.find('.channel-empty-state').exists()).toBe(true);
  });

  it('shows loading instead of an empty state before the first response', () => {
    const wrapper = mountView({}, [], undefined, [], true);
    expect(wrapper.find('[data-testid="channel-card-skeleton"]').exists()).toBe(true);
    expect(wrapper.find('.channel-card-grid-skeleton').exists()).toBe(true);
    expect(wrapper.findAll('.channel-card-skeleton')).toHaveLength(6);
    expect(wrapper.find('.channel-empty-state').exists()).toBe(false);
  });

  it('shows an error instead of an empty state for a malformed response', () => {
    const wrapper = mountView({}, [], undefined, [], false, 'Invalid channel response');
    expect(wrapper.find('.channel-error-state').text()).toContain('Invalid channel response');
    expect(wrapper.find('.channel-empty-state').exists()).toBe(false);
  });

  it('renders operation errors on the affected channel only', () => {
    const wrapper = mountView(rawChannels, [], undefined, [], false, null, { telegram: 'Save failed' });
    const telegram = wrapper.findAll('.channel-card').find((card) => card.text().includes('Telegram'));
    const discord = wrapper.findAll('.channel-card').find((card) => card.text().includes('Discord'));
    expect(telegram?.find('.channel-operation-error').text()).toContain('Save failed');
    expect(discord?.find('.channel-operation-error').exists()).toBe(false);
  });

  it('hides the empty-state add action when recovery is unavailable', () => {
    const wrapper = mountView({}, [], false);
    expect(wrapper.find('.channel-empty-state').exists()).toBe(true);
    expect(wrapper.find('.empty-actions').exists()).toBe(false);
    expect(wrapper.find('.channel-empty-state p').text()).toBe('channels.noChannelsHintNoRecovery');
  });

  it('marks only the busy channel controls as disabled', () => {
    const wrapper = mountView(rawChannels, [], undefined, ['telegram']);
    const cards = wrapper.findAll('.channel-card');

    expect(cards[0].attributes('aria-busy')).toBe('true');
    expect(cards[0].findAll('button[disabled]')).toHaveLength(3);
    expect(cards[1].attributes('aria-busy')).toBeUndefined();
    expect(cards[1].findAll('button[disabled]')).toHaveLength(0);
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

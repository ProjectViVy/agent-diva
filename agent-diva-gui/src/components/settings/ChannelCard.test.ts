import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ChannelCard from './ChannelCard.vue';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}));

vi.mock('@lucide/vue', () => {
  const icon = (name: string) => ({ name, template: `<span class="${name}" />` });
  return {
    Pencil: icon('Pencil'),
    Trash2: icon('Trash2'),
    Power: icon('Power'),
    MessageSquare: icon('MessageSquare'),
    Mail: icon('Mail'),
    Globe: icon('Globe'),
    Hash: icon('Hash'),
    Boxes: icon('Boxes'),
  };
});

const mountCard = (channel: any, status?: any) =>
  mount(ChannelCard, {
    props: { channel, status },
    global: { mocks: { $t: (key: string) => key } },
  });

describe('ChannelCard', () => {
  it('renders platform display name and enabled state from normalized channel', () => {
    const wrapper = mountCard({ name: 'telegram', enabled: true, config: {} });
    expect(wrapper.text()).toContain('Telegram');
    expect(wrapper.text()).toContain('channels.enabled');
  });

  it('renders disabled state when channel is off', () => {
    const wrapper = mountCard({ name: 'discord', enabled: false, config: {} });
    expect(wrapper.text()).toContain('Discord');
    expect(wrapper.text()).toContain('channels.disabled');
  });

  it('falls back to the raw channel name for unknown platforms', () => {
    const wrapper = mountCard({ name: 'my-custom-pipe', enabled: true, config: {} });
    expect(wrapper.text()).toContain('my-custom-pipe');
  });

  it('shows missing fields from status', () => {
    const wrapper = mountCard(
      { name: 'telegram', enabled: true, config: {} },
      { name: 'telegram', enabled: true, ready: false, missing_fields: ['token'], notes: [] },
    );
    expect(wrapper.text()).toContain('token');
  });

  it('emits toggle, edit, and delete actions', async () => {
    const wrapper = mountCard({ name: 'telegram', enabled: true, config: {} });
    const buttons = wrapper.findAll('.action-btn');
    await buttons[0].trigger('click');
    await buttons[1].trigger('click');
    await buttons[2].trigger('click');
    expect(wrapper.emitted('toggle')).toHaveLength(1);
    expect(wrapper.emitted('edit')).toHaveLength(1);
    expect(wrapper.emitted('delete')).toHaveLength(1);
  });
});

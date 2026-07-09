import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import MaskCard from './MaskCard.vue';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'mask.modeAssist': 'Assist',
        'mask.modeNormal': 'Normal',
        'mask.readOnly': 'Read-only',
      };
      return map[key] ?? key;
    },
  }),
}));

function createMask(overrides: Record<string, unknown> = {}) {
  return {
    name: 'Test Mask',
    icon: '😊',
    description: 'A test mask for testing',
    mode: 'normal',
    readOnly: false,
    ...overrides,
  };
}

describe('MaskCard', () => {
  it('renders the mask name', () => {
    const mask = createMask();
    const wrapper = mount(MaskCard, { props: { mask } });
    expect(wrapper.text()).toContain('Test Mask');
  });

  it('renders the mask icon', () => {
    const mask = createMask();
    const wrapper = mount(MaskCard, { props: { mask } });
    expect(wrapper.text()).toContain('😊');
  });

  it('renders the description', () => {
    const mask = createMask();
    const wrapper = mount(MaskCard, { props: { mask } });
    expect(wrapper.text()).toContain('A test mask for testing');
  });

  it('shows "Normal" mode badge for normal mode', () => {
    const mask = createMask({ mode: 'normal' });
    const wrapper = mount(MaskCard, { props: { mask } });
    expect(wrapper.text()).toContain('Normal');
  });

  it('shows "Assist" mode badge for assist mode', () => {
    const mask = createMask({ mode: 'assist' });
    const wrapper = mount(MaskCard, { props: { mask } });
    expect(wrapper.text()).toContain('Assist');
  });

  it('shows "Read-only" badge when mask is readOnly', () => {
    const mask = createMask({ readOnly: true });
    const wrapper = mount(MaskCard, { props: { mask } });
    expect(wrapper.text()).toContain('Read-only');
  });

  it('does not show "Read-only" badge when mask is not readOnly', () => {
    const mask = createMask({ readOnly: false });
    const wrapper = mount(MaskCard, { props: { mask } });
    expect(wrapper.text()).not.toContain('Read-only');
  });

  it('shows active checkmark when active is true', () => {
    const mask = createMask();
    const wrapper = mount(MaskCard, { props: { mask, active: true } });
    // The checkmark SVG path is rendered in the active indicator div
    expect(wrapper.find('.ring-2').exists()).toBe(true);
  });

  it('does not show active ring when active is false', () => {
    const mask = createMask();
    const wrapper = mount(MaskCard, { props: { mask, active: false } });
    expect(wrapper.find('.ring-2').exists()).toBe(false);
  });

  it('renders a button element', () => {
    const mask = createMask();
    const wrapper = mount(MaskCard, { props: { mask } });
    expect(wrapper.find('button').exists()).toBe(true);
  });

  it('is not clickable when selectable is false', () => {
    const mask = createMask();
    const wrapper = mount(MaskCard, { props: { mask, selectable: false } });
    const button = wrapper.find('button');
    expect(button.attributes('disabled')).toBeDefined();
  });
});

import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import MaskSelectorButton from './MaskSelectorButton.vue';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'mask.selectMask': 'Select mask',
        'mask.title': 'Masks',
        'mask.noMasks': 'No masks available',
        'mask.manageMasks': 'Manage masks',
      };
      return map[key] ?? key;
    },
  }),
}));

vi.mock('lucide-vue-next', () => ({
  Settings: { name: 'Settings', template: '<span class="Settings" />' },
}));

const mockMasks = [
  { name: 'Default', icon: '😊', description: 'Default mask', mode: 'normal', readOnly: false },
  { name: 'Research', icon: '🔍', description: 'Research mode', mode: 'normal', readOnly: false },
];

const activeMask = { name: 'Default', icon: '😊', description: 'Default mask', mode: 'normal', readOnly: false };

let currentActiveMask = { ...activeMask };
let currentMasks = [...mockMasks];

vi.mock('../composables/useMasks', () => ({
  useMasks: () => ({
    masks: currentMasks,
    activeMask: currentActiveMask,
    switchTo: vi.fn(async (name: string) => {
      const found = currentMasks.find((m: { name: string }) => m.name === name);
      if (found) currentActiveMask = { ...found };
    }),
    refresh: vi.fn(async () => {}),
  }),
}));

describe('MaskSelectorButton', () => {
  it('renders the trigger button with active mask icon', () => {
    const wrapper = mount(MaskSelectorButton);
    const trigger = wrapper.find('button');
    expect(trigger.exists()).toBe(true);
    expect(trigger.text()).toContain('😊');
  });

  it('shows popover with mask list when trigger is clicked', async () => {
    const wrapper = mount(MaskSelectorButton);
    const trigger = wrapper.find('button');
    await trigger.trigger('click');

    // Popover should be visible
    expect(wrapper.text()).toContain('Masks');
    expect(wrapper.text()).toContain('Default');
    expect(wrapper.text()).toContain('Research');
  });

  it('shows a checkmark next to the active mask', async () => {
    const wrapper = mount(MaskSelectorButton);
    const trigger = wrapper.find('button');
    await trigger.trigger('click');

    // Active mask should have checkmark
    expect(wrapper.text()).toContain('✓');
  });

  it('shows "Manage masks" link in footer', async () => {
    const wrapper = mount(MaskSelectorButton);
    const trigger = wrapper.find('button');
    await trigger.trigger('click');

    expect(wrapper.text()).toContain('Manage masks');
  });

  it('shows empty state when no masks available', async () => {
    currentMasks = [];
    currentActiveMask = { name: '', icon: '😊', description: '', mode: 'normal', readOnly: false };

    const wrapper = mount(MaskSelectorButton);
    const trigger = wrapper.find('button');
    await trigger.trigger('click');

    expect(wrapper.text()).toContain('No masks available');

    // Restore
    currentMasks = [...mockMasks];
    currentActiveMask = { ...activeMask };
  });

  it('emits navigate-settings when manage button is clicked', async () => {
    const wrapper = mount(MaskSelectorButton);
    const trigger = wrapper.find('button');
    await trigger.trigger('click');

    const manageButton = wrapper.findAll('button').filter(b => b.text().includes('Manage masks'));
    expect(manageButton.length).toBeGreaterThan(0);
    await manageButton[0].trigger('click');

    expect(wrapper.emitted('navigate-settings')).toBeTruthy();
  });

  it('closes popover after selecting a mask', async () => {
    currentMasks = [...mockMasks];
    currentActiveMask = { ...activeMask };

    const wrapper = mount(MaskSelectorButton);
    const trigger = wrapper.find('button');
    await trigger.trigger('click');

    // Click on the research mask
    const items = wrapper.findAll('button').filter(b => b.text().includes('Research'));
    expect(items.length).toBeGreaterThan(0);
    await items[0].trigger('click');
    await flushPromises();

    // Popover should close (backdrop removed)
    expect(wrapper.find('.fixed').exists()).toBe(false);
  });
});

import { describe, it, expect, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import SectionGroupList from './SectionGroupList.vue';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('@lucide/vue', () => ({
  ChevronDown: { name: 'ChevronDown', template: '<span class="chevron-down" />' },
  ChevronRight: { name: 'ChevronRight', template: '<span class="chevron-right" />' },
  FileText: { name: 'FileText', template: '<span class="file-text" />' },
}));

const ALL_SECTIONS: Record<string, { status: 'owned' | 'tbd'; last_modified?: string | null }> = {
  identity: { status: 'owned', last_modified: '2026-07-05T12:00:00Z' },
  relationship: { status: 'owned', last_modified: '2026-07-05T11:00:00Z' },
  commitment: { status: 'tbd' },
  preferences: { status: 'tbd' },
  memory_md: { status: 'owned' },
  changelog: { status: 'owned' },
};

function mountList(selectedSection = 'identity' as const) {
  return mount(SectionGroupList, {
    props: {
      snapshot: { sections: ALL_SECTIONS },
      selectedSection,
    },
  });
}

describe('SectionGroupList', () => {
  it('renders 3 group headers and 6 canonical persona sections by default', () => {
    const wrapper = mountList();

    expect(wrapper.findAll('.group-header')).toHaveLength(3);
    expect(wrapper.findAll('.section-item')).toHaveLength(6);
  });

  it('starts with all groups expanded', () => {
    const wrapper = mountList();

    const headers = wrapper.findAll('.group-header');
    headers.forEach((header) => {
      expect(header.attributes('aria-expanded')).toBe('true');
    });
    expect(wrapper.findAll('.chevron-down')).toHaveLength(3);
    expect(wrapper.findAll('.chevron-right')).toHaveLength(0);
  });

  it('toggles group expansion and flips the chevron on header click', async () => {
    const wrapper = mountList();
    const firstHeader = wrapper.find('.group-header');

    await firstHeader.trigger('click');
    await nextTick();

    expect(firstHeader.attributes('aria-expanded')).toBe('false');
    expect(wrapper.findAll('.chevron-down')).toHaveLength(2);
    expect(wrapper.findAll('.chevron-right')).toHaveLength(1);

    const firstGroupItems = wrapper.findAll('.group-items')[0];
    expect((firstGroupItems.element as HTMLElement).style.display).toBe('none');
  });

  it('emits select with the clicked section name and applies active styling', async () => {
    const wrapper = mountList('identity');
    const items = wrapper.findAll('.section-item');

    await items[1].trigger('click');

    expect(wrapper.emitted('select')?.[0]).toEqual(['relationship']);
  });

  it('marks the selected section with active state and aria-current', () => {
    const wrapper = mountList('memory_md');

    const activeItem = wrapper.find('.section-item--active');
    expect(activeItem.exists()).toBe(true);
    expect(activeItem.attributes('aria-current')).toBe('true');
    expect(activeItem.text()).toContain('memory_md');

    const inactiveItem = wrapper.findAll('.section-item').find(
      (item) => !item.classes().includes('section-item--active')
    );
    expect(inactiveItem?.attributes('aria-current')).toBeUndefined();
  });

  it('renders owned and tbd status badges with distinct styling', () => {
    const wrapper = mountList();

    const ownedBadge = wrapper.findAll('.section-status--owned');
    const tbdBadge = wrapper.findAll('.section-status--tbd');

    expect(ownedBadge.length).toBeGreaterThan(0);
    expect(tbdBadge.length).toBeGreaterThan(0);
  });

  it('renders last_modified timestamps for sections that have them', () => {
    const wrapper = mountList();

    const updatedItems = wrapper.findAll('.section-updated');
    expect(updatedItems.length).toBe(2);

    const identityItem = wrapper.findAll('.section-item').find(
      (item) => item.text().includes('identity')
    );
    expect(identityItem?.find('.section-updated').exists()).toBe(true);
    expect(identityItem?.find('.section-updated').text()).toContain('2026');
  });
});

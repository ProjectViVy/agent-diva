import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import InstalledSkillsTab from './InstalledSkillsTab.vue';

const getSkills = vi.fn();
const deleteSkill = vi.fn();
const appConfirm = vi.fn();
const showAppToast = vi.fn();

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('../../api/desktop', () => ({
  getSkills: (...args: unknown[]) => getSkills(...args),
  deleteSkill: (...args: unknown[]) => deleteSkill(...args),
  uploadSkill: vi.fn(),
  isTauriRuntime: () => true,
}));

vi.mock('../../utils/appDialog', () => ({
  appConfirm: (...args: unknown[]) => appConfirm(...args),
}));

vi.mock('../../utils/appToast', () => ({
  showAppToast: (...args: unknown[]) => showAppToast(...args),
}));

function skillFixture(overrides: Record<string, unknown> = {}) {
  return {
    slug: 'evolution',
    name: 'evolution',
    description: 'Demo skill',
    source: 'home',
    enabled: true,
    always: false,
    available: true,
    active: true,
    content_hash: 'hash-1',
    updated_at: '2026-08-21T00:00:00Z',
    can_hard_delete: true,
    evolution_managed: false,
    path: '',
    can_delete: true,
    ...overrides,
  };
}

function mountTab() {
  return mount(InstalledSkillsTab);
}

describe('InstalledSkillsTab', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    deleteSkill.mockResolvedValue(undefined);
    appConfirm.mockResolvedValue(true);
  });

  it('shows the evolution-managed hint and no delete button for evolution skills', async () => {
    getSkills.mockResolvedValue([skillFixture({ evolution_managed: true })]);

    const wrapper = mountTab();
    await flushPromises();

    expect(wrapper.text()).toContain('general.skillManagedByEvolution');
    expect(wrapper.find('button.skills-btn-danger').exists()).toBe(false);
  });

  it('shows a delete button for marketplace/manual skills', async () => {
    getSkills.mockResolvedValue([skillFixture()]);

    const wrapper = mountTab();
    await flushPromises();

    expect(wrapper.text()).not.toContain('general.skillManagedByEvolution');
    const button = wrapper.find('button.skills-btn-danger');
    expect(button.exists()).toBe(true);
    expect(button.text()).toContain('general.deleteSkill');
  });

  it('deletes after confirmation and refreshes the list', async () => {
    getSkills.mockResolvedValue([skillFixture()]);

    const wrapper = mountTab();
    await flushPromises();

    await wrapper.find('button.skills-btn-danger').trigger('click');
    await flushPromises();

    expect(appConfirm).toHaveBeenCalledTimes(1);
    expect(deleteSkill).toHaveBeenCalledWith('evolution', 'hash-1');
    expect(getSkills).toHaveBeenCalledTimes(2);
  });

  it('does not delete when the confirmation is cancelled', async () => {
    appConfirm.mockResolvedValue(false);
    getSkills.mockResolvedValue([skillFixture()]);

    const wrapper = mountTab();
    await flushPromises();

    await wrapper.find('button.skills-btn-danger').trigger('click');
    await flushPromises();

    expect(deleteSkill).not.toHaveBeenCalled();
    expect(getSkills).toHaveBeenCalledTimes(1);
  });

  it('shows no action for builtin skills', async () => {
    getSkills.mockResolvedValue([
      skillFixture({ slug: 'built', name: 'built', source: 'builtin', can_hard_delete: false, can_delete: false }),
    ]);

    const wrapper = mountTab();
    await flushPromises();

    expect(wrapper.find('button.skills-btn-danger').exists()).toBe(false);
    expect(wrapper.text()).not.toContain('general.skillManagedByEvolution');
  });
});

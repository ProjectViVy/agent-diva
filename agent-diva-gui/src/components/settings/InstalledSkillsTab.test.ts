import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import InstalledSkillsTab from './InstalledSkillsTab.vue';
import type { InstalledSkill } from '../../api/settings';

const loadInstalledSkills = vi.fn();
const setSkillEnabled = vi.fn();

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('../../api/desktop', () => ({
  isTauriRuntime: () => true,
}));

vi.mock('../../api/settings', () => ({
  loadInstalledSkills: (...args: unknown[]) => loadInstalledSkills(...args),
  setSkillEnabled: (...args: unknown[]) => setSkillEnabled(...args),
}));

function skillFixture(overrides: Partial<InstalledSkill> = {}): InstalledSkill {
  return {
    name: 'demo-skill',
    description: 'Demo skill',
    origin: 'workspace',
    enabled: true,
    hash: 'hash-1',
    warnings: [],
    userInvocable: true,
    ...overrides,
  };
}

function mountTab() {
  return mount(InstalledSkillsTab);
}

describe('InstalledSkillsTab', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    loadInstalledSkills.mockResolvedValue([]);
    setSkillEnabled.mockResolvedValue(undefined);
  });

  it('lists installed skills with enabled state', async () => {
    loadInstalledSkills.mockResolvedValue([
      skillFixture({ name: 'caveman', enabled: true }),
      skillFixture({ name: 'off-skill', enabled: false }),
    ]);
    const wrapper = mountTab();
    await flushPromises();

    expect(loadInstalledSkills).toHaveBeenCalled();
    expect(wrapper.text()).toContain('caveman');
    expect(wrapper.text()).toContain('off-skill');
    expect(wrapper.text()).toContain('general.skillStatusActive');
    expect(wrapper.text()).toContain('general.skillStatusAvailable');
  });

  it('toggles a skill with its base hash', async () => {
    loadInstalledSkills.mockResolvedValue([skillFixture({ name: 'caveman', enabled: true, hash: 'h-7' })]);
    const wrapper = mountTab();
    await flushPromises();

    await wrapper.find('.skills-list-item .skills-btn').trigger('click');
    await flushPromises();

    expect(setSkillEnabled).toHaveBeenCalledWith('caveman', false, 'h-7');
  });

  it('shows the backend error and re-lists when the toggle conflicts', async () => {
    loadInstalledSkills.mockResolvedValue([skillFixture()]);
    setSkillEnabled.mockRejectedValue(new Error('conflict (-32009): stale hash'));
    const wrapper = mountTab();
    await flushPromises();

    await wrapper.find('.skills-list-item .skills-btn').trigger('click');
    await flushPromises();

    expect(setSkillEnabled).toHaveBeenCalled();
    expect(loadInstalledSkills.mock.calls.length).toBeGreaterThanOrEqual(2);
    expect(wrapper.text()).toContain('stale hash');
  });

  it('shows backend warnings on the skill row', async () => {
    loadInstalledSkills.mockResolvedValue([
      skillFixture({ warnings: ['unrecognized frontmatter key'] }),
    ]);
    const wrapper = mountTab();
    await flushPromises();

    expect(wrapper.text()).toContain('unrecognized frontmatter key');
  });

  it('filters by search query', async () => {
    loadInstalledSkills.mockResolvedValue([
      skillFixture({ name: 'alpha', description: 'first' }),
      skillFixture({ name: 'beta', description: 'second' }),
    ]);
    const wrapper = mountTab();
    await flushPromises();

    await wrapper.find('input.skills-search-input').setValue('beta');
    expect(wrapper.text()).toContain('beta');
    expect(wrapper.text()).not.toContain('alpha');
  });
});

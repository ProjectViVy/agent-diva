import { flushPromises, mount } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import MarketplaceTab from './MarketplaceTab.vue';

const searchMarketplaceSkills = vi.fn();
const installMarketplaceSkill = vi.fn();
const getSkills = vi.fn();

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('../../api/desktop', () => ({
  searchMarketplaceSkills: (...args: unknown[]) => searchMarketplaceSkills(...args),
  installMarketplaceSkill: (...args: unknown[]) => installMarketplaceSkill(...args),
  getSkills: (...args: unknown[]) => getSkills(...args),
  isTauriRuntime: () => true,
}));

function mountTab() {
  return mount(MarketplaceTab);
}

describe('MarketplaceTab', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    getSkills.mockResolvedValue([]);
    searchMarketplaceSkills.mockResolvedValue([]);
    installMarketplaceSkill.mockResolvedValue({ name: 'demo-skill' });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('shows the search prompt without calling the API for short queries', async () => {
    const wrapper = mountTab();
    await flushPromises();

    const input = wrapper.find('input.skills-search-input');
    await input.setValue('a');
    vi.advanceTimersByTime(400);
    await flushPromises();

    expect(searchMarketplaceSkills).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain('general.marketplaceSearchPrompt');
  });

  it('searches, sorts by installs, and marks installed skills', async () => {
    getSkills.mockResolvedValue([{ name: 'git-commit' }]);
    searchMarketplaceSkills.mockResolvedValue([
      { id: 'a/b/less-popular', name: 'less-popular', source: 'a/b', installs: 5 },
      { id: 'github/awesome-copilot/git-commit', name: 'git-commit', source: 'github/awesome-copilot', installs: 42795 },
    ]);

    const wrapper = mountTab();
    await flushPromises();

    const input = wrapper.find('input.skills-search-input');
    await input.setValue('commit');
    vi.advanceTimersByTime(400);
    await flushPromises();

    expect(searchMarketplaceSkills).toHaveBeenCalledWith('commit', 20);
    const cards = wrapper.findAll('.marketplace-card');
    expect(cards).toHaveLength(2);
    expect(cards[0].text()).toContain('git-commit');
    expect(cards[0].text()).toContain('general.installed');
    expect(cards[0].find('button').attributes('disabled')).toBeDefined();
    expect(cards[1].text()).toContain('less-popular');
    expect(cards[1].text()).toContain('general.install');
  });

  it('installs a skill by its marketplace id and refreshes the installed list', async () => {
    searchMarketplaceSkills.mockResolvedValue([
      { id: 'demo-owner/demo-repo/demo-skill', name: 'demo-skill', source: 'demo-owner/demo-repo', installs: 42 },
    ]);

    const wrapper = mountTab();
    await flushPromises();

    const input = wrapper.find('input.skills-search-input');
    await input.setValue('demo');
    vi.advanceTimersByTime(400);
    await flushPromises();

    await wrapper.find('.marketplace-card button').trigger('click');
    await flushPromises();

    expect(installMarketplaceSkill).toHaveBeenCalledWith('demo-owner/demo-repo/demo-skill');
    expect(getSkills).toHaveBeenCalledTimes(2);
  });

  it('surfaces install failures with a retryable error box', async () => {
    installMarketplaceSkill.mockRejectedValue(new Error('conflict'));
    searchMarketplaceSkills.mockResolvedValue([
      { id: 'demo-owner/demo-repo/demo-skill', name: 'demo-skill', source: 'demo-owner/demo-repo', installs: 42 },
    ]);

    const wrapper = mountTab();
    await flushPromises();

    const input = wrapper.find('input.skills-search-input');
    await input.setValue('demo');
    vi.advanceTimersByTime(400);
    await flushPromises();

    await wrapper.find('.marketplace-card button').trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('general.installFailed');
  });
});

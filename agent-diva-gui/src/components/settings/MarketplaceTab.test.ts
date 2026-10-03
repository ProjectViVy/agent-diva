import { flushPromises, mount } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import MarketplaceTab from './MarketplaceTab.vue';

const marketplaceSearch = vi.fn();
const marketplaceInstall = vi.fn();
const marketplaceFeatured = vi.fn();
const loadInstalledSkills = vi.fn();

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('../../api/desktop', () => ({
  isTauriRuntime: () => true,
}));

vi.mock('../../api/vivy/instance', () => ({
  vivyClient: {
    marketplaceSearch: (...args: unknown[]) => marketplaceSearch(...args),
    marketplaceInstall: (...args: unknown[]) => marketplaceInstall(...args),
    marketplaceFeatured: (...args: unknown[]) => marketplaceFeatured(...args),
  },
}));

vi.mock('../../api/settings', () => ({
  loadInstalledSkills: (...args: unknown[]) => loadInstalledSkills(...args),
}));

function mountTab() {
  return mount(MarketplaceTab);
}

describe('MarketplaceTab', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    loadInstalledSkills.mockResolvedValue([]);
    marketplaceSearch.mockResolvedValue({ skills: [] });
    marketplaceFeatured.mockResolvedValue({ skills: [], generated_at: '' });
    marketplaceInstall.mockResolvedValue({ outcome: 'created' });
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

    expect(marketplaceSearch).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain('general.marketplaceSearchPrompt');
  });

  it('searches, sorts by installs, and marks installed skills', async () => {
    loadInstalledSkills.mockResolvedValue([{ name: 'git-commit' }]);
    marketplaceSearch.mockResolvedValue({ skills: [
      { id: 'a/b/less-popular', name: 'less-popular', source: 'a/b', installs: 5 },
      { id: 'github/awesome-copilot/git-commit', name: 'git-commit', source: 'github/awesome-copilot', installs: 42795 },
    ] });

    const wrapper = mountTab();
    await flushPromises();

    const input = wrapper.find('input.skills-search-input');
    await input.setValue('commit');
    vi.advanceTimersByTime(400);
    await flushPromises();

    expect(marketplaceSearch).toHaveBeenCalledWith('commit', 20);
    const cards = wrapper.findAll('.marketplace-card');
    expect(cards).toHaveLength(2);
    expect(cards[0].text()).toContain('git-commit');
    expect(cards[0].text()).toContain('general.installed');
    expect(cards[0].find('button').attributes('disabled')).toBeDefined();
    expect(cards[1].text()).toContain('less-popular');
    expect(cards[1].text()).toContain('general.install');
  });

  it('installs a skill by its marketplace id and refreshes the installed list', async () => {
    marketplaceSearch.mockResolvedValue({ skills: [
      { id: 'demo-owner/demo-repo/demo-skill', name: 'demo-skill', source: 'demo-owner/demo-repo', installs: 42 },
    ] });

    const wrapper = mountTab();
    await flushPromises();

    const input = wrapper.find('input.skills-search-input');
    await input.setValue('demo');
    vi.advanceTimersByTime(400);
    await flushPromises();

    await wrapper.find('.marketplace-card button').trigger('click');
    await flushPromises();

    expect(marketplaceInstall).toHaveBeenCalledWith('demo-owner/demo-repo/demo-skill', 'create');
    expect(loadInstalledSkills).toHaveBeenCalledTimes(2);
  });

  it('surfaces install failures with a retryable error box', async () => {
    marketplaceInstall.mockRejectedValue(new Error('conflict'));
    marketplaceSearch.mockResolvedValue({ skills: [
      { id: 'demo-owner/demo-repo/demo-skill', name: 'demo-skill', source: 'demo-owner/demo-repo', installs: 42 },
    ] });

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

  it('shows the featured leaderboard sorted by installs when no search is active', async () => {
    marketplaceFeatured.mockResolvedValue({
      skills: [
        { id: 'a/b/less-popular', name: 'less-popular', source: 'a/b', installs: 5 },
        { id: 'vercel-labs/skills/find-skills', name: 'find-skills', source: 'vercel-labs/skills', installs: 846620 },
      ],
      generated_at: '2026-08-20T21:11:19+00:00',
    });

    const wrapper = mountTab();
    await flushPromises();

    expect(marketplaceFeatured).toHaveBeenCalled();
    expect(wrapper.text()).toContain('general.marketplaceFeaturedTitle');
    expect(wrapper.text()).toContain('general.marketplaceFeaturedSnapshot');
    const cards = wrapper.findAll('.marketplace-card');
    expect(cards).toHaveLength(2);
    expect(cards[0].text()).toContain('find-skills');
    expect(cards[1].text()).toContain('less-popular');
  });

  it('installs a featured skill by its marketplace id', async () => {
    marketplaceFeatured.mockResolvedValue({
      skills: [
        { id: 'demo-owner/demo-repo/demo-skill', name: 'demo-skill', source: 'demo-owner/demo-repo', installs: 42 },
      ],
      generated_at: '2026-08-20T21:11:19+00:00',
    });

    const wrapper = mountTab();
    await flushPromises();

    await wrapper.find('.marketplace-card button').trigger('click');
    await flushPromises();

    expect(marketplaceInstall).toHaveBeenCalledWith('demo-owner/demo-repo/demo-skill', 'create');
  });
});

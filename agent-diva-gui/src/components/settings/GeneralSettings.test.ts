import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import GeneralSettings from './GeneralSettings.vue';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('../../api/desktop', () => ({
  getConfigStatus: vi.fn(() => Promise.resolve(null)),
  startGateway: vi.fn(() => Promise.resolve()),
  wipeLocalData: vi.fn(() => Promise.resolve()),
}));

vi.mock('../../utils/localStorageAgentDiva', () => ({
  clearAgentDivaLocalStorage: vi.fn(),
  UI_CACHE_KEYS: [],
  UI_CACHE_PREFIXES: [],
}));

const prefs = {
  cleanMode: false,
  autoExpandReasoning: true,
  autoExpandToolDetails: true,
  showRawMetaByDefault: true,
};

describe('GeneralSettings chat display preferences', () => {
  it('temporarily disables auto-expand controls while preserving their values', async () => {
    const wrapper = mount(GeneralSettings, {
      props: { chatDisplayPrefs: prefs },
    });

    const checkboxes = wrapper.findAll('input[type="checkbox"]');
    expect(checkboxes).toHaveLength(5);
    await checkboxes[0].setValue(true);

    expect(checkboxes[1].element).toHaveProperty('disabled', true);
    expect(checkboxes[2].element).toHaveProperty('disabled', true);
    expect(checkboxes[3].element).toHaveProperty('disabled', true);
    expect(wrapper.emitted('save-chat-display-prefs')?.at(-1)?.[0]).toEqual({
      ...prefs,
      cleanMode: true,
    });
  });
});

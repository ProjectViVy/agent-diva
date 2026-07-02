import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SelfEvolutionSettings from './SelfEvolutionSettings.vue';
import { invoke } from '@tauri-apps/api/core';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../utils/appToast', () => ({
  showAppToast: vi.fn(),
}));

vi.mock('lucide-vue-next', () => ({
  Sparkles: { name: 'Sparkles', template: '<span />' },
  LoaderCircle: { name: 'LoaderCircle', template: '<span />' },
  BrainCircuit: { name: 'BrainCircuit', template: '<span />' },
  ShieldQuestion: { name: 'ShieldQuestion', template: '<span />' },
}));

describe('SelfEvolutionSettings governance safety', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockResolvedValue({
      enabled: true,
      autodream_frequency: 'weekly',
      trigger_threshold_sessions: 10,
      trigger_threshold_messages: 50,
      auto_merge_confidence: 0.95,
      require_confirmation_for: ['identity'],
    });
  });

  it('does not expose an enabled durable auto-merge control', async () => {
    const wrapper = mount(SelfEvolutionSettings);
    await flushPromises();

    expect(wrapper.find('[data-testid="self-evo-governance-notice"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="self-evo-auto-merge-disabled"]').exists()).toBe(true);
    expect(wrapper.find('input[type="range"]').exists()).toBe(false);
    expect(wrapper.text()).toContain('selfEvolution.reviewRequiredStatement');
  });
});

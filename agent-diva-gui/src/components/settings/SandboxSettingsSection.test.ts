import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SandboxSettingsSection from './SandboxSettingsSection.vue';

const { getCommandRules, setCommandRuleEnabled, deleteCommandRule } = vi.hoisted(() => ({
  getCommandRules: vi.fn(),
  setCommandRuleEnabled: vi.fn(),
  deleteCommandRule: vi.fn(),
}));

vi.mock('../../api/desktop', () => ({
  getSandboxConfig: vi.fn().mockResolvedValue({
    mode: 'workspace-write',
    approval_policy: 'on-failure',
    network_access: false,
    writable_roots: [],
    protected_paths: [],
    deny_patterns: [],
    timeout_seconds: 30,
  }),
  saveSandboxConfig: vi.fn(),
  getCommandRules: (...args: unknown[]) => getCommandRules(...args),
  setCommandRuleEnabled: (...args: unknown[]) => setCommandRuleEnabled(...args),
  deleteCommandRule: (...args: unknown[]) => deleteCommandRule(...args),
}));
vi.mock('../../utils/appDialog', () => ({ appConfirm: vi.fn().mockResolvedValue(true) }));
vi.mock('../../utils/appToast', () => ({ showAppToast: vi.fn() }));
vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}));

const rule = {
  id: 'rule-1',
  pattern: ['git', 'status'],
  decision: 'allow',
  enabled: true,
  source: 'approval',
  justification: 'Validated read-only command pattern',
  created_at: '2026-07-29T08:00:00Z',
  revision: 1,
};

describe('SandboxSettingsSection command rules', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getCommandRules.mockResolvedValue([rule]);
  });

  it('loads rules and uses revisioned enable updates', async () => {
    setCommandRuleEnabled.mockResolvedValue({ ...rule, enabled: false, revision: 2 });
    const wrapper = mount(SandboxSettingsSection);
    await flushPromises();
    expect(wrapper.text()).toContain('git status');

    await wrapper.find('.command-rule-row [role="switch"]').trigger('click');
    await flushPromises();
    expect(setCommandRuleEnabled).toHaveBeenCalledWith(rule, false);
    expect(wrapper.find('.command-rule-row [role="switch"]').attributes('aria-checked')).toBe('false');
  });

  it('deletes a confirmed rule without exposing an arbitrary add input', async () => {
    deleteCommandRule.mockResolvedValue(undefined);
    const wrapper = mount(SandboxSettingsSection);
    await flushPromises();
    await wrapper.find('.command-rule-row .sandbox-tag-remove').trigger('click');
    await flushPromises();
    expect(deleteCommandRule).toHaveBeenCalledWith(rule);
    expect(wrapper.find('.command-rule-row').exists()).toBe(false);
    expect(wrapper.find('input[placeholder*="command"]').exists()).toBe(false);
  });
});

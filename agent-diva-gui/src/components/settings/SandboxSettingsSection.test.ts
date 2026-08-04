import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SandboxSettingsSection from './SandboxSettingsSection.vue';

const { getCommandRules, setCommandRuleEnabled, deleteCommandRule, saveSandboxConfig, showAppToast } = vi.hoisted(() => ({
  getCommandRules: vi.fn(),
  setCommandRuleEnabled: vi.fn(),
  deleteCommandRule: vi.fn(),
  saveSandboxConfig: vi.fn(),
  showAppToast: vi.fn(),
}));

vi.mock('../../api/desktop', () => ({
  getSandboxConfig: vi.fn().mockResolvedValue({
    mode: 'workspace_write',
    approval_policy: 'on_failure',
    network_access: false,
    writable_roots: [],
    protected_paths: [],
    deny_patterns: [],
    timeout_seconds: 30,
  }),
  saveSandboxConfig: (...args: unknown[]) => saveSandboxConfig(...args),
  getCommandRules: (...args: unknown[]) => getCommandRules(...args),
  setCommandRuleEnabled: (...args: unknown[]) => setCommandRuleEnabled(...args),
  deleteCommandRule: (...args: unknown[]) => deleteCommandRule(...args),
}));
vi.mock('../../utils/appDialog', () => ({ appConfirm: vi.fn().mockResolvedValue(true) }));
vi.mock('../../utils/appToast', () => ({ showAppToast: (...args: unknown[]) => showAppToast(...args) }));
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

describe('SandboxSettingsSection save', () => {
  it('saves the config with snake_case enum values after changing the mode', async () => {
    saveSandboxConfig.mockResolvedValue(undefined);
    const wrapper = mount(SandboxSettingsSection);
    await flushPromises();

    await wrapper.findAll('select')[0].setValue('read_only');
    await wrapper.find('button.settings-btn-primary').trigger('click');
    await flushPromises();

    expect(saveSandboxConfig).toHaveBeenCalledWith({
      mode: 'read_only',
      approval_policy: 'on_failure',
      network_access: false,
      writable_roots: [],
      protected_paths: [],
      deny_patterns: [],
      timeout_seconds: 30,
    });
  });

  it('falls back to the default timeout when the input is cleared', async () => {
    saveSandboxConfig.mockResolvedValue(undefined);
    const wrapper = mount(SandboxSettingsSection);
    await flushPromises();

    await wrapper.find('input[type="number"]').setValue('');
    await wrapper.find('button.settings-btn-primary').trigger('click');
    await flushPromises();

    expect(saveSandboxConfig).toHaveBeenCalledWith(expect.objectContaining({ timeout_seconds: 60 }));
  });

  it('shows the backend error message when saving fails', async () => {
    saveSandboxConfig.mockRejectedValue(new Error('Invalid sandbox config: unknown variant'));
    const wrapper = mount(SandboxSettingsSection);
    await flushPromises();

    await wrapper.findAll('select')[1].setValue('unless_trusted');
    await wrapper.find('button.settings-btn-primary').trigger('click');
    await flushPromises();

    expect(showAppToast).toHaveBeenCalledWith(expect.stringContaining('unknown variant'), 'error');
  });
});

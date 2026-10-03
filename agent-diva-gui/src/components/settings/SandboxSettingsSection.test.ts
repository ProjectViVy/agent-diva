import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SandboxSettingsSection from './SandboxSettingsSection.vue';

const { settingsGet, settingsUpdate, showAppToast } = vi.hoisted(() => ({
  settingsGet: vi.fn(),
  settingsUpdate: vi.fn(),
  showAppToast: vi.fn(),
}));

const sandboxView = {
  default_preset: 'smart',
  config_default_preset: 'smart',
  deny_private_ips: true,
  allowed_domains: ['example.com'],
  workspace_root: '/srv/vivy/workspaces',
  execute_allowed_commands: ['git status'],
  approval_timeout_seconds: 60,
  config_approval_timeout_seconds: 30,
  approval_expiration_seconds: 300,
};

vi.mock('../../api/settings', () => ({
  loadSandboxSettings: vi.fn(async () => {
    const res = await settingsGet();
    if (!res.sandbox) throw new Error('no sandbox');
    return res.sandbox;
  }),
  saveSandboxSettings: vi.fn(async (update: unknown) => {
    const res = await settingsUpdate({ sandbox: update });
    if (!res.sandbox) throw new Error('no sandbox');
    return res.sandbox;
  }),
}));
vi.mock('../../utils/appToast', () => ({ showAppToast: (...args: unknown[]) => showAppToast(...args) }));
vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}));

describe('SandboxSettingsSection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    settingsGet.mockResolvedValue({ sandbox: { ...sandboxView } });
    settingsUpdate.mockImplementation(async () => ({ sandbox: { ...sandboxView, deny_private_ips: false } }));
  });

  it('loads the VIVY sandbox view into the form', async () => {
    const wrapper = mount(SandboxSettingsSection);
    await flushPromises();
    expect(settingsGet).toHaveBeenCalled();
    expect(wrapper.find('.sandbox-toggle').classes()).toContain('active');
    expect(wrapper.text()).toContain('example.com');
    expect(wrapper.text()).toContain('/srv/vivy/workspaces');
    expect(wrapper.find('.sandbox-preset-btn.active').text()).toBe('sandbox.presets.smart');
  });

  it('sends the full sandbox overlay on save', async () => {
    const wrapper = mount(SandboxSettingsSection);
    await flushPromises();
    await wrapper.find('.sandbox-toggle').trigger('click');
    const saveBtn = wrapper.findAll('button').find(b => b.text() === 'sandbox.save');
    await saveBtn!.trigger('click');
    await flushPromises();
    expect(settingsUpdate).toHaveBeenCalledWith({
      sandbox: {
        default_preset: 'smart',
        deny_private_ips: false,
        allowed_domains: ['example.com'],
        approval_timeout_seconds: 60,
      },
    });
    expect(showAppToast).toHaveBeenCalledWith('sandbox.saved', 'success');
  });

  it('surfaces backend load failure without defaults fabrication', async () => {
    settingsGet.mockRejectedValueOnce(new Error('settings are read-only'));
    const wrapper = mount(SandboxSettingsSection);
    await flushPromises();
    expect(wrapper.text()).toContain('settings are read-only');
    expect(wrapper.find('input[type="number"]').exists()).toBe(false);
  });
});

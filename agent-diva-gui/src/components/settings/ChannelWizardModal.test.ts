import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ChannelWizardModal from './ChannelWizardModal.vue';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}));

vi.mock('@lucide/vue', () => {
  const icon = (name: string) => ({ name, template: `<span class="${name}" />` });
  return {
    X: icon('X'),
    ChevronRight: icon('ChevronRight'),
    LoaderCircle: icon('LoaderCircle'),
    Check: icon('Check'),
    CircleAlert: icon('CircleAlert'),
    PlugZap: icon('PlugZap'),
    Lightbulb: icon('Lightbulb'),
    Eye: icon('Eye'),
    EyeOff: icon('EyeOff'),
    Mail: icon('Mail'),
    Globe: icon('Globe'),
  };
});

vi.mock('./TutorialModal.vue', () => ({
  default: { props: ['open', 'platform'], template: '<div class="tutorial-stub" />' },
}));

const mountWizard = (props: Record<string, unknown>) =>
  mount(ChannelWizardModal, {
    props,
    global: {
      mocks: { $t: (key: string) => key },
      stubs: {
        Teleport: { template: '<div><slot /></div>' },
        Transition: false,
      },
    },
  });

describe('ChannelWizardModal', () => {
  it('hydrates credentials and skips the platform step when opened for edit', async () => {
    const wrapper = mountWizard({
      open: true,
      initialData: {
        platform: 'feishu',
        credentials: { app_id: 'cli_x', app_secret: 'sec', enabled: false },
      },
    });
    await flushPromises();

    expect(wrapper.text()).toContain('channels.wizardEditTitle');
    expect(wrapper.text()).not.toContain('channels.choosePlatform');
    const appId = wrapper.findAll('input').find((input) => (input.element as HTMLInputElement).value === 'cli_x');
    expect(appId).toBeTruthy();
    wrapper.unmount();
  });

  it('starts from platform selection when adding a channel', async () => {
    const wrapper = mountWizard({ open: true });
    await flushPromises();

    expect(wrapper.text()).toContain('channels.choosePlatform');
    expect(wrapper.text()).toContain('飞书');
    wrapper.unmount();
  });

  it('does not keep previous credentials after close and reopen as add', async () => {
    const wrapper = mountWizard({
      open: true,
      initialData: { platform: 'telegram', credentials: { token: 'keep-me' } },
    });
    await flushPromises();
    expect(wrapper.text()).toContain('channels.wizardEditTitle');

    await wrapper.setProps({ open: false, initialData: undefined });
    await flushPromises();
    await wrapper.setProps({ open: true, initialData: undefined });
    await flushPromises();

    expect(wrapper.text()).toContain('channels.choosePlatform');
    expect(wrapper.text()).not.toContain('keep-me');
    wrapper.unmount();
  });
});

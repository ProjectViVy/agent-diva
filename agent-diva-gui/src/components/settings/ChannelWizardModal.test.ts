import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';
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
  it('runs the real async test step and still permits saving after failure', async () => {
    const onTest = vi.fn(async () => ({ success: false, message: 'offline' }));
    const onComplete = vi.fn(async () => undefined);
    const wrapper = mountWizard({
      open: true,
      availablePlatforms: ['telegram'],
      onTest,
      onComplete,
    });
    await flushPromises();

    await wrapper.find('.platform-card').trigger('click');
    await wrapper.find('.wizard-footer .wizard-btn-primary').trigger('click');
    await wrapper.find('input[type="password"]').setValue('bot-token');
    await wrapper.find('.wizard-footer .wizard-btn-primary').trigger('click');
    await flushPromises();

    expect(onTest).toHaveBeenCalledWith(
      expect.objectContaining({
        platform: 'telegram',
        credentials: expect.objectContaining({ token: 'bot-token' }),
      }),
    );
    expect(wrapper.find('.test-result.failed').text()).toContain('offline');

    const testNext = wrapper.find('.wizard-footer .wizard-btn-primary');
    expect((testNext.element as HTMLButtonElement).disabled).toBe(false);
    expect(testNext.text()).toContain('channels.wizardSave');
    await testNext.trigger('click');
    await flushPromises();
    expect(onComplete).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain('channels.wizardDone');

    await wrapper.find('.wizard-footer .wizard-btn-primary').trigger('click');
    await flushPromises();
    expect(onComplete).toHaveBeenCalledTimes(1);
    expect(wrapper.emitted('update:open')?.at(-1)).toEqual([false]);
    wrapper.unmount();
  });

  it('shows a local saving state while persisting from the test step', async () => {
    let resolveSave: (() => void) | undefined;
    const onTest = vi.fn(async () => ({ success: true, message: 'ok' }));
    const onComplete = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          resolveSave = resolve;
        }),
    );
    const wrapper = mountWizard({
      open: true,
      availablePlatforms: ['telegram'],
      onTest,
      onComplete,
    });
    await flushPromises();

    await wrapper.find('.platform-card').trigger('click');
    await wrapper.find('.wizard-footer .wizard-btn-primary').trigger('click');
    await wrapper.find('input[type="password"]').setValue('bot-token');
    await wrapper.find('.wizard-footer .wizard-btn-primary').trigger('click');
    await flushPromises();

    const saveButton = wrapper.find('.wizard-footer .wizard-btn-primary');
    const saveClick = saveButton.trigger('click');
    await nextTick();

    expect(saveButton.text()).toContain('channels.wizardSaving');
    expect((saveButton.element as HTMLButtonElement).disabled).toBe(true);
    expect(saveButton.find('.animate-spin').exists()).toBe(true);

    expect(resolveSave).toBeTypeOf('function');
    resolveSave?.();
    await saveClick;
    await flushPromises();
    expect(wrapper.text()).toContain('channels.wizardDone');
    wrapper.unmount();
  });

  it('invalidates a completed test when credentials change', async () => {
    const onTest = vi.fn(async () => ({ success: true, message: 'ok' }));
    const wrapper = mountWizard({
      open: true,
      availablePlatforms: ['telegram'],
      onTest,
    });
    await flushPromises();

    await wrapper.find('.platform-card').trigger('click');
    await wrapper.find('.wizard-footer .wizard-btn-primary').trigger('click');
    await wrapper.find('input[type="password"]').setValue('first-token');
    await wrapper.find('.wizard-footer .wizard-btn-primary').trigger('click');
    await flushPromises();
    expect(wrapper.find('.test-result.success').exists()).toBe(true);

    await wrapper.find('.wizard-footer .wizard-btn-secondary').trigger('click');
    await wrapper.find('input[type="password"]').setValue('second-token');
    await flushPromises();
    await wrapper.find('.wizard-footer .wizard-btn-primary').trigger('click');
    await flushPromises();

    expect(onTest).toHaveBeenCalledTimes(2);
    expect(wrapper.find('.test-result.success').exists()).toBe(true);
    wrapper.unmount();
  });

  it('keeps the form open when the completion callback fails', async () => {
    const onTest = vi.fn(async () => ({ success: true, message: 'ok' }));
    const onComplete = vi
      .fn<() => Promise<void>>()
      .mockRejectedValueOnce(new Error('save failed'))
      .mockResolvedValueOnce(undefined);
    const wrapper = mountWizard({
      open: true,
      availablePlatforms: ['telegram'],
      onTest,
      onComplete,
    });
    await flushPromises();

    await wrapper.find('.platform-card').trigger('click');
    await wrapper.find('.wizard-footer .wizard-btn-primary').trigger('click');
    await wrapper.find('input[type="password"]').setValue('bot-token');
    await wrapper.find('.wizard-footer .wizard-btn-primary').trigger('click');
    await flushPromises();
    await wrapper.find('.wizard-footer .wizard-btn-primary').trigger('click');
    await flushPromises();

    expect(onComplete).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain('save failed');
    expect(wrapper.find('.wizard-overlay').exists()).toBe(true);

    await wrapper.find('.wizard-footer .wizard-btn-primary').trigger('click');
    await flushPromises();
    expect(onComplete).toHaveBeenCalledTimes(2);
    expect(wrapper.text()).toContain('channels.wizardDone');

    await wrapper.find('.wizard-footer .wizard-btn-primary').trigger('click');
    expect(wrapper.emitted('update:open')?.at(-1)).toEqual([false]);
    wrapper.unmount();
  });

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

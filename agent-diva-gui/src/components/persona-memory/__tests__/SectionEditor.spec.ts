import { describe, it, expect, vi, beforeEach } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import SectionEditor from '../SectionEditor.vue';
import en from '../../../locales/en';
import * as desktop from '../../../api/desktop';
import * as appDialog from '../../../utils/appDialog';

vi.mock('../../../api/desktop', () => ({
  writeLaputaSection: vi.fn(),
}));

vi.mock('../../../utils/appDialog', () => ({
  appConfirm: vi.fn(),
}));

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: { en },
});

function factory(props: Record<string, unknown> = {}) {
  return mount(SectionEditor, {
    props: {
      sectionName: 'identity',
      displayName: en.laputa.sections.identity,
      initialContent: '',
      ...props,
    },
    global: {
      plugins: [i18n],
    },
  });
}

describe('SectionEditor', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('initializes the textarea from initialContent', () => {
    const wrapper = factory({ initialContent: '# Identity\n\nHello' });
    expect(wrapper.find('textarea').element.value).toBe('# Identity\n\nHello');
  });

  it('enables the Save button when content changes', async () => {
    const wrapper = factory({ initialContent: 'original' });
    const button = wrapper.find('.section-editor-save-btn');

    expect(button.attributes('disabled')).toBeDefined();

    await wrapper.find('textarea').setValue('changed');
    await flushPromises();

    expect(button.attributes('disabled')).toBeUndefined();
  });

  it('disables the Save button when content reverts to original', async () => {
    const wrapper = factory({ initialContent: 'original' });
    const textarea = wrapper.find('textarea');
    await textarea.setValue('changed');
    await textarea.setValue('original');
    await flushPromises();

    expect(wrapper.find('.section-editor-save-btn').attributes('disabled')).toBeDefined();
  });

  it('shows a confirmation dialog when saving non-empty existing content', async () => {
    (appDialog.appConfirm as ReturnType<typeof vi.fn>).mockResolvedValue(true);
    (desktop.writeLaputaSection as ReturnType<typeof vi.fn>).mockResolvedValue({ changelog_id: '1' });

    const wrapper = factory({ initialContent: 'existing content' });
    await wrapper.find('textarea').setValue('modified content');
    await flushPromises();

    await wrapper.find('.section-editor-save-btn').trigger('click');
    await flushPromises();

    expect(appDialog.appConfirm).toHaveBeenCalledTimes(1);
  });

  it('skips confirmation and saves when original content is empty', async () => {
    (desktop.writeLaputaSection as ReturnType<typeof vi.fn>).mockResolvedValue({ changelog_id: '1' });

    const wrapper = factory({ initialContent: '' });
    await wrapper.find('textarea').setValue('new content');
    await flushPromises();

    await wrapper.find('.section-editor-save-btn').trigger('click');
    await flushPromises();

    expect(appDialog.appConfirm).not.toHaveBeenCalled();
    expect(desktop.writeLaputaSection).toHaveBeenCalledWith('identity', 'new content');
  });

  it('calls writeLaputaSection with the current draftContent after confirmation', async () => {
    (appDialog.appConfirm as ReturnType<typeof vi.fn>).mockResolvedValue(true);
    (desktop.writeLaputaSection as ReturnType<typeof vi.fn>).mockResolvedValue({ changelog_id: '2' });

    const wrapper = factory({ initialContent: 'existing' });
    await wrapper.find('textarea').setValue('final draft');
    await flushPromises();

    await wrapper.find('.section-editor-save-btn').trigger('click');
    await flushPromises();

    expect(desktop.writeLaputaSection).toHaveBeenCalledWith('identity', 'final draft');
  });

  it('does not save when the user cancels the confirmation', async () => {
    (appDialog.appConfirm as ReturnType<typeof vi.fn>).mockResolvedValue(false);

    const wrapper = factory({ initialContent: 'existing' });
    await wrapper.find('textarea').setValue('changed');
    await flushPromises();

    await wrapper.find('.section-editor-save-btn').trigger('click');
    await flushPromises();

    expect(desktop.writeLaputaSection).not.toHaveBeenCalled();
  });

  it('resets dirty state and emits saved after a successful save', async () => {
    (appDialog.appConfirm as ReturnType<typeof vi.fn>).mockResolvedValue(true);
    (desktop.writeLaputaSection as ReturnType<typeof vi.fn>).mockResolvedValue({ changelog_id: '3' });

    const wrapper = factory({ initialContent: 'existing' });
    await wrapper.find('textarea').setValue('saved content');
    await flushPromises();

    await wrapper.find('.section-editor-save-btn').trigger('click');
    await flushPromises();

    expect(wrapper.emitted('saved')).toHaveLength(1);
    expect(wrapper.emitted('saved')![0]).toEqual(['identity']);
    expect(wrapper.find('.section-editor-save-btn').attributes('disabled')).toBeDefined();
  });

  it('preserves draftContent and shows an error when save fails', async () => {
    (appDialog.appConfirm as ReturnType<typeof vi.fn>).mockResolvedValue(true);
    (desktop.writeLaputaSection as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('Network error'));

    const wrapper = factory({ initialContent: 'existing' });
    await wrapper.find('textarea').setValue('changed content');
    await flushPromises();

    await wrapper.find('.section-editor-save-btn').trigger('click');
    await flushPromises();

    expect(wrapper.find('textarea').element.value).toBe('changed content');
    expect(wrapper.find('.section-editor-save-error').exists()).toBe(true);
    expect(wrapper.text()).toContain('Network error');
    expect(wrapper.find('.section-editor-save-btn').attributes('disabled')).toBeUndefined();
  });

  it('disables the Save button and shows saving label while saving', async () => {
    let resolveSave: (value: unknown) => void = () => void 0;
    const savePromise = new Promise((resolve) => {
      resolveSave = resolve;
    });
    (desktop.writeLaputaSection as ReturnType<typeof vi.fn>).mockReturnValue(savePromise);
    (appDialog.appConfirm as ReturnType<typeof vi.fn>).mockResolvedValue(true);

    const wrapper = factory({ initialContent: 'existing' });
    await wrapper.find('textarea').setValue('saving test');
    await flushPromises();

    const clickPromise = wrapper.find('.section-editor-save-btn').trigger('click');
    await flushPromises();

    const button = wrapper.find('.section-editor-save-btn');
    expect(button.attributes('disabled')).toBeDefined();
    expect(button.text()).toContain(en.laputa.saving);
    expect(wrapper.find('.spin').exists()).toBe(true);

    resolveSave({ changelog_id: '4' });
    await clickPromise;
    await flushPromises();

    expect(button.text()).toContain(en.laputa.save);
  });
});

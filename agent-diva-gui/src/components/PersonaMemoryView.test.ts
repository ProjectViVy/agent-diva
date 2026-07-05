import { describe, it, expect, vi, beforeEach } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import PersonaMemoryView from './PersonaMemoryView.vue';
import * as desktop from '../api/desktop';
import * as appToast from '../utils/appToast';
import * as appDialog from '../utils/appDialog';
import en from '../locales/en';

vi.mock('../api/desktop', async () => {
  const actual = await vi.importActual<typeof desktop>('../api/desktop');
  return {
    ...actual,
    getLaputaSection: vi.fn(),
    getLaputaSnapshot: vi.fn(),
    writeLaputaSection: vi.fn(),
    isTauriRuntime: vi.fn(() => true),
  };
});

vi.mock('../utils/appToast', async () => {
  const actual = await vi.importActual<typeof appToast>('../utils/appToast');
  return {
    ...actual,
    showAppToast: vi.fn(),
  };
});

const appConfirm = vi.fn(() => Promise.resolve(true));
vi.mock('../utils/appDialog', () => ({
  appConfirm: (...args: unknown[]) => appConfirm(...args),
}));

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: { en },
});

function makeSection(name: desktop.LaputaSectionName, content: string): desktop.LaputaSection {
  return {
    name,
    status: 'owned',
    content,
    metadata: {},
    last_modified: '2026-07-05T12:00:00Z',
    version: '1',
  };
}

function makeSnapshot(): desktop.LaputaSnapshot {
  return {
    schema_version: '1',
    sections: {
      identity: makeSection('identity', 'initial content'),
      relationship: makeSection('relationship', 'relationship content'),
    },
    changed_sections: [],
    updated_at: '2026-07-05T12:00:00Z',
    server_time: '2026-07-05T12:00:00Z',
  };
}

function factory() {
  return mount(PersonaMemoryView, {
    global: {
      plugins: [i18n],
    },
  });
}

describe('PersonaMemoryView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    appConfirm.mockReset();
    appConfirm.mockResolvedValue(true);
    (desktop.getLaputaSection as ReturnType<typeof vi.fn>).mockImplementation((name: desktop.LaputaSectionName) =>
      Promise.resolve(makeSection(name, `${name} content`)),
    );
    (desktop.getLaputaSnapshot as ReturnType<typeof vi.fn>).mockResolvedValue(makeSnapshot());
  });

  it('reloads section, snapshot, and shows success toast after a successful save', async () => {
    (desktop.writeLaputaSection as ReturnType<typeof vi.fn>).mockResolvedValue({ changelog_id: '1' });

    const wrapper = factory();
    await flushPromises();

    const textarea = wrapper.find('textarea');
    await textarea.setValue('updated content');
    await flushPromises();

    await wrapper.find('.section-editor-save-btn').trigger('click');
    await flushPromises();

    expect(desktop.writeLaputaSection).toHaveBeenCalledWith('identity', 'updated content');
    expect(desktop.getLaputaSection).toHaveBeenCalledWith('identity');
    expect(desktop.getLaputaSnapshot).toHaveBeenCalled();
    expect(appToast.showAppToast).toHaveBeenCalledWith(en.laputa.saved, 'success');

    const saveButton = wrapper.find('.section-editor-save-btn');
    expect(saveButton.attributes('disabled')).toBeDefined();
  });

  it('shows an error toast and preserves the draft when save fails', async () => {
    (desktop.writeLaputaSection as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('disk full'));

    const wrapper = factory();
    await flushPromises();

    const textarea = wrapper.find('textarea');
    await textarea.setValue('updated content');
    await flushPromises();

    await wrapper.find('.section-editor-save-btn').trigger('click');
    await flushPromises();

    expect(desktop.writeLaputaSection).toHaveBeenCalledWith('identity', 'updated content');
    expect(appToast.showAppToast).toHaveBeenCalledWith(
      en.laputa.saveFailed.replace('{message}', 'disk full'),
      'error',
    );
    expect(textarea.element.value).toBe('updated content');

    const saveButton = wrapper.find('.section-editor-save-btn');
    expect(saveButton.attributes('disabled')).toBeUndefined();
  });

  it('prompts before switching sections when dirty and preserves state on cancel', async () => {
    appConfirm.mockResolvedValue(false);

    const wrapper = factory();
    await flushPromises();

    const textarea = wrapper.find('textarea');
    await textarea.setValue('updated content');
    await flushPromises();

    const items = wrapper.findAll('.section-item');
    const relationship = items.find((el) => el.text().includes('Relationship'));
    expect(relationship).toBeDefined();
    await relationship!.trigger('click');
    await flushPromises();

    expect(appConfirm).toHaveBeenCalledWith(
      en.laputa.confirmDiscard.message,
      expect.objectContaining({
        title: en.laputa.confirmDiscard.title,
        confirmLabel: en.laputa.confirmDiscard.discard,
        cancelLabel: en.laputa.confirmDiscard.cancel,
      }),
    );
    expect(textarea.element.value).toBe('updated content');
    expect(wrapper.find('.section-item--active').text()).toContain('Identity');
  });

  it('switches sections and resets dirty state when user discards changes', async () => {
    appConfirm.mockResolvedValue(true);

    const wrapper = factory();
    await flushPromises();

    const textarea = wrapper.find('textarea');
    await textarea.setValue('updated content');
    await flushPromises();

    const items = wrapper.findAll('.section-item');
    const relationship = items.find((el) => el.text().includes('Relationship'));
    expect(relationship).toBeDefined();
    await relationship!.trigger('click');
    await flushPromises();

    expect(appConfirm).toHaveBeenCalledWith(
      en.laputa.confirmDiscard.message,
      expect.objectContaining({
        title: en.laputa.confirmDiscard.title,
        confirmLabel: en.laputa.confirmDiscard.discard,
        cancelLabel: en.laputa.confirmDiscard.cancel,
      }),
    );
    await flushPromises();
    expect(desktop.getLaputaSection).toHaveBeenCalledWith('relationship');
    await flushPromises();
    expect(wrapper.find('.section-item--active').text()).toContain('Relationship');
    expect(wrapper.find('textarea').element.value).toBe('relationship content');
    expect(wrapper.find('.section-editor-save-btn').attributes('disabled')).toBeDefined();
  });
});

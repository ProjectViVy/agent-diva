import { describe, it, expect, vi, beforeEach } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import HistoryModal from './HistoryModal.vue';
import * as desktop from '../../api/desktop';
import en from '../../locales/en';

vi.mock('../../api/desktop', async () => {
  const actual = await vi.importActual<typeof desktop>('../../api/desktop');
  return {
    ...actual,
    listLaputaChangelog: vi.fn(),
  };
});

const clipboardWriteText = vi.fn(() => Promise.resolve());
Object.defineProperty(window, 'navigator', {
  value: {
    clipboard: {
      writeText: clipboardWriteText,
    },
  },
  writable: true,
  configurable: true,
});

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: { en },
});

function makeRecord(id: string, overrides: Partial<desktop.ChangelogRecord> = {}): desktop.ChangelogRecord {
  return {
    id,
    action: 'apply',
    target_section: 'identity',
    before: 'before',
    after: 'after content ' + id,
    diff: '',
    proposal_id: null,
    audit_event_id: null,
    reverted: false,
    stale: false,
    created_at: '2026-07-05T12:00:00Z',
    applied_by: 'tester',
    ...overrides,
  };
}

function factory(props: { open: boolean; sectionName: desktop.LaputaSectionName }) {
  return mount(HistoryModal, {
    props,
    global: {
      plugins: [i18n],
    },
    attachTo: document.body,
  });
}

function getCopyButtons(): HTMLElement[] {
  return Array.from(document.body.querySelectorAll('.history-modal-copy'));
}

describe('HistoryModal', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    document.body.innerHTML = '';
    (desktop.listLaputaChangelog as ReturnType<typeof vi.fn>).mockResolvedValue({
      items: [makeRecord('1'), makeRecord('2', { after: 'second version' })],
      total: 2,
      page: 1,
      page_size: 50,
      has_more: false,
    });
  });

  it('fetches and renders changelog records when opened', async () => {
    const wrapper = factory({ open: true, sectionName: 'identity' });
    await flushPromises();

    expect(desktop.listLaputaChangelog).toHaveBeenCalledWith({ section: 'identity', limit: 50 });
    expect(document.body.textContent).toContain('apply');
    expect(document.body.textContent).toContain('tester');
    expect(document.body.textContent).toContain('after content 1');
    expect(document.body.textContent).toContain('second version');
    wrapper.unmount();
  });

  it('copies record.after and swaps button label for 1 second', async () => {
    vi.useFakeTimers();
    const wrapper = factory({ open: true, sectionName: 'identity' });
    await flushPromises();

    const buttons = getCopyButtons();
    expect(buttons.length).toBe(2);
    buttons[0].click();
    await flushPromises();

    expect(clipboardWriteText).toHaveBeenCalledWith('after content 1');
    expect(getCopyButtons()[0].textContent).toBe(en.laputa.historyModal.copied);

    vi.advanceTimersByTime(1000);
    await flushPromises();

    expect(getCopyButtons()[0].textContent).toBe(en.laputa.historyModal.copy);
    vi.useRealTimers();
    wrapper.unmount();
  });

  it('emits close when Escape is pressed', async () => {
    const wrapper = factory({ open: true, sectionName: 'identity' });
    await flushPromises();

    const event = new KeyboardEvent('keydown', { key: 'Escape' });
    document.dispatchEvent(event);
    await flushPromises();

    expect(wrapper.emitted('close')).toHaveLength(1);
    wrapper.unmount();
  });

  it('emits close when scrim is clicked', async () => {
    const wrapper = factory({ open: true, sectionName: 'identity' });
    await flushPromises();

    const scrim = document.body.querySelector('.history-modal-scrim');
    expect(scrim).not.toBeNull();
    scrim!.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await flushPromises();

    expect(wrapper.emitted('close')).toHaveLength(1);
    wrapper.unmount();
  });

  it('emits close when X button is clicked', async () => {
    const wrapper = factory({ open: true, sectionName: 'identity' });
    await flushPromises();

    const closeBtn = document.body.querySelector('.history-modal-close');
    expect(closeBtn).not.toBeNull();
    closeBtn!.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await flushPromises();

    expect(wrapper.emitted('close')).toHaveLength(1);
    wrapper.unmount();
  });
});

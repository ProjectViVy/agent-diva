import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import NotebookView from './NotebookView.vue';
import { showAppToast } from '../utils/appToast';

const invokeMock = vi.fn();

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, unknown>) =>
      params ? `${key}:${JSON.stringify(params)}` : key,
  }),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock('../utils/appDialog', () => ({
  appConfirm: vi.fn(() => Promise.resolve(true)),
}));

vi.mock('../utils/appToast', () => ({
  showAppToast: vi.fn(),
}));

vi.mock('lucide-vue-next', () => {
  const icon = (name: string) => ({ name, template: `<span class="${name}" />` });
  return {
    BookOpen: icon('BookOpen'),
    Calendar: icon('Calendar'),
    FileText: icon('FileText'),
    RefreshCw: icon('RefreshCw'),
    ShieldCheck: icon('ShieldCheck'),
    Brain: icon('Brain'),
    StickyNote: icon('StickyNote'),
    Loader2: icon('Loader2'),
    AlertCircle: icon('AlertCircle'),
    Inbox: icon('Inbox'),
  };
});

function enableTauriInternals() {
  Object.defineProperty(window, '__TAURI_INTERNALS__', {
    configurable: true,
    value: {},
  });
}

describe('NotebookView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    enableTauriInternals();
    invokeMock.mockResolvedValue([]);
  });

  it('defaults to daily and offers a trigger action in the empty state', async () => {
    const wrapper = mount(NotebookView);
    await flushPromises();

    expect(invokeMock).toHaveBeenCalledWith('get_notebook_reports', { period: 'daily' });
    const emptyButton = wrapper.find('.notebook-empty-state .notebook-retry-btn');
    expect(emptyButton.exists()).toBe(true);
    expect(emptyButton.text()).toContain('notebook.generateDaily');

    invokeMock.mockResolvedValueOnce({ id: 'run-1' });
    await emptyButton.trigger('click');
    await flushPromises();

    expect(invokeMock).toHaveBeenCalledWith('trigger_notebook_report_generation', {
      period: 'daily',
    });
    expect(showAppToast).toHaveBeenCalled();
  });

  it('shows a truncated banner when the selected report was clipped by the backend', async () => {
    invokeMock.mockResolvedValueOnce([
      {
        id: 'daily:2026-06-14',
        period: 'daily',
        date: '2026-06-14',
        title: 'Daily Reflection',
        summary: 'Summary',
        content: '# Daily Reflection\n\nSummary\n',
        isTruncated: true,
        displayedLineCount: 5000,
        originalLineCount: 6400,
      },
    ]);

    const wrapper = mount(NotebookView);
    await flushPromises();

    const banner = wrapper.find('.notebook-truncated-banner');
    expect(banner.exists()).toBe(true);
    expect(banner.text()).toContain('notebook.truncatedNotice');
    expect(wrapper.find('.notebook-detail-title').text()).toContain('Daily Reflection');
  });

  it('uses proposal preview and creation commands instead of legacy solidification commands', async () => {
    invokeMock
      .mockResolvedValueOnce([
        {
          id: 'daily:2026-06-14',
          period: 'daily',
          date: '2026-06-14',
          title: 'Daily Reflection',
          summary: 'Summary',
          content: '# Daily Reflection\n\nSummary\n',
        },
      ])
      .mockResolvedValueOnce({
        action: 'sop',
        proposalType: 'sop_create',
        targetSection: 'identity',
        extractedSummary: 'Summary',
        evidenceRefs: [{ id: 'evidence-daily-2026-06-14', source: 'report', uri: 'report://daily' }],
        riskLevel: 'medium',
        reviewStatus: 'pending_review',
        proposedPatch: '{}',
        needsAttentionReason: null,
      })
      .mockResolvedValueOnce({
        id: 'proposal-1',
        state: 'pending_review',
      });

    const wrapper = mount(NotebookView);
    await flushPromises();

    const buttons = wrapper.findAll('.notebook-action-btn');
    expect(buttons).toHaveLength(3);

    await buttons[0].trigger('click');
    await flushPromises();

    expect(invokeMock).toHaveBeenCalledWith('preview_notebook_report_proposal', {
      reportId: 'daily:2026-06-14',
      action: 'sop',
    });
    expect(
      invokeMock.mock.calls.some(([command]) => command === 'solidify_report_as_sop'),
    ).toBe(false);
    expect(
      invokeMock.mock.calls.some(([command]) => command === 'solidify_report_as_skill'),
    ).toBe(false);
    expect(
      invokeMock.mock.calls.some(([command]) => command === 'update_memory_from_report'),
    ).toBe(false);

    const submit = wrapper.find('.notebook-preview-primary');
    expect(submit.exists()).toBe(true);
    await submit.trigger('click');
    await flushPromises();

    expect(invokeMock).toHaveBeenCalledWith('create_notebook_report_proposal', {
      reportId: 'daily:2026-06-14',
      action: 'sop',
    });
    expect(showAppToast).toHaveBeenCalledWith('notebook.proposalSuccess', 'success');
  });
});

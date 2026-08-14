import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import NotebookView from './NotebookView.vue';
import { showAppToast } from '../utils/appToast';

const invokeMock = vi.fn();

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string, params?: Record<string, unknown>) => params ? `${key}:${JSON.stringify(params)}` : key }),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));
vi.mock('../utils/appToast', () => ({ showAppToast: vi.fn() }));
vi.mock('@lucide/vue', () => Object.fromEntries(
  ['BookOpen', 'Calendar', 'FileText', 'RefreshCw', 'Loader2', 'AlertCircle', 'Inbox', 'Search', 'Link2', 'CheckSquare']
    .map((name) => [name, { name, template: `<span class="${name}" />` }]),
));

function enableTauriInternals() {
  Object.defineProperty(window, '__TAURI_INTERNALS__', { configurable: true, value: {} });
}

const report = {
  id: 'daily:2026-06-14',
  period: 'daily',
  date: '2026-06-14',
  title: 'Daily Reflection',
  summary: 'Summary',
  content: '# Daily Reflection\n\nSummary\n',
};

describe('NotebookView reports-only surface', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    enableTauriInternals();
    invokeMock.mockResolvedValue([]);
  });

  it('defaults to daily and offers generation in the empty state', async () => {
    const wrapper = mount(NotebookView);
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith('get_notebook_reports', { period: 'daily' });
    const generate = wrapper.find('.notebook-empty-state .notebook-retry-btn');
    expect(generate.text()).toContain('notebook.generateDaily');
    invokeMock.mockResolvedValueOnce({ id: 'run-1' });
    await generate.trigger('click');
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith('trigger_notebook_report_generation', { period: 'daily' });
    expect(showAppToast).toHaveBeenCalled();
  });

  it('shows clipped-report metadata', async () => {
    invokeMock.mockResolvedValueOnce([{ ...report, isTruncated: true, displayedLineCount: 5000, originalLineCount: 6400 }]);
    const wrapper = mount(NotebookView);
    await flushPromises();
    expect(wrapper.find('.notebook-truncated-banner').text()).toContain('notebook.truncatedNotice');
    expect(wrapper.find('.notebook-detail-title').text()).toContain('Daily Reflection');
  });

  it('keeps report search but exposes no SOP, Skill, or Memory proposal action', async () => {
    invokeMock.mockResolvedValueOnce([report]);
    const wrapper = mount(NotebookView);
    await flushPromises();

    expect(wrapper.find('.notebook-session-search-input').exists()).toBe(true);
    expect(wrapper.findAll('.notebook-action-btn')).toHaveLength(0);
    expect(wrapper.find('.notebook-preview').exists()).toBe(false);
    expect(invokeMock.mock.calls.some(([command]) => String(command).includes('proposal'))).toBe(false);
  });

  it('regenerates the selected report period', async () => {
    invokeMock
      .mockResolvedValueOnce([report])
      .mockResolvedValueOnce({ id: 'run-daily' })
      .mockResolvedValueOnce([report]);
    const wrapper = mount(NotebookView);
    await flushPromises();
    await wrapper.find('.notebook-regenerate-btn').trigger('click');
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith('trigger_notebook_report_generation', { period: 'daily' });
  });
});

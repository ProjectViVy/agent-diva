import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import AuditPage from './AuditPage.vue';
import StructuredEventsTab from './StructuredEventsTab.vue';

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

vi.mock('../../../utils/appToast', () => ({
  showAppToast: vi.fn(),
}));

vi.mock('@lucide/vue', () => ({
  Copy: { name: 'Copy', template: '<span class="Copy" />' },
  Activity: { name: 'Activity', template: '<span class="Activity" />' },
  Monitor: { name: 'Monitor', template: '<span class="Monitor" />' },
}));

describe('audit page i18n', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    invokeMock.mockResolvedValue([]);
  });

  it('renders title and aria labels from i18n keys', async () => {
    const wrapper = mount(AuditPage);
    await flushPromises();

    expect(wrapper.text()).toContain('auditPage.title');
    expect(wrapper.find('[role="region"]').attributes('aria-label')).toBe('auditPage.regionLabel');
    expect(wrapper.find('[role="tablist"]').attributes('aria-label')).toBe('auditPage.tabListLabel');
    expect(wrapper.find('input[type="date"]').attributes('aria-label')).toBe('auditPage.datePickerLabel');
  });

  it('renders structured empty state from i18n keys', () => {
    const wrapper = mount(StructuredEventsTab, {
      props: {
        events: [],
        loading: false,
        selectedDate: '2026-07-05',
      },
    });

    expect(wrapper.text()).toContain('auditPage.structured.emptyTitle');
    expect(wrapper.text()).toContain('auditPage.structured.emptyHint');
    expect(wrapper.attributes('aria-label')).toBe('auditPage.structured.panelLabel');
  });

  it('loads each log source only when its tab becomes active', async () => {
    const wrapper = mount(AuditPage);
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith('get_audit_events', expect.any(Object));

    const tabs = wrapper.findAll('[role="tab"]');
    await tabs[1].trigger('click');
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith('get_gateway_log_lines', expect.objectContaining({ maxLines: 500 }));

    await tabs[2].trigger('click');
    await flushPromises();
    expect(invokeMock).toHaveBeenCalledWith('get_gui_log_lines', expect.objectContaining({ maxLines: 500 }));
  });
});

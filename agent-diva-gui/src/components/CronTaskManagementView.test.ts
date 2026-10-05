import { flushPromises, mount } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import CronTaskManagementView from './CronTaskManagementView.vue';

const { listCronJobs } = vi.hoisted(() => ({ listCronJobs: vi.fn() }));

vi.mock('../api/settings', () => ({ listCronJobs }));
vi.mock('../api/vivy/instance', () => ({
  vivyClient: {
    cronCreate: vi.fn(),
    cronUpdate: vi.fn(),
    cronTrigger: vi.fn(),
    cronStop: vi.fn(),
    cronDelete: vi.fn(),
  },
}));
vi.mock('../utils/appDialog', () => ({ appConfirm: vi.fn() }));
vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}));

describe('CronTaskManagementView loading states', () => {
  beforeEach(() => {
    listCronJobs.mockReset();
    Object.defineProperty(window, '_wails', {
      configurable: true,
      value: { environment: {} },
    });
  });

  afterEach(() => {
    delete (window as Window & { _wails?: unknown })._wails;
  });

  it('shows a retryable load error instead of an empty list after the initial request fails', async () => {
    listCronJobs
      .mockRejectedValueOnce(new Error('gateway unavailable'))
      .mockResolvedValueOnce([]);

    const wrapper = mount(CronTaskManagementView);
    await flushPromises();

    expect(wrapper.find('.cron-load-error-state').text()).toContain('gateway unavailable');
    expect(wrapper.find('.cron-empty-state').exists()).toBe(false);

    await wrapper.get('.cron-load-error-state button').trigger('click');
    await flushPromises();

    expect(wrapper.find('.cron-load-error-state').exists()).toBe(false);
    expect(wrapper.find('.cron-empty-state').exists()).toBe(true);
    wrapper.unmount();
  });
});

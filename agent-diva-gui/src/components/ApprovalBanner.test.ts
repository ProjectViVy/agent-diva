import { mount } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ApprovalBanner from './ApprovalBanner.vue';
import type { CommandApprovalRequest } from '../api/desktop';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, unknown>) =>
      params ? `${key}:${JSON.stringify(params)}` : key,
  }),
}));

const request: CommandApprovalRequest = {
  approval_id: 'approval-1',
  command: 'echo harmless',
  cwd: 'C:\\workspace',
  reason: 'sandbox denied',
  scope: {
    channel: 'gui',
    chat_id: 'chat-1',
    session_key: 'gui:chat-1',
  },
  created_at: '2026-07-29T08:00:00.000Z',
  timeout_seconds: 300,
};

afterEach(() => {
  vi.useRealTimers();
});

describe('ApprovalBanner', () => {
  it('renders command context and emits all three explicit decisions', async () => {
    vi.setSystemTime(new Date('2026-07-29T08:01:00.000Z'));
    const wrapper = mount(ApprovalBanner, { props: { request, active: true } });

    expect(wrapper.text()).toContain(request.command);
    expect(wrapper.text()).toContain(request.cwd);
    expect(wrapper.text()).toContain(request.reason);
    expect(wrapper.text()).toContain(request.scope.session_key);

    const buttons = wrapper.findAll('.actions button');
    await buttons[0].trigger('click');
    await buttons[1].trigger('click');
    await buttons[2].trigger('click');
    expect(wrapper.emitted('respond')).toEqual([
      [{ approval_id: request.approval_id, decision: 'reject' }],
      [{ approval_id: request.approval_id, decision: 'approve_once' }],
      [{ approval_id: request.approval_id, decision: 'approve_session' }],
    ]);
  });

  it('requires locating the source session before approval', async () => {
    const wrapper = mount(ApprovalBanner, { props: { request, active: false } });
    expect(wrapper.find('.actions').exists()).toBe(false);
    await wrapper.find('.inactive-action button').trigger('click');
    expect(wrapper.emitted('locate')).toEqual([[request.scope.session_key]]);
  });

  it('does not emit a rejection when the server-owned deadline expires', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-07-29T08:04:59.000Z'));
    const wrapper = mount(ApprovalBanner, { props: { request, active: true } });
    await vi.advanceTimersByTimeAsync(2_000);

    expect(wrapper.classes()).toContain('expired');
    expect(wrapper.emitted('respond')).toBeUndefined();
    expect(wrapper.findAll('.actions button').every((button) => button.attributes('disabled') !== undefined))
      .toBe(true);
  });
});

import { mount } from '@vue/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import fixture from '../../../agent-diva-manager/tests/fixtures/approval_contract_v1.json';
import ApprovalCenterCard from './ApprovalCenterCard.vue';
import type { ApprovalView } from '../api/approvals';

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }));

const approval = {
  ...fixture.approval,
  expires_at: '2030-01-01T00:00:00Z',
} as ApprovalView;

afterEach(() => vi.useRealTimers());

describe('ApprovalCenterCard', () => {
  it('shows non-color status, scope, TTL, and explicit grant decisions', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2029-12-31T23:59:00Z'));
    const wrapper = mount(ApprovalCenterCard, { props: { approval, detail: approval } });

    expect(wrapper.text()).toContain('approvalCenter.status.pending');
    expect(wrapper.text()).toContain('gui:chat');
    expect(wrapper.text()).toContain('60s');
    await wrapper.get('select').setValue('session');
    await wrapper.get('button.allow').trigger('click');
    expect(wrapper.emitted('decide')?.[0]).toEqual([{ approval, decision: 'allow', grant: 'session' }]);
  });

  it('fails closed for uncertain outcomes and only offers refresh', () => {
    const wrapper = mount(ApprovalCenterCard, { props: { approval, outcomeUnknown: true } });
    expect(wrapper.get('button.allow').attributes('disabled')).toBeDefined();
    expect(wrapper.text()).toContain('approvalCenter.outcomeUnknown');
    expect(wrapper.find('.approval-refresh').exists()).toBe(true);
  });

  it('disables high-risk Memory approval without evidence', () => {
    const memory = {
      ...approval,
      domain: 'memory',
      risk: 'high',
      evidence: [],
      presentation: { title: 'Memory change' },
    } as ApprovalView;
    const wrapper = mount(ApprovalCenterCard, { props: { approval: memory } });
    expect(wrapper.get('button.allow').attributes('disabled')).toBeDefined();
    expect(wrapper.text()).toContain('approvalCenter.missingEvidence');
  });
});

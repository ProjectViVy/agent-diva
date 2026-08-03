import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { afterEach, describe, expect, it, vi } from 'vitest';
import fixture from '../../../agent-diva-manager/tests/fixtures/approval_contract_v1.json';
import ApprovalCenterDrawer from './ApprovalCenterDrawer.vue';
import type { ApprovalView } from '../api/approvals';

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }));

const command = { ...fixture.approval, expires_at: '2030-01-01T00:00:00Z' } as ApprovalView;
const memory = {
  ...command,
  request_id: 'memory-1',
  domain: 'memory',
  risk: 'critical',
  resource: { ...command.resource, session_id: 'gui:memory', resource_id: 'proposal-1' },
  presentation: { title: 'Memory change' },
} as ApprovalView;

afterEach(() => {
  document.body.innerHTML = '';
});

describe('ApprovalCenterDrawer', () => {
  it('does not render a floating page-level trigger button', () => {
    mount(ApprovalCenterDrawer, {
      attachTo: document.body,
      props: { open: false, approvals: [command, memory], details: {} },
    });
    expect(document.body.querySelector('.approval-center-trigger')).toBeNull();
    expect(document.body.querySelector('.approval-center-drawer')).toBeNull();
  });

  it('sorts highest risk first when the drawer is open', () => {
    mount(ApprovalCenterDrawer, {
      attachTo: document.body,
      props: { open: true, approvals: [command, memory], details: {} },
    });
    const cards = [...document.body.querySelectorAll('.approval-center-card h3')].map((node) => node.textContent);
    expect(cards).toEqual(['Memory change', 'Command execution']);
  });

  it('filters by domain and exposes keyboard-sized close controls', async () => {
    mount(ApprovalCenterDrawer, {
      attachTo: document.body,
      props: { open: true, approvals: [command, memory], details: {} },
    });
    const domain = document.body.querySelector('.drawer-filters select') as HTMLSelectElement;
    domain.value = 'memory';
    domain.dispatchEvent(new Event('change'));
    await nextTick();
    expect(document.body.querySelectorAll('.approval-center-card')).toHaveLength(1);
    expect(document.body.querySelector('.drawer-header-actions button')).not.toBeNull();
  });
});

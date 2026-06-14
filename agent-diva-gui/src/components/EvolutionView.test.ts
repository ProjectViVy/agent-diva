import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import EvolutionView from './EvolutionView.vue';
import {
  applyLaputaProposal,
  editLaputaProposal,
  getLaputaSection,
  listLaputaChangelog,
  listLaputaProposals,
  pollLaputaEvents,
  rollbackLaputaChangelog,
  transitionLaputaProposal,
} from '../api/desktop';
import { appConfirm } from '../utils/appDialog';
import { showAppToast } from '../utils/appToast';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, unknown>) =>
      params ? `${key}:${JSON.stringify(params)}` : key,
  }),
}));

vi.mock('lucide-vue-next', () => ({
  AlertCircle: { name: 'AlertCircle', template: '<span class="AlertCircle" />' },
  AlertTriangle: { name: 'AlertTriangle', template: '<span class="AlertTriangle" />' },
  Archive: { name: 'Archive', template: '<span class="Archive" />' },
  Check: { name: 'Check', template: '<span class="Check" />' },
  ClipboardList: { name: 'ClipboardList', template: '<span class="ClipboardList" />' },
  Edit3: { name: 'Edit3', template: '<span class="Edit3" />' },
  ExternalLink: { name: 'ExternalLink', template: '<span class="ExternalLink" />' },
  FileClock: { name: 'FileClock', template: '<span class="FileClock" />' },
  FileDiff: { name: 'FileDiff', template: '<span class="FileDiff" />' },
  FileSearch: { name: 'FileSearch', template: '<span class="FileSearch" />' },
  GitBranch: { name: 'GitBranch', template: '<span class="GitBranch" />' },
  History: { name: 'History', template: '<span class="History" />' },
  RefreshCw: { name: 'RefreshCw', template: '<span class="RefreshCw" />' },
  RotateCcw: { name: 'RotateCcw', template: '<span class="RotateCcw" />' },
  ShieldAlert: { name: 'ShieldAlert', template: '<span class="ShieldAlert" />' },
  ShieldCheck: { name: 'ShieldCheck', template: '<span class="ShieldCheck" />' },
  X: { name: 'X', template: '<span class="X" />' },
}));

vi.mock('../utils/appDialog', () => ({
  appConfirm: vi.fn(() => Promise.resolve(true)),
}));

vi.mock('../utils/appToast', () => ({
  showAppToast: vi.fn(),
}));

vi.mock('../api/desktop', () => ({
  listLaputaProposals: vi.fn(),
  pollLaputaEvents: vi.fn(),
  getLaputaSection: vi.fn(),
  listLaputaChangelog: vi.fn(),
  transitionLaputaProposal: vi.fn(),
  applyLaputaProposal: vi.fn(),
  editLaputaProposal: vi.fn(),
  rollbackLaputaChangelog: vi.fn(),
}));

const baseProposal = {
  id: 'proposal-1',
  created_at: '2026-06-14T00:00:00Z',
  updated_at: '2026-06-14T00:00:00Z',
  created_by: 'autodream',
  proposal_type: 'memory_patch',
  target_section: 'memory_md',
  evidence_refs: [],
  proposed_patch: 'append memory',
  risk_level: 'high',
  state: 'pending_review',
  source_run_id: null,
};

const section = {
  name: 'memory_md',
  status: 'owned',
  content: 'current memory',
  metadata: {},
  last_modified: '2026-06-14T00:00:00Z',
  version: 'v1',
};

const changelogPage = {
  items: [
    {
      id: 'log-1',
      action: 'apply',
      target_section: 'memory_md',
      before: 'old',
      after: 'new',
      diff: '@@ -1 +1 @@',
      proposal_id: 'proposal-1',
      audit_event_id: null,
      reverted: false,
      stale: false,
      created_at: '2026-06-14T01:00:00Z',
      applied_by: 'tester',
    },
  ],
  total: 1,
  page: 1,
  page_size: 5,
  has_more: false,
};

function mountView() {
  return mount(EvolutionView);
}

describe('EvolutionView governance detail', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(listLaputaProposals).mockResolvedValue([baseProposal]);
    vi.mocked(pollLaputaEvents).mockResolvedValue([]);
    vi.mocked(getLaputaSection).mockResolvedValue(section);
    vi.mocked(listLaputaChangelog).mockResolvedValue(changelogPage);
    vi.mocked(transitionLaputaProposal).mockResolvedValue({
      ...baseProposal,
      state: 'approved',
    });
    vi.mocked(applyLaputaProposal).mockResolvedValue({
      proposal: { ...baseProposal, state: 'applied' },
      changelog: changelogPage.items[0],
      audit_event: {
        id: 'audit-1',
        kind: 'proposal_applied',
        actor: 'gui',
        proposal_id: 'proposal-1',
        target_section: 'memory_md',
        message: 'ok',
        created_at: '2026-06-14T01:00:00Z',
      },
      rollback_request: {
        changelog_id: 'log-1',
        requested_by: 'gui',
        reason: 'ok',
        requested_at: '2026-06-14T01:00:00Z',
      },
    });
    vi.mocked(editLaputaProposal).mockResolvedValue(baseProposal);
    vi.mocked(rollbackLaputaChangelog).mockResolvedValue({
      changelog: changelogPage.items[0],
      audit_event: {
        id: 'audit-2',
        kind: 'rollback_applied',
        actor: 'gui',
        proposal_id: 'proposal-1',
        target_section: 'memory_md',
        message: 'ok',
        created_at: '2026-06-14T01:10:00Z',
      },
    });
  });

  it('disables approval for high-risk proposal with missing evidence', async () => {
    const wrapper = mountView();
    await flushPromises();

    const approveButtons = wrapper.findAll('button').filter((button) =>
      button.text().includes('evolution.actions.approveApply'),
    );
    expect(approveButtons[0]?.attributes('disabled')).toBeDefined();
  });

  it('includes target section in reject confirmation', async () => {
    vi.mocked(listLaputaProposals).mockResolvedValue([
      { ...baseProposal, evidence_refs: [{ id: 'e1', source: 'report', uri: 'file://x', created_at: '2026-06-14T00:00:00Z' }] },
    ]);
    const wrapper = mountView();
    await flushPromises();

    const rejectButton = wrapper.findAll('button').find((button) =>
      button.text().includes('evolution.actions.reject'),
    );
    await rejectButton?.trigger('click');

    expect(appConfirm).toHaveBeenCalledWith(
      expect.stringContaining('"target":"memory_md"'),
      expect.any(Object),
    );
  });

  it('keeps detail visible when apply fails', async () => {
    vi.mocked(listLaputaProposals).mockResolvedValue([
      { ...baseProposal, evidence_refs: [{ id: 'e1', source: 'report', uri: 'file://x', created_at: '2026-06-14T00:00:00Z' }], risk_level: 'medium' },
    ]);
    vi.mocked(applyLaputaProposal).mockRejectedValue(new Error('apply failed'));
    const wrapper = mountView();
    await flushPromises();

    const approveButton = wrapper.findAll('button').find((button) =>
      button.text().includes('evolution.actions.approveApply'),
    );
    await approveButton?.trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('proposal-1');
    expect(showAppToast).toHaveBeenCalledWith('apply failed', 'error', 3600);
  });

  it('keeps responsive inbox/detail shell classes', async () => {
    const wrapper = mountView();
    await flushPromises();

    const inbox = wrapper.find('[data-testid="evolution-inbox-shell"]');
    expect(inbox.classes()).toContain('evolution-inbox-shell');
    expect(inbox.classes()).toContain('evolution-inbox-responsive');
  });
});

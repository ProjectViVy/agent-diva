import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import EvolutionView from './EvolutionView.vue';
import {
  applyLaputaProposal,
  cancelAutoDreamRun,
  decideLaputaProposal,
  editLaputaProposal,
  getLaputaSection,
  getEvolutionHealth,
  getAutoDreamRunStatus,
  getSelfEvolutionConfig,
  listAutoDreamRunRecords,
  listAutoDreamRunEvents,
  listLaputaChangelog,
  listLaputaProposals,
  listRecallFeedback,
  pollLaputaEvents,
  rollbackLaputaChangelog,
  transitionLaputaProposal,
  triggerAutoDream,
} from '../api/desktop';
import { appConfirm } from '../utils/appDialog';
import { showAppToast } from '../utils/appToast';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, unknown>) =>
      params ? `${key}:${JSON.stringify(params)}` : key,
  }),
}));

vi.mock('@lucide/vue', () => ({
  AlertCircle: { name: 'AlertCircle', template: '<span class="AlertCircle" />' },
  AlertTriangle: { name: 'AlertTriangle', template: '<span class="AlertTriangle" />' },
  Archive: { name: 'Archive', template: '<span class="Archive" />' },
  Check: { name: 'Check', template: '<span class="Check" />' },
  ChevronDown: { name: 'ChevronDown', template: '<span class="ChevronDown" />' },
  ChevronUp: { name: 'ChevronUp', template: '<span class="ChevronUp" />' },
  ClipboardList: { name: 'ClipboardList', template: '<span class="ClipboardList" />' },
  Clock3: { name: 'Clock3', template: '<span class="Clock3" />' },
  Edit3: { name: 'Edit3', template: '<span class="Edit3" />' },
  Eye: { name: 'Eye', template: '<span class="Eye" />' },
  EyeOff: { name: 'EyeOff', template: '<span class="EyeOff" />' },
  ExternalLink: { name: 'ExternalLink', template: '<span class="ExternalLink" />' },
  FileClock: { name: 'FileClock', template: '<span class="FileClock" />' },
  FileDiff: { name: 'FileDiff', template: '<span class="FileDiff" />' },
  FileSearch: { name: 'FileSearch', template: '<span class="FileSearch" />' },
  Filter: { name: 'Filter', template: '<span class="Filter" />' },
  GitBranch: { name: 'GitBranch', template: '<span class="GitBranch" />' },
  History: { name: 'History', template: '<span class="History" />' },
  Inbox: { name: 'Inbox', template: '<span class="Inbox" />' },
  Pencil: { name: 'Pencil', template: '<span class="Pencil" />' },
  RefreshCw: { name: 'RefreshCw', template: '<span class="RefreshCw" />' },
  RotateCcw: { name: 'RotateCcw', template: '<span class="RotateCcw" />' },
  Search: { name: 'Search', template: '<span class="Search" />' },
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
  listAutoDreamRunRecords: vi.fn(),
  listAutoDreamRunEvents: vi.fn(),
  getAutoDreamRunStatus: vi.fn(),
  getSelfEvolutionConfig: vi.fn(),
  pollLaputaEvents: vi.fn(),
  getLaputaSection: vi.fn(),
  listLaputaChangelog: vi.fn(),
  transitionLaputaProposal: vi.fn(),
  decideLaputaProposal: vi.fn(),
  applyLaputaProposal: vi.fn(),
  editLaputaProposal: vi.fn(),
  rollbackLaputaChangelog: vi.fn(),
  getEvolutionHealth: vi.fn(),
  listRecallFeedback: vi.fn(),
  triggerAutoDream: vi.fn(),
  cancelAutoDreamRun: vi.fn(),
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
  governance: {
    proposal_id: 'proposal-1',
    request_id: 'request-1',
    request_version: 1,
    status: 'pending',
    policy: 'require_human',
    receipt: null,
  },
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

function mountViewWithProps(props?: Record<string, unknown>) {
  return mount(EvolutionView, { props });
}

describe('EvolutionView governance detail', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
    vi.mocked(listLaputaProposals).mockResolvedValue([baseProposal]);
    vi.mocked(listAutoDreamRunRecords).mockRejectedValue(new Error('not implemented'));
    vi.mocked(getEvolutionHealth).mockResolvedValue({
      status: 'ok',
      version: '0.5.0',
      memory: {
        authority_mode: 'typed',
        status: 'ready',
        store_revision: 7,
        record_count: 3,
        tombstone_count: 1,
      },
    });
    vi.mocked(listRecallFeedback).mockResolvedValue([]);
    vi.mocked(triggerAutoDream).mockResolvedValue({
      id: 'run-new',
      started_at: '2026-06-14T00:00:00Z',
      state: 'pending',
      trigger: 'manual',
      proposal_ids: [],
    });
    vi.mocked(cancelAutoDreamRun).mockImplementation(async (id) => ({
      id,
      started_at: '2026-06-14T00:00:00Z',
      completed_at: '2026-06-14T00:01:00Z',
      state: 'cancelled',
      trigger: 'manual',
      proposal_ids: [],
    }));
    vi.mocked(getAutoDreamRunStatus).mockImplementation(async (id) => ({
      id,
      started_at: '2026-06-14T00:00:00Z',
      state: 'running',
      trigger: 'manual',
      proposal_ids: [],
      orchestration: { phase: 'reflecting', attempt: 1 },
    }));
    vi.mocked(listAutoDreamRunEvents).mockResolvedValue([]);
    vi.mocked(getSelfEvolutionConfig).mockResolvedValue({
      enabled: true,
      autodream_frequency: 'weekly',
      trigger_threshold_sessions: 10,
      trigger_threshold_messages: 50,
      auto_merge_confidence: 0.95,
      require_confirmation_for: ['identity', 'sop'],
    });
    vi.mocked(pollLaputaEvents).mockResolvedValue([]);
    vi.mocked(getLaputaSection).mockResolvedValue(section);
    vi.mocked(listLaputaChangelog).mockResolvedValue(changelogPage);
    vi.mocked(transitionLaputaProposal).mockResolvedValue({
      ...baseProposal,
      state: 'approved',
    });
    vi.mocked(decideLaputaProposal).mockImplementation(async (id, payload) => ({
      proposal: {
        ...baseProposal,
        id,
        state: payload.decision === 'allow' ? 'approved' : 'rejected',
      },
      governance: {
        ...baseProposal.governance,
        proposal_id: id,
        status: payload.decision === 'allow' ? 'allowed' : 'denied',
        request_version: 2,
      },
    }));
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

  it('labels the automated vertical pipeline honestly', async () => {
    const wrapper = mountView();
    await flushPromises();

    const notice = wrapper.get('[data-testid="evolution-stage-notice"]');
    expect(notice.text()).toContain('evolution.stageNotice.title');
    expect(notice.text()).toContain('evolution.stageNotice.desc');
  });

  it('shows typed workspace status and triggers a manual run', async () => {
    const wrapper = mountView();
    await flushPromises();

    expect(wrapper.find('[data-testid="evolution-workspace-status"]').text()).toContain('typed');
    expect(wrapper.find('[data-testid="evolution-workspace-status"]').text()).toContain('7');

    await wrapper.find('[data-testid="evolution-tab-runs"]').trigger('click');
    await flushPromises();
    await wrapper.find('[data-testid="trigger-autodream"]').trigger('click');
    await flushPromises();

    expect(triggerAutoDream).toHaveBeenCalledWith('manual');
    expect(showAppToast).toHaveBeenCalledWith('evolution.runs.triggerSuccess', 'success');
  });

  it('refreshes persisted AutoDream runs before the runs tab is opened', async () => {
    vi.mocked(listAutoDreamRunRecords).mockResolvedValue([
      {
        id: 'run-history',
        started_at: '2026-06-14T00:00:00Z',
        completed_at: '2026-06-14T00:00:01Z',
        state: 'completed',
        trigger: 'manual',
        summary: 'completed with no eligible candidates',
        proposal_ids: [],
      },
    ]);

    const wrapper = mountView();
    await flushPromises();

    expect(listAutoDreamRunRecords).toHaveBeenCalledTimes(1);
    await wrapper.find('[data-testid="evolution-tab-runs"]').trigger('click');
    await flushPromises();

    expect(wrapper.find('.evolution-record-card').exists()).toBe(true);
    expect(wrapper.text()).toContain('completed with no eligible candidates');
    expect(wrapper.find('.evolution-record-card dd[title="2026-06-14T00:00:00Z"]').text()).not.toBe(
      '2026-06-14T00:00:00Z',
    );
    expect(listAutoDreamRunRecords).toHaveBeenCalledTimes(1);
  });

  it('opens a read-only live monitor for a run and renders its progress events', async () => {
    vi.mocked(listAutoDreamRunRecords).mockResolvedValue([
      {
        id: 'run-monitor',
        started_at: '2026-06-14T00:00:00Z',
        state: 'running',
        trigger: 'manual',
        proposal_ids: [],
      },
    ]);
    vi.mocked(getAutoDreamRunStatus).mockResolvedValue({
      id: 'run-monitor',
      started_at: '2026-06-14T00:00:00Z',
      state: 'running',
      trigger: 'manual',
      proposal_ids: [],
      orchestration: {
        schema_version: 1,
        phase: 'reflecting',
        attempt: 1,
        deadline_at: '2026-06-14T00:02:00Z',
        updated_at: '2026-06-14T00:00:05Z',
      },
    });
    vi.mocked(listAutoDreamRunEvents).mockResolvedValue([
      {
        id: 'evt-1',
        run_id: 'run-monitor',
        kind: 'reflection_started',
        message: 'Bounded provider request started',
        created_at: '2026-06-14T00:00:05Z',
      },
    ]);

    const wrapper = mountView();
    await flushPromises();
    await wrapper.find('[data-testid="evolution-tab-runs"]').trigger('click');
    await wrapper.find('[data-testid="monitor-autodream-run-monitor"]').trigger('click');
    await flushPromises();

    expect(getAutoDreamRunStatus).toHaveBeenCalledWith('run-monitor');
    expect(listAutoDreamRunEvents).toHaveBeenCalledWith('run-monitor');
    expect(wrapper.find('[data-testid="autodream-monitor-events"]').text()).toContain(
      'Bounded provider request started',
    );
  });

  it('edits proposal content and invalidates the prior revision through backend edit', async () => {
    const wrapper = mountView();
    await flushPromises();

    const editButton = wrapper
      .findAll('button')
      .find((button) => button.text().includes('evolution.actions.edit'));
    await editButton?.trigger('click');
    await wrapper.find('[data-testid="proposal-edit-textarea"]').setValue('revised memory');
    await wrapper.find('[data-testid="proposal-edit-save"]').trigger('click');
    await flushPromises();

    expect(editLaputaProposal).toHaveBeenCalledWith('proposal-1', {
      proposed_patch: 'revised memory',
      updated_at: '2026-06-14T00:00:00Z',
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

  it('rejects proposal through the inbox action', async () => {
    vi.mocked(listLaputaProposals).mockResolvedValue([
      { ...baseProposal, evidence_refs: [{ id: 'e1', source: 'report', uri: 'file://x', created_at: '2026-06-14T00:00:00Z' }] },
    ]);
    const wrapper = mountView();
    await flushPromises();

    const rejectButton = wrapper.findAll('button').find((button) =>
      button.text().includes('evolution.actions.reject'),
    );
    await rejectButton?.trigger('click');

    expect(decideLaputaProposal).toHaveBeenCalledWith(
      'proposal-1',
      expect.objectContaining({ decision: 'deny', grant: 'once', expected_version: 1 }),
    );
  });

  it('defers proposal through the durable backend transition', async () => {
    vi.mocked(listLaputaProposals).mockResolvedValue([
      { ...baseProposal, evidence_refs: [{ id: 'e1', source: 'report', uri: 'file://x', created_at: '2026-06-14T00:00:00Z' }] },
    ]);
    vi.mocked(transitionLaputaProposal).mockResolvedValue({
      ...baseProposal,
      state: 'deferred',
    });
    const wrapper = mountView();
    await flushPromises();

    const deferButton = wrapper.findAll('button').find((button) =>
      button.text().includes('evolution.actions.defer'),
    );
    await deferButton?.trigger('click');
    await flushPromises();

    expect(transitionLaputaProposal).toHaveBeenCalledWith('proposal-1', { state: 'deferred' });
    expect(showAppToast).toHaveBeenCalledWith('evolution.actions.deferSuccess', 'success');
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

  it('filters proposals by status, risk, and search text', async () => {
    vi.mocked(listLaputaProposals).mockResolvedValue([
      {
        ...baseProposal,
        id: 'proposal-1',
        state: 'pending_review',
        risk_level: 'high',
        proposed_patch: 'append alpha memory',
      },
      {
        ...baseProposal,
        id: 'proposal-2',
        state: 'needs_attention',
        risk_level: 'medium',
        target_section: 'identity',
        proposed_patch: 'update beta identity',
      },
    ]);
    const wrapper = mountView();
    await flushPromises();

    await wrapper.find('[data-testid="proposal-status-filter"]').setValue('needs_attention');
    await wrapper.find('[data-testid="proposal-risk-filter"]').setValue('medium');
    await wrapper.find('[data-testid="proposal-search"]').setValue('beta');
    await flushPromises();

    expect(wrapper.find('[data-testid="proposal-row-proposal-1"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="proposal-row-proposal-2"]').exists()).toBe(true);
  });

  it('uses keyboard navigation outside inputs and ignores action shortcuts inside search', async () => {
    vi.mocked(listLaputaProposals).mockResolvedValue([
      { ...baseProposal, id: 'proposal-1', proposed_patch: 'first' },
      {
        ...baseProposal,
        id: 'proposal-2',
        risk_level: 'medium',
        target_section: 'identity',
        proposed_patch: 'second',
      },
    ]);
    const wrapper = mountView();
    await flushPromises();

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'j' }));
    await flushPromises();

    expect(wrapper.find('[data-testid="proposal-row-proposal-2"]').classes()).toContain('active');

    const search = wrapper.find<HTMLInputElement>('[data-testid="proposal-search"]');
    await search.trigger('keydown', { key: 'r' });
    await flushPromises();

    expect(transitionLaputaProposal).not.toHaveBeenCalled();
  });

  it('disables illegal batch approve and prevents backend transition calls', async () => {
    vi.mocked(listLaputaProposals).mockResolvedValue([
      { ...baseProposal, id: 'proposal-1', state: 'applied', risk_level: 'medium' },
    ]);
    const wrapper = mountView();
    await flushPromises();

    const approveButton = wrapper.find('[data-testid="batch-approve"]');
    expect(approveButton.attributes('disabled')).toBeDefined();
    await approveButton.trigger('click');
    await flushPromises();

    expect(transitionLaputaProposal).not.toHaveBeenCalled();
  });

  it('confirms batch reject and reports partial backend failures', async () => {
    vi.mocked(listLaputaProposals).mockResolvedValue([
      { ...baseProposal, id: 'proposal-1', state: 'pending_review', risk_level: 'medium' },
      { ...baseProposal, id: 'proposal-2', state: 'pending_review', risk_level: 'medium' },
    ]);
    vi.mocked(decideLaputaProposal)
      .mockResolvedValueOnce({
        proposal: { ...baseProposal, id: 'proposal-1', state: 'rejected' },
        governance: { ...baseProposal.governance, status: 'denied', request_version: 2 },
      })
      .mockRejectedValueOnce(new Error('reject failed'));
    const wrapper = mountView();
    await flushPromises();

    await wrapper.find('[data-testid="batch-select-visible"]').trigger('click');
    await wrapper.find('[data-testid="batch-reject"]').trigger('click');
    await flushPromises();

    expect(appConfirm).toHaveBeenCalledWith(
      'evolution.confirm.batchReject:{"count":2,"target":"proposal-1 (memory_md), proposal-2 (memory_md)"}',
      { title: 'evolution.confirm.title' },
    );
    expect(decideLaputaProposal).toHaveBeenCalledTimes(2);
    expect(showAppToast).toHaveBeenCalledWith(
      'evolution.actions.batchPartialFailure:{"total":2,"success":1,"failed":1}',
      'error',
      3600,
    );
  });

  it('renders audit changelog records with rollback availability', async () => {
    const wrapper = mountView();
    await flushPromises();

    await wrapper.find('[data-testid="evolution-tab-audit"]').trigger('click');
    await flushPromises();

    expect(listLaputaChangelog).toHaveBeenCalledWith(1, 25);
    expect(wrapper.text()).toContain('tester');
    expect(wrapper.text()).toContain('proposal-1');
    expect(wrapper.text()).toContain('memory_md');
    expect(wrapper.text()).toContain('evolution.audit.rollbackAvailable');
  });

  it('shows rollback unavailable reason for stale audit records', async () => {
    vi.mocked(listLaputaChangelog).mockResolvedValue({
      ...changelogPage,
      items: [{ ...changelogPage.items[0], stale: true }],
    });
    const wrapper = mountView();
    await flushPromises();

    await wrapper.find('[data-testid="evolution-tab-audit"]').trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('evolution.audit.rollbackStale');
  });

  it('renders policy exact copy and settings link', async () => {
    const wrapper = mountView();
    await flushPromises();

    await wrapper.find('[data-testid="evolution-tab-policy"]').trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain(
      'Durable personality, memory, SOP, skill, and policy changes require review before they are applied.',
    );
    expect(wrapper.text()).toContain('evolution.policy.settingsLink');
  });

  it('renders runs unavailable state without spinner-only UI', async () => {
    const wrapper = mountView();
    await flushPromises();

    await wrapper.find('[data-testid="evolution-tab-runs"]').trigger('click');
    await flushPromises();

    expect(listAutoDreamRunRecords).toHaveBeenCalled();
    expect(wrapper.text()).toContain('evolution.runs.unavailableTitle');
    expect(wrapper.text()).toContain('not implemented');
  });

  it('renders runs empty state when backend returns no records', async () => {
    vi.mocked(listAutoDreamRunRecords).mockResolvedValue([]);
    const wrapper = mountView();
    await flushPromises();

    await wrapper.find('[data-testid="evolution-tab-runs"]').trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('evolution.runs.emptyDesc');
  });

  it('opens a proposal from a Chat deep link and filters by source run', async () => {
    vi.mocked(listLaputaProposals).mockResolvedValue([
      { ...baseProposal, id: 'proposal-1', source_run_id: 'run-1' },
      { ...baseProposal, id: 'proposal-2', source_run_id: 'run-2' },
    ]);

    const wrapper = mount(EvolutionView, {
      props: {
        initialTab: 'inbox',
        initialProposalId: 'proposal-1',
        initialSourceRunId: 'run-1',
      },
    });
    await flushPromises();

    expect(wrapper.find('[data-testid="proposal-row-proposal-1"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="proposal-row-proposal-2"]').exists()).toBe(false);
    expect(wrapper.find('[data-testid="proposal-row-proposal-1"]').classes()).toContain('active');
  });

  it('resolves notebook deep-link detail after proposals refresh', async () => {
    vi.mocked(listLaputaProposals)
      .mockResolvedValueOnce([baseProposal])
      .mockResolvedValueOnce([
        { ...baseProposal, id: 'proposal-2', evidence_refs: [{ id: 'e1', source: 'report', uri: 'file://x', created_at: '2026-06-14T00:00:00Z' }] },
        baseProposal,
      ]);

    const wrapper = mountViewWithProps({
      initialTab: 'inbox',
      initialProposalId: 'proposal-2',
      requestKey: 'req-1',
    });
    await flushPromises();

    await wrapper.setProps({
      initialTab: 'inbox',
      initialProposalId: 'proposal-2',
      requestKey: 'req-2',
    });
    await flushPromises();

    expect(wrapper.text()).toContain('proposal-2');
  });
});

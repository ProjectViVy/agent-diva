import { shallowMount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import TrajectoryPanel from './TrajectoryPanel.vue';
import type { TrajectoryView } from '../../state/vivy-trajectory';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      const value = ({
        'trajectory.loading': 'Loading trajectory...',
        'trajectory.hasOlderRuns': 'Older runs exist',
        'trajectory.incomplete': '{count} run(s) incomplete',
        'trajectory.stale': 'Rows may be stale',
        'trajectory.connectionGap': 'Live connection gap',
        'trajectory.usageMissing': '—',
        'trajectory.partialEvidence': 'partial',
        'trajectory.wait.approval': 'waiting approval',
        'trajectory.wait.question': 'waiting question',
        'trajectory.parentRun': 'Parent run',
        'trajectory.childRuns': 'Child runs',
        'trajectory.workflow': 'workflow',
        'trajectory.sessionRecords': 'Session records',
      } as Record<string, string>)[key] ?? key;
      return value.replace(/\{(\w+)\}/g, (_, name: string) => String(params?.[name] ?? `{${name}}`));
    },
  }),
}));

function makeView(over: Partial<TrajectoryView> = {}): TrajectoryView {
  return {
    sessionId: 's1',
    projectionVersion: 2,
    turns: 1,
    records: [
      {
        index: 1,
        id: 'run_a:1:system',
        turn: null,
        group: 'Session',
        kind: 'system',
        text: 'Run start',
      },
      {
        index: 2,
        id: 'rec-7',
        turn: null,
        group: 'Session',
        kind: 'meta',
        text: 'session row',
      },
    ],
    requests: [
      {
        number: 1,
        request_id: 'run_a:call-1',
        run_id: 'run_a',
        call_id: 'call-1',
        turn: 1,
        group: 'Step 1',
        status: 'active',
        call_status: 'active',
        started_at: 1000,
        provider: 'deepseek',
        model: 'deepseek-flash',
        usage: {},
        usage_state: 'reported',
        usage_evidence: {
          prompt_tokens: 10,
          completion_tokens: 20,
          total_tokens: 30,
        },
        messages: 2,
      },
      {
        number: 2,
        request_id: 'run_a:call-2',
        run_id: 'run_a',
        call_id: 'call-2',
        turn: 1,
        group: 'Step 1',
        status: 'complete',
        call_status: 'completed',
        started_at: 2000,
        provider: 'deepseek',
        model: 'deepseek-flash',
        usage: {},
        usage_state: 'missing',
        usage_evidence: null,
        messages: 3,
      },
    ],
    activity: [
      {
        run_id: 'run_a',
        status: 'active',
        activity_state: 'waiting',
        wait_kind: 'approval',
        child_run_ids: ['run_child_1'],
        parent_run_id: 'run_parent_0',
      },
    ],
    hasOlderRuns: true,
    incompleteRuns: [],
    stale: false,
    ...over,
  } as TrajectoryView;
}

describe('TrajectoryPanel', () => {
  it('renders error honestly when no view is available', () => {
    const wrapper = shallowMount(TrajectoryPanel, {
      props: { view: null, error: 'trajectory is not configured', connectionState: 'connected' },
    });
    expect(wrapper.text()).toContain('trajectory is not configured');
  });

  it('renders loading when view is null and no error', () => {
    const wrapper = shallowMount(TrajectoryPanel, {
      props: { view: null, error: null, connectionState: 'connected' },
    });
    expect(wrapper.text()).toContain('Loading trajectory...');
  });

  it('renders run activity with wait kind, refs, requests, and records', () => {
    const wrapper = shallowMount(TrajectoryPanel, {
      props: { view: makeView(), error: null, connectionState: 'connected' },
    });
    const text = wrapper.text();
    expect(text).toContain('waiting');
    expect(text).toContain('waiting approval');
    expect(text).toContain('Parent run');
    expect(text).toContain('Child runs');
    expect(text).toContain('30 tok');
    expect(text).toContain('Run start');
    // session-level record (rec-7) lands in the unowned bucket — not dropped.
    expect(text).toContain('session row');
    expect(text).toContain('Older runs exist');
  });

  it('shows the incomplete banner instead of stale when gap detected', () => {
    const wrapper = shallowMount(TrajectoryPanel, {
      props: {
        view: makeView({ incompleteRuns: ['run_a'], stale: true }),
        error: null,
        connectionState: 'gap',
      },
    });
    expect(wrapper.text()).toContain('1 run(s) incomplete');
  });

  it('shows stale notice when view is stale but complete', () => {
    const wrapper = shallowMount(TrajectoryPanel, {
      props: { view: makeView({ stale: true }), error: null, connectionState: 'connected' },
    });
    expect(wrapper.text()).toContain('Rows may be stale');
  });

  it('missing usage evidence renders a dash, never a zero', () => {
    const wrapper = shallowMount(TrajectoryPanel, {
      props: { view: makeView(), error: null, connectionState: 'connected' },
    });
    const rows = wrapper.findAll('.request-row');
    const missing = rows[rows.length - 1];
    expect(missing.text()).toContain('—');
    expect(missing.text()).not.toContain('0 tok');
  });
});

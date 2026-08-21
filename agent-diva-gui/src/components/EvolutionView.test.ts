import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import EvolutionView from './EvolutionView.vue';
import PersonaMarkdownEditor from './persona-memory/PersonaMarkdownEditor.vue';
import {
  acceptSkillRequest,
  createSkillRequest,
  getAutoDreamLiveText,
  getAutoDreamRunStatus,
  getSkill,
  getSkillRequest,
  getSkills,
  listAutoDreamRunEvents,
  listAutoDreamRunRecords,
  listSkillRequests,
  updateSkill,
} from '../api/desktop';
import type { AutoDreamRunRecord, SkillDocument, SkillDto, SkillRequest } from '../api/desktop';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      const value = ({
        'evolution.autodream.states.pending': 'pending',
        'evolution.autodream.states.running': 'running',
        'evolution.autodream.states.completed': 'completed',
        'evolution.autodream.states.failed': 'failed',
        'evolution.autodream.states.cancelled': 'cancelled',
        'evolution.autodream.phases.queued': 'queued',
        'evolution.autodream.phases.reflecting': 'reflecting',
        'evolution.autodream.phases.completed': 'completed',
      } as Record<string, string>)[key] ?? key;
      return params ? value.replace(/\{(\w+)\}/g, (_, name: string) => String(params[name] ?? `{${name}}`)) : value;
    },
  }),
}));

vi.mock('./persona-memory/PersonaMarkdownEditor.vue', () => ({
  default: {
    name: 'PersonaMarkdownEditor',
    props: ['modelValue', 'readonly'],
    emits: ['update:modelValue'],
    template: '<textarea class="editor-stub" :value="modelValue" :readonly="readonly" @input="$emit(\'update:modelValue\', $event.target.value)" />',
  },
}));

vi.mock('@lucide/vue', () => Object.fromEntries(
  ['FilePlus2', 'GitBranch', 'History', 'RefreshCw', 'Save', 'Search', 'ShieldCheck', 'WandSparkles']
    .map((name) => [name, { name, template: `<span class="${name}" />` }]),
));

vi.mock('../utils/appDialog', () => ({ appConfirm: vi.fn(() => Promise.resolve(true)) }));
vi.mock('../utils/appToast', () => ({ showAppToast: vi.fn() }));
vi.mock('../api/desktop', () => ({
  acceptSkillRequest: vi.fn(),
  createSkillRequest: vi.fn(),
  deleteSkill: vi.fn(),
  disableSkill: vi.fn(),
  getAutoDreamLiveText: vi.fn(),
  getAutoDreamRunStatus: vi.fn(),
  getSkill: vi.fn(),
  getSkillHistoryRevision: vi.fn(),
  getSkillRequest: vi.fn(),
  getSkills: vi.fn(),
  listAutoDreamRunEvents: vi.fn(),
  listAutoDreamRunRecords: vi.fn(),
  listSkillHistory: vi.fn(),
  listSkillRequests: vi.fn(),
  rejectSkillRequest: vi.fn(),
  updateSkill: vi.fn(),
}));

const skill = (slug: string, description = `${slug} description`, evolution_managed = true): SkillDto => ({
  slug,
  name: slug,
  description,
  source: 'home',
  enabled: true,
  always: false,
  available: true,
  active: true,
  content_hash: `hash-${slug}`,
  updated_at: '2026-08-15T00:00:00Z',
  can_hard_delete: true,
  evolution_managed,
  path: '',
  can_delete: true,
});

const document = (slug: string): SkillDocument => ({
  slug,
  description: `${slug} description`,
  source: 'home',
  enabled: true,
  always: false,
  available: true,
  content_hash: `hash-${slug}`,
  updated_at: '2026-08-15T00:00:00Z',
  can_hard_delete: true,
  markdown: `---\nname: ${slug}\ndescription: ${slug} description\n---\nbody`,
});

const request = (id: string, status: SkillRequest['status'] = 'pending'): SkillRequest => ({
  id,
  slug: `${id}-skill`,
  title: `${id} title`,
  proposed_markdown: `---\nname: ${id}-skill\ndescription: request\n---\nbody`,
  evidence: [{ autodream_run_id: 'run-1', actmem_pointer: 'ACTMEM.MD#Work' }],
  attestation: null,
  base_hash: '0',
  source: 'autodream',
  reason: 'bounded reason',
  status,
  created_at: '2026-08-15T00:00:00Z',
  updated_at: '2026-08-15T00:00:00Z',
});

function mountView(props: Record<string, unknown> = {}) {
  return mount(EvolutionView, {
    props,
    global: {
      stubs: {
        PersonaMarkdownEditor: {
          name: 'PersonaMarkdownEditor',
          props: ['modelValue', 'readonly'],
          emits: ['update:modelValue'],
          template: '<textarea class="editor-stub" :value="modelValue" :readonly="readonly" @input="$emit(\'update:modelValue\', $event.target.value)" />',
        },
      },
    },
  });
}

describe('EvolutionView Skill authority', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(listAutoDreamRunRecords).mockResolvedValue([]);
    vi.mocked(listAutoDreamRunEvents).mockResolvedValue([]);
    vi.mocked(getAutoDreamLiveText).mockResolvedValue('');
    vi.mocked(getSkills).mockResolvedValue([skill('alpha'), skill('beta')]);
    vi.mocked(listSkillRequests).mockResolvedValue([request('request-1')]);
    vi.mocked(getSkill).mockImplementation(async (slug) => document(slug));
    vi.mocked(getSkillRequest).mockImplementation(async (id) => request(id));
    vi.mocked(createSkillRequest).mockImplementation(async (payload) => ({
      ...request('created'),
      slug: payload.slug,
      title: payload.title,
      proposed_markdown: payload.proposed_markdown,
      reason: payload.reason,
      attestation: payload.attestation,
    }));
  });

  it('loads Skill and request lists and emits pending-only badge count', async () => {
    const wrapper = mountView();
    await flushPromises();

    expect(wrapper.text()).toContain('request-1 title');
    const emitted = wrapper.emitted('count-change') ?? [];
    expect(emitted.at(-1)?.[0]).toMatchObject({ total: 1, tone: 'warning' });

    await wrapper.findAll('.tabbar button')[0].trigger('click');
    expect(wrapper.text()).toContain('alpha description');
    expect(wrapper.text()).toContain('beta description');
  });

  it('hides installed skills that are not evolution managed', async () => {
    const installed = skill('installed-one', 'installed one description', false);
    const legacyInstalled = { ...skill('legacy-one', 'legacy one description') };
    delete (legacyInstalled as Record<string, unknown>).evolution_managed;
    vi.mocked(getSkills).mockResolvedValue([
      skill('alpha'),
      installed,
      legacyInstalled as SkillDto,
    ]);

    const wrapper = mountView();
    await flushPromises();
    await wrapper.findAll('.tabbar button')[0].trigger('click');

    expect(wrapper.text()).toContain('alpha description');
    expect(wrapper.text()).not.toContain('installed one description');
    expect(wrapper.text()).not.toContain('legacy one description');
  });

  it('keeps detail responses bound to the currently selected slug', async () => {
    let resolveAlpha!: (value: SkillDocument) => void;
    let resolveBeta!: (value: SkillDocument) => void;
    vi.mocked(getSkill).mockImplementation((slug) => new Promise((resolve) => {
      if (slug === 'alpha') resolveAlpha = resolve;
      else resolveBeta = resolve;
    }));
    const wrapper = mountView({ initialTab: 'skills' });
    await flushPromises();
    const rows = wrapper.findAll('.list-row');
    await rows[0].trigger('click');
    await rows[1].trigger('click');
    resolveBeta(document('beta'));
    await flushPromises();
    resolveAlpha(document('alpha'));
    await flushPromises();

    expect(wrapper.find('.detail-heading h2').text()).toBe('beta');
    expect(wrapper.findComponent(PersonaMarkdownEditor).props('modelValue')).toContain('name: beta');
  });

  it('retains the editor draft and selection when CAS save fails', async () => {
    vi.mocked(updateSkill).mockRejectedValue({ code: 'skill_hash_conflict', message: 'hash conflict' });
    const wrapper = mountView({ initialTab: 'skills' });
    await flushPromises();
    await wrapper.findAll('.list-row')[0].trigger('click');
    await flushPromises();
    await wrapper.findAll('.detail-actions button')[0].trigger('click');
    const editor = wrapper.findComponent(PersonaMarkdownEditor);
    editor.vm.$emit('update:modelValue', 'draft survives conflict');
    await flushPromises();
    const save = wrapper.findAll('.detail-actions button').find((button) => button.text().includes('保存'))!;
    await save.trigger('click');
    await flushPromises();

    expect(wrapper.find('.detail-heading h2').text()).toBe('alpha');
    expect(wrapper.findComponent(PersonaMarkdownEditor).props('modelValue')).toBe('draft survives conflict');
    expect(wrapper.text()).toContain('hash conflict');
  });

  it('renders stale requests read-only and prevents acceptance', async () => {
    vi.mocked(listSkillRequests).mockResolvedValue([request('stale-request', 'stale')]);
    vi.mocked(getSkillRequest).mockResolvedValue(request('stale-request', 'stale'));
    const wrapper = mountView({ initialProposalId: 'stale-request' });
    await flushPromises();

    expect(wrapper.text()).toContain('不能接受');
    const accept = wrapper.findAll('.detail-actions button').find((button) => button.text() === '接受')!;
    expect(accept.attributes('disabled')).toBeDefined();
    expect(vi.mocked(acceptSkillRequest)).not.toHaveBeenCalled();
  });

  it('creates a user request with attestation and the current base hash', async () => {
    const wrapper = mountView();
    await flushPromises();
    await wrapper.find('.toolbar .primary').trigger('click');
    const inputs = wrapper.findAll('.form-grid input');
    await inputs[0].setValue('alpha');
    await inputs[1].setValue('Improve alpha');
    await inputs[2].setValue('Reusable behavior');
    await inputs[3].setValue('I reviewed this content');
    await wrapper.find('.create-panel .primary').trigger('click');
    await flushPromises();

    expect(createSkillRequest).toHaveBeenCalledWith(expect.objectContaining({
      slug: 'alpha',
      base_hash: 'hash-alpha',
      attestation: 'I reviewed this content',
    }));
    expect(wrapper.text()).toContain('Improve alpha');
  });

  it('loads persisted AutoDream runs and opens the linked run detail', async () => {
    const run = {
      id: 'run-persisted',
      started_at: '2026-08-20T00:00:00Z',
      completed_at: '2026-08-20T00:00:05Z',
      state: 'completed' as const,
      trigger: 'manual',
      summary: 'AutoDream completed',
      proposal_ids: ['request-1'],
      orchestration: {
        schema_version: 1,
        phase: 'completed' as const,
        attempt: 1,
        deadline_at: '2026-08-20T00:01:00Z',
        updated_at: '2026-08-20T00:00:05Z',
      },
    } satisfies AutoDreamRunRecord;
    vi.mocked(listAutoDreamRunRecords).mockResolvedValue([run]);
    vi.mocked(getAutoDreamRunStatus).mockResolvedValue(run);
    vi.mocked(listAutoDreamRunEvents).mockResolvedValue([{
      id: 'event-1',
      run_id: run.id,
      kind: 'worker_succeeded',
      message: 'AutoDream completed',
      created_at: '2026-08-20T00:00:05Z',
    }]);
    vi.mocked(getAutoDreamLiveText).mockResolvedValue('{"schema_version":1}');

    const wrapper = mountView({
      initialTab: 'autodream',
      initialSourceRunId: run.id,
    });
    await flushPromises();

    expect(listAutoDreamRunRecords).toHaveBeenCalled();
    expect(wrapper.text()).toContain('run-persisted');
    expect(wrapper.text()).toContain('{"schema_version":1}');
    expect(wrapper.text()).toContain('AutoDream completed');
    expect(wrapper.find('.autodream-events-panel').text()).toContain('worker_succeeded');
    expect(wrapper.find('.autodream-open-requests').exists()).toBe(true);
  });

  it('keeps the persisted AutoDream list available when the status detail fails', async () => {
    const run = {
      id: 'run-detail-error',
      started_at: '2026-08-20T00:00:00Z',
      state: 'running' as const,
      trigger: 'manual',
      proposal_ids: [],
    } satisfies AutoDreamRunRecord;
    vi.mocked(listAutoDreamRunRecords).mockResolvedValue([run]);
    vi.mocked(getAutoDreamRunStatus).mockRejectedValue(new Error('status unavailable'));

    const wrapper = mountView({ initialTab: 'autodream' });
    await flushPromises();

    expect(wrapper.text()).toContain('run-detail-error');
    expect(wrapper.text()).toContain('status unavailable');
    expect(wrapper.find('.list-row').exists()).toBe(true);
  });

  it('polls an active AutoDream run and stops polling after it reaches a terminal state', async () => {
    vi.useFakeTimers();
    try {
      const activeRun = {
        id: 'run-active',
        started_at: '2026-08-20T00:00:00Z',
        state: 'running' as const,
        trigger: 'manual',
        proposal_ids: [],
        orchestration: {
          schema_version: 1,
          phase: 'reflecting' as const,
          attempt: 1,
          deadline_at: '2026-08-20T00:01:00Z',
          updated_at: '2026-08-20T00:00:01Z',
        },
      } satisfies AutoDreamRunRecord;
      const completedRun = { ...activeRun, state: 'completed' as const, completed_at: '2026-08-20T00:00:02Z' };
      vi.mocked(listAutoDreamRunRecords).mockResolvedValue([activeRun]);
      vi.mocked(getAutoDreamRunStatus)
        .mockResolvedValueOnce(activeRun)
        .mockResolvedValueOnce(completedRun);

      const wrapper = mountView({ initialTab: 'autodream' });
      await flushPromises();
      expect(getAutoDreamRunStatus).toHaveBeenCalledTimes(1);

      await vi.advanceTimersByTimeAsync(1000);
      await flushPromises();
      expect(getAutoDreamRunStatus).toHaveBeenCalledTimes(2);
      expect(wrapper.text()).toContain('completed');

      await vi.advanceTimersByTimeAsync(2000);
      await flushPromises();
      expect(getAutoDreamRunStatus).toHaveBeenCalledTimes(2);
      wrapper.unmount();
    } finally {
      vi.useRealTimers();
    }
  });
});

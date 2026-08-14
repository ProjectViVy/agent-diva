import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import EvolutionView from './EvolutionView.vue';
import PersonaMarkdownEditor from './persona-memory/PersonaMarkdownEditor.vue';
import {
  acceptSkillRequest,
  createSkillRequest,
  getSkill,
  getSkillRequest,
  getSkills,
  listSkillRequests,
  updateSkill,
} from '../api/desktop';
import type { SkillDocument, SkillDto, SkillRequest } from '../api/desktop';

vi.mock('@lucide/vue', () => Object.fromEntries(
  ['FilePlus2', 'History', 'RefreshCw', 'Save', 'Search', 'ShieldCheck', 'WandSparkles']
    .map((name) => [name, { name, template: `<span class="${name}" />` }]),
));

vi.mock('../utils/appDialog', () => ({ appConfirm: vi.fn(() => Promise.resolve(true)) }));
vi.mock('../utils/appToast', () => ({ showAppToast: vi.fn() }));
vi.mock('../api/desktop', () => ({
  acceptSkillRequest: vi.fn(),
  createSkillRequest: vi.fn(),
  deleteSkill: vi.fn(),
  disableSkill: vi.fn(),
  getSkill: vi.fn(),
  getSkillHistoryRevision: vi.fn(),
  getSkillRequest: vi.fn(),
  getSkills: vi.fn(),
  listSkillHistory: vi.fn(),
  listSkillRequests: vi.fn(),
  rejectSkillRequest: vi.fn(),
  updateSkill: vi.fn(),
}));

const skill = (slug: string, description = `${slug} description`): SkillDto => ({
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
});

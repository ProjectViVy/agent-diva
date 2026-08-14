import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import PersonaMemoryView from './PersonaMemoryView.vue';
import * as desktop from '../api/desktop';
import en from '../locales/en';

vi.mock('../api/desktop', async () => ({
  ...await vi.importActual<typeof desktop>('../api/desktop'),
  getPersonaDocument: vi.fn(), listPersonaRequests: vi.fn(), listPersonaHistory: vi.fn(),
  getPersonaHistoryRevision: vi.fn(), savePersonaDocument: vi.fn(),
  acceptPersonaRequest: vi.fn(), rejectPersonaRequest: vi.fn(), isTauriRuntime: vi.fn(() => true),
}));
vi.mock('../utils/appDialog', () => ({ appConfirm: vi.fn(() => Promise.resolve(true)) }));
vi.mock('../utils/appToast', () => ({ showAppToast: vi.fn() }));
vi.mock('./persona-memory/PersonaMarkdownEditor.vue', () => ({
  default: { props: ['modelValue'], emits: ['update:modelValue'], template: '<textarea :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />' },
}));

const document = (kind: desktop.PersonaKind, content = `# ${kind}`): desktop.PersonaDocument => ({
  kind, file_name: `${kind.toUpperCase()}.MD`, exists: true, valid: true, content,
  revision: 1, content_hash: 'sha256:one', updated_at: '2026-08-14T00:00:00Z', pending_count: 0,
});
const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } });

describe('PersonaMemoryView Markdown workspace', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(desktop.getPersonaDocument).mockImplementation(async (kind) => document(kind));
    vi.mocked(desktop.listPersonaRequests).mockResolvedValue([]);
    vi.mocked(desktop.listPersonaHistory).mockResolvedValue([]);
  });

  it('loads the seven-file authority and switches documents', async () => {
    const wrapper = mount(PersonaMemoryView, { global: { plugins: [i18n] } });
    await flushPromises();
    expect(wrapper.findAll('nav button')).toHaveLength(7);
    expect(wrapper.find('textarea').element.value).toBe('# identity');
    await wrapper.findAll('nav button')[4].trigger('click');
    await flushPromises();
    expect(desktop.getPersonaDocument).toHaveBeenLastCalledWith('world');
    expect(wrapper.find('textarea').element.value).toBe('# world');
  });

  it('saves current Markdown with its CAS revision', async () => {
    vi.mocked(desktop.savePersonaDocument).mockResolvedValue({ document: document('identity', '# changed'), changed: true });
    const wrapper = mount(PersonaMemoryView, { global: { plugins: [i18n] } });
    await flushPromises();
    await wrapper.find('textarea').setValue('# changed');
    await wrapper.find('.actions .save').trigger('click');
    await flushPromises();
    expect(desktop.savePersonaDocument).toHaveBeenCalledWith('identity', '# changed', 1, 'GUI direct save');
  });

  it('renders pending Persona requests and accepts through the dedicated API', async () => {
    vi.mocked(desktop.listPersonaRequests).mockResolvedValue([{
      id: 'request-1', kind: 'identity', base_revision: 1, base_hash: 'sha256:one',
      proposed_markdown: '# proposed', actor: 'agent', reason: 'observed change',
      created_at: '2026-08-14T00:00:00Z', state: 'pending', decided_at: null,
    }]);
    const wrapper = mount(PersonaMemoryView, { global: { plugins: [i18n] } });
    await flushPromises();
    await wrapper.findAll('.tabs button')[1].trigger('click');
    expect(wrapper.find('.request-card').text()).toContain('observed change');
    await wrapper.find('.accept').trigger('click');
    await flushPromises();
    expect(desktop.acceptPersonaRequest).toHaveBeenCalledWith('request-1');
  });

  it('loads immutable history diff and restores it into the editor', async () => {
    vi.mocked(desktop.listPersonaHistory).mockResolvedValue([{
      revision: 1, content_hash: 'sha256:one', snapshot: '1.md', diff: '1.diff', actor: 'user',
      source: 'user_direct', reason: 'initial', base_revision: 0, created_at: '2026-08-14T00:00:00Z',
    }]);
    vi.mocked(desktop.getPersonaHistoryRevision).mockResolvedValue({
      revision: 1, content_hash: 'sha256:one', snapshot: '1.md', diff: '1.diff', actor: 'user',
      source: 'user_direct', reason: 'initial', base_revision: 0, created_at: '2026-08-14T00:00:00Z',
      content: '# old', unified_diff: '-before\n+old',
    });
    const wrapper = mount(PersonaMemoryView, { global: { plugins: [i18n] } });
    await flushPromises();
    await wrapper.findAll('.tabs button')[2].trigger('click');
    await wrapper.find('.history-pane aside button').trigger('click');
    await flushPromises();
    expect(wrapper.find('.revision').text()).toContain('-before');
    await wrapper.find('.revision header button').trigger('click');
    expect(wrapper.find('textarea').element.value).toBe('# old');
  });
});

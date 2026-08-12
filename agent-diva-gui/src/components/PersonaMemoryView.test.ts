import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import PersonaMemoryView from './PersonaMemoryView.vue';
import * as desktop from '../api/desktop';
import en from '../locales/en';

vi.mock('../api/desktop', async () => ({
  ...await vi.importActual<typeof desktop>('../api/desktop'),
  getLaputaPersonaWorkspace: vi.fn(),
  writeLaputaSection: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));
vi.mock('../utils/appDialog', () => ({ appConfirm: vi.fn(() => Promise.resolve(true)) }));
vi.mock('../utils/appToast', () => ({ showAppToast: vi.fn() }));

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } });
const section = (name: desktop.LaputaSectionName, value: string): desktop.LaputaSection => ({
  name, status: 'owned', content: { value }, metadata: {}, last_modified: '2026-08-09T08:00:00Z', version: '1',
});
const projection: desktop.PersonaWorkspaceProjection = {
  snapshot: {
    schema_version: '1',
    sections: { identity: section('identity', 'Diva'), relationship: section('relationship', 'partner') },
    changed_sections: [], server_time: '2026-08-09T08:00:00Z',
  },
  authority_versions: { identity: 'authority-v2', relationship: 'relationship-v1' },
  session: { session_key: 'desktop:test', captured_at: '2026-08-09T08:00:00Z', section_versions: { identity: 'authority-v1' } },
  proposals: [], changelog: [], cognitive: { memrules: 'rules', world: 'world' },
};

describe('PersonaMemoryView lifecycle workspace', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (desktop.getLaputaPersonaWorkspace as ReturnType<typeof vi.fn>).mockResolvedValue(projection);
  });

  it('loads the aggregate projection for the active session', async () => {
    const wrapper = mount(PersonaMemoryView, { props: { sessionKey: 'desktop:test' }, global: { plugins: [i18n] } });
    await flushPromises();
    expect(desktop.getLaputaPersonaWorkspace).toHaveBeenCalledWith('desktop:test');
    expect(wrapper.text()).toContain(en.laputa.workspace.nextSessionEffective);
    expect(wrapper.find('textarea').element.value).toContain('Diva');
  });

  it('shows cognitive governance files as read-only', async () => {
    const wrapper = mount(PersonaMemoryView, { global: { plugins: [i18n] } });
    await flushPromises();
    const world = wrapper.findAll('.section-item').find((item) => item.text().includes('World'))!;
    await world.trigger('click');
    expect(wrapper.find('.cognitive-panel').text()).toContain('world');
    expect(wrapper.find('textarea').exists()).toBe(false);
  });

  it('renders a real pending proposal in the lifecycle rail', async () => {
    (desktop.getLaputaPersonaWorkspace as ReturnType<typeof vi.fn>).mockResolvedValue({
      ...projection,
      proposals: [{
        id: 'proposal-1', target_section: 'identity', state: 'pending_review', updated_at: '2026-08-09T09:00:00Z',
        governance: { request_id: 'request-1', request_version: 1 },
      }],
    });
    const wrapper = mount(PersonaMemoryView, { global: { plugins: [i18n] } });
    await flushPromises();
    expect(wrapper.find('.lifecycle-rail').text()).toContain('pending_review');
    expect(wrapper.find('.proposal-pending-note').text()).toContain('not active yet');
  });

  it('renders null authority as an editable empty JSON object', async () => {
    (desktop.getLaputaPersonaWorkspace as ReturnType<typeof vi.fn>).mockResolvedValue({
      ...projection,
      snapshot: {
        ...projection.snapshot,
        sections: {
          ...projection.snapshot.sections,
          identity: { ...section('identity', ''), status: 'tbd', content: null },
        },
      },
    });
    const wrapper = mount(PersonaMemoryView, { global: { plugins: [i18n] } });
    await flushPromises();
    expect(wrapper.find('textarea').element.value).toBe('{}');
  });

  it('shows a structured error message and preserves the last successful workspace', async () => {
    (desktop.getLaputaPersonaWorkspace as ReturnType<typeof vi.fn>)
      .mockResolvedValueOnce(projection)
      .mockRejectedValueOnce({ message: 'governance ledger unavailable', status: 503 });
    const wrapper = mount(PersonaMemoryView, { global: { plugins: [i18n] } });
    await flushPromises();

    await wrapper.find('.workspace-header button').trigger('click');
    await flushPromises();

    expect(wrapper.find('.workspace-error').text()).toContain('governance ledger unavailable');
    expect(wrapper.find('.workspace-error').text()).not.toContain('[object Object]');
    expect(wrapper.find('textarea').element.value).toContain('Diva');
  });
});

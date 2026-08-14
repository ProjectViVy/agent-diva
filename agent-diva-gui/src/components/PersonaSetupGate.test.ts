import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import PersonaSetupGate from './PersonaSetupGate.vue';
import * as desktop from '../api/desktop';
import en from '../locales/en';

vi.mock('../api/desktop', async () => ({
  ...await vi.importActual<typeof desktop>('../api/desktop'),
  getPersonaStatus: vi.fn(), initializePersona: vi.fn(), repairPersona: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

const status = (value: desktop.PersonaStatus): desktop.PersonaStatusView => ({
  status: value,
  files: Object.fromEntries((['identity', 'relationship', 'redline', 'user', 'world', 'dream', 'dark'] as desktop.PersonaKind[])
    .map((kind) => [kind, { kind, file_name: `${kind}.md`, exists: value === 'ready', valid: value === 'ready' || ['dream', 'dark'].includes(kind), reason: null, revision: value === 'ready' ? 1 : 0, updated_at: null, pending_count: 0 }])) as Record<desktop.PersonaKind, desktop.PersonaFileState>,
});
const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } });

describe('PersonaSetupGate', () => {
  beforeEach(() => { vi.useFakeTimers(); vi.clearAllMocks(); });

  it('stays absent when Persona is independently ready', async () => {
    vi.mocked(desktop.getPersonaStatus).mockResolvedValue(status('ready'));
    const wrapper = mount(PersonaSetupGate, { global: { plugins: [i18n] } });
    await vi.runAllTimersAsync(); await flushPromises();
    expect(wrapper.find('.persona-setup-backdrop').exists()).toBe(false);
    expect(wrapper.emitted('ready')).toHaveLength(1);
  });

  it('collects five raw Markdown authorities on first initialization', async () => {
    vi.mocked(desktop.getPersonaStatus).mockResolvedValue(status('uninitialized'));
    vi.mocked(desktop.initializePersona).mockResolvedValue(status('ready'));
    const wrapper = mount(PersonaSetupGate, { global: { plugins: [i18n] } });
    await vi.runAllTimersAsync(); await flushPromises();
    const fields = wrapper.findAll('textarea');
    expect(fields).toHaveLength(5);
    for (const [index, field] of fields.entries()) await field.setValue(`value-${index}`);
    await wrapper.find('.primary').trigger('click'); await flushPromises();
    expect(desktop.initializePersona).toHaveBeenCalledWith({
      identity: 'value-0', relationship: 'value-1', redline: 'value-2', user: 'value-3', world: 'value-4',
    });
  });
});

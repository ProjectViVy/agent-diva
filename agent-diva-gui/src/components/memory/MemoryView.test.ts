import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import MemoryView from './MemoryView.vue';
import PersonaMarkdownEditor from '../persona-memory/PersonaMarkdownEditor.vue';
import * as desktop from '../../api/desktop';
import en from '../../locales/en';

vi.mock('../../api/desktop', async () => {
  const actual = await vi.importActual<typeof desktop>('../../api/desktop');
  return {
    ...actual,
    listMemoryRecords: vi.fn(), createMemoryRecord: vi.fn(), getMemoryRecord: vi.fn(),
    updateMemoryRecord: vi.fn(), deleteMemoryRecord: vi.fn(), getActmem: vi.fn(),
    putActmem: vi.fn(), listActmemCapsules: vi.fn(), getActmemCapsule: vi.fn(),
    deleteActmemCapsule: vi.fn(), getMemoryRules: vi.fn(), putMemoryRules: vi.fn(),
  };
});

const appPrompt = vi.fn(() => Promise.resolve<string | null>('obsolete'));
const appConfirm = vi.fn(() => Promise.resolve(true));
vi.mock('../../utils/appDialog', () => ({
  appPrompt: (...args: unknown[]) => appPrompt(...args),
  appConfirm: (...args: unknown[]) => appConfirm(...args),
}));
vi.mock('../../utils/appToast', () => ({ showAppToast: vi.fn() }));

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } });
const record: desktop.MemoryRecord = {
  id: 'memory-1', content: 'kestrel patrols at dawn', trust: 'applied_authority',
  provenance: 'user_input', evidence_refs: [], revision: 4, created_at: '2026-08-15T00:00:00Z',
  updated_at: '2026-08-15T00:01:00Z',
};
const actmem: desktop.ActmemDocument = {
  revision: 7, updated_at: '2026-08-15T00:02:00Z', pulse: '- user pulse',
  recap: '- assistant recap', work: '### Goal\n- ship',
  markdown: '# ACTMEM',
};

function factory() {
  return mount(MemoryView, { global: { plugins: [i18n] } });
}

describe('MemoryView S3 workspace', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    appPrompt.mockResolvedValue('obsolete');
    appConfirm.mockResolvedValue(true);
    vi.mocked(desktop.listMemoryRecords).mockResolvedValue([record]);
    vi.mocked(desktop.getMemoryRecord).mockResolvedValue(record);
    vi.mocked(desktop.createMemoryRecord).mockResolvedValue({ ...record, id: 'memory-new', revision: 1 });
    vi.mocked(desktop.updateMemoryRecord).mockResolvedValue({ ...record, content: 'updated', revision: 5 });
    vi.mocked(desktop.deleteMemoryRecord).mockResolvedValue({ record, deleted: true });
    vi.mocked(desktop.getActmem).mockResolvedValue(actmem);
    vi.mocked(desktop.putActmem).mockResolvedValue({ ...actmem, pulse: '- changed', revision: 8 });
    vi.mocked(desktop.listActmemCapsules).mockResolvedValue([{ name: 'capsule.md', session_key: 'gui:chat', created_at: '2026-08-15T00:00:00Z', chars: 80 }]);
    vi.mocked(desktop.getActmemCapsule).mockResolvedValue({ name: 'capsule.md', session_key: 'gui:chat', created_at: '2026-08-15T00:00:00Z', markdown: '# Capsule' });
    vi.mocked(desktop.deleteActmemCapsule).mockResolvedValue({ deleted: true });
    vi.mocked(desktop.getMemoryRules).mockResolvedValue({ content: '# R1', source: 'default' });
    vi.mocked(desktop.putMemoryRules).mockResolvedValue({ content: '# Custom', source: 'file' });
  });

  it('uses a LongTerm list and detail without kind filters', async () => {
    const wrapper = factory();
    await flushPromises();
    expect(wrapper.text()).toContain('kestrel patrols at dawn');
    expect(wrapper.find('select').exists()).toBe(false);
    await wrapper.find('.record-row').trigger('click');
    await flushPromises();
    expect(desktop.getMemoryRecord).toHaveBeenCalledWith('memory-1');
    expect(wrapper.text()).toContain('user_input');
  });

  it('creates and updates with one explicit save boundary', async () => {
    const wrapper = factory();
    await flushPromises();
    await wrapper.find('.button.primary').trigger('click');
    await wrapper.find('textarea').setValue('new durable fact');
    await wrapper.findAll('.action-row .button.primary').at(0)!.trigger('click');
    await flushPromises();
    expect(desktop.createMemoryRecord).toHaveBeenCalledWith('new durable fact');

    await wrapper.findAll('.record-row').at(0)!.trigger('click');
    await flushPromises();
    await wrapper.find('.record-detail .button.secondary').trigger('click');
    await wrapper.find('textarea').setValue('updated');
    await wrapper.find('.editor-pane .button.primary').trigger('click');
    await flushPromises();
    expect(desktop.updateMemoryRecord).toHaveBeenCalledWith('memory-1', 'updated', 4);
  });

  it('soft deletes directly with record revision and emits no approval event', async () => {
    const wrapper = factory();
    await flushPromises();
    await wrapper.find('.record-row').trigger('click');
    await flushPromises();
    await wrapper.find('.button.danger').trigger('click');
    await flushPromises();
    expect(desktop.deleteMemoryRecord).toHaveBeenCalledWith('memory-1', 'obsolete', 4);
    expect(wrapper.emitted('open-approval')).toBeUndefined();
  });

  it('edits one ACTMEM section with the displayed base revision', async () => {
    const wrapper = factory();
    await flushPromises();
    await wrapper.findAll('.workspace-tabs button').at(1)!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('assistant recap');
    await wrapper.find('.section-card .button.secondary').trigger('click');
    wrapper.findComponent(PersonaMarkdownEditor).vm.$emit('update:modelValue', '- changed');
    await flushPromises();
    await wrapper.find('.editor-dialog .button.primary').trigger('click');
    await flushPromises();
    expect(desktop.putActmem).toHaveBeenCalledWith({ pulse: '- changed', base_revision: 7 });
  });

  it('reads and deletes a projected capsule with one confirmation', async () => {
    const wrapper = factory();
    await flushPromises();
    await wrapper.findAll('.workspace-tabs button').at(1)!.trigger('click');
    await flushPromises();
    await wrapper.find('.capsule-panel .record-row').trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('# Capsule');
    await wrapper.find('.capsule-detail .button.danger').trigger('click');
    await flushPromises();
    expect(appConfirm).toHaveBeenCalledTimes(1);
    expect(desktop.deleteActmemCapsule).toHaveBeenCalledWith('capsule.md');
  });

  it('shows MEMRULES source and saves only after explicit edit', async () => {
    const wrapper = factory();
    await flushPromises();
    await wrapper.findAll('.workspace-tabs button').at(2)!.trigger('click');
    await flushPromises();
    expect(wrapper.text()).toContain('Built-in default');
    await wrapper.find('.rules-panel .button.secondary').trigger('click');
    wrapper.findComponent(PersonaMarkdownEditor).vm.$emit('update:modelValue', '# Custom');
    await flushPromises();
    await wrapper.find('.rules-editor .button.primary').trigger('click');
    await flushPromises();
    expect(desktop.putMemoryRules).toHaveBeenCalledWith('# Custom');
  });
});

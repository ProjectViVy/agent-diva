import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import SectionEditor from '../SectionEditor.vue';
import en from '../../../locales/en';
import * as desktop from '../../../api/desktop';
import * as appDialog from '../../../utils/appDialog';

vi.mock('../../../api/desktop', () => ({ writeLaputaSection: vi.fn() }));
vi.mock('../../../utils/appDialog', () => ({ appConfirm: vi.fn() }));

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } });
const proposal = { proposal_id: 'proposal-1', proposal_type: 'identity_patch', risk_level: 'high', state: 'pending_review' };

function factory(initialContent = '{"name":"Diva"}') {
  return mount(SectionEditor, {
    props: { sectionName: 'identity', displayName: 'Identity', modelValue: initialContent, initialContent },
    global: { plugins: [i18n] },
  });
}

async function change(wrapper: ReturnType<typeof factory>, content = '{"name":"Laputa"}', reason = 'Refine identity') {
  await wrapper.find('textarea').setValue(content);
  await wrapper.find('.section-editor-reason input').setValue(reason);
  await flushPromises();
}

describe('SectionEditor JSON governance contract', () => {
  beforeEach(() => vi.clearAllMocks());

  it('requires valid JSON and a change reason', async () => {
    const wrapper = factory();
    await wrapper.find('textarea').setValue('not json');
    await wrapper.find('.section-editor-reason input').setValue('reason');
    expect(wrapper.find('.section-editor-json-error').exists()).toBe(true);
    expect(wrapper.find('.section-editor-save-btn').attributes('disabled')).toBeDefined();
  });

  it('formats valid JSON without changing its meaning', async () => {
    const wrapper = factory();
    await wrapper.find('textarea').setValue('{"name":"Laputa","traits":["calm"]}');
    await wrapper.findAll('.section-editor-history-btn')[0].trigger('click');
    expect(wrapper.find('textarea').element.value).toContain('\n  "name"');
  });

  it('submits JSON plus the explicit governance reason', async () => {
    (appDialog.appConfirm as ReturnType<typeof vi.fn>).mockResolvedValue(true);
    (desktop.writeLaputaSection as ReturnType<typeof vi.fn>).mockResolvedValue(proposal);
    const wrapper = factory();
    await change(wrapper);
    await wrapper.find('.section-editor-save-btn').trigger('click');
    await flushPromises();
    expect(desktop.writeLaputaSection).toHaveBeenCalledWith('identity', '{"name":"Laputa"}', 'Refine identity');
    expect(wrapper.emitted('proposal-created')).toEqual([['identity', proposal]]);
  });

  it('preserves the draft when proposal creation fails', async () => {
    (appDialog.appConfirm as ReturnType<typeof vi.fn>).mockResolvedValue(true);
    (desktop.writeLaputaSection as ReturnType<typeof vi.fn>).mockRejectedValue(new Error('disk full'));
    const wrapper = factory();
    await change(wrapper);
    await wrapper.find('.section-editor-save-btn').trigger('click');
    await flushPromises();
    expect(wrapper.find('textarea').element.value).toBe('{"name":"Laputa"}');
    expect(wrapper.text()).toContain('disk full');
  });

  it('does not submit when confirmation is cancelled', async () => {
    (appDialog.appConfirm as ReturnType<typeof vi.fn>).mockResolvedValue(false);
    const wrapper = factory();
    await change(wrapper);
    await wrapper.find('.section-editor-save-btn').trigger('click');
    await flushPromises();
    expect(desktop.writeLaputaSection).not.toHaveBeenCalled();
  });
});

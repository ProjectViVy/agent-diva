import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import SectionEditor from '../SectionEditor.vue';

function factory(props: Record<string, unknown> = {}, slots: Record<string, string> = {}) {
  return mount(SectionEditor, {
    props: {
      sectionName: 'identity',
      modelValue: '',
      ...props,
    },
    slots,
  });
}

describe('SectionEditor', () => {
  it('renders a textarea with the correct aria-label', () => {
    const wrapper = factory();
    const textarea = wrapper.find('textarea');
    expect(textarea.exists()).toBe(true);
    expect(textarea.attributes('aria-label')).toBe('Laputa section editor');
  });

  it('initializes the textarea from modelValue', () => {
    const wrapper = factory({ modelValue: '# Identity\n\nHello' });
    expect(wrapper.find('textarea').element.value).toBe('# Identity\n\nHello');
  });

  it('emits update:modelValue on textarea input', async () => {
    const wrapper = factory();
    const textarea = wrapper.find('textarea');
    await textarea.setValue('# Updated');
    expect(wrapper.emitted('update:modelValue')).toHaveLength(1);
    expect(wrapper.emitted('update:modelValue')![0]).toEqual(['# Updated']);
  });

  it('renders the Markdown preview from modelValue', () => {
    const wrapper = factory({ modelValue: '# Identity\n\n- Trait one\n- Trait two' });
    const preview = wrapper.find('.section-editor-preview');
    expect(preview.exists()).toBe(true);
    expect(preview.html()).toContain('<h1>Identity</h1>');
    expect(preview.html()).toContain('<li>Trait one</li>');
  });

  it('emits save on Ctrl+S keydown', async () => {
    const wrapper = factory();
    const textarea = wrapper.find('textarea');
    await textarea.trigger('keydown', { key: 's', ctrlKey: true });
    expect(wrapper.emitted('save')).toHaveLength(1);
  });

  it('emits save on Meta+S keydown', async () => {
    const wrapper = factory();
    const textarea = wrapper.find('textarea');
    await textarea.trigger('keydown', { key: 's', metaKey: true });
    expect(wrapper.emitted('save')).toHaveLength(1);
  });

  it('renders the toolbar-actions slot content', () => {
    const wrapper = factory({}, { 'toolbar-actions': '<button type="button">Save</button>' });
    expect(wrapper.find('button').exists()).toBe(true);
    expect(wrapper.text()).toContain('Save');
  });

  it('shows the section name in the toolbar', () => {
    const wrapper = factory({ sectionName: 'memory_md' });
    expect(wrapper.text()).toContain('memory_md');
  });

  it('updates the textarea when modelValue prop changes', async () => {
    const wrapper = factory({ modelValue: 'initial' });
    expect(wrapper.find('textarea').element.value).toBe('initial');
    await wrapper.setProps({ modelValue: 'updated' });
    expect(wrapper.find('textarea').element.value).toBe('updated');
  });
});

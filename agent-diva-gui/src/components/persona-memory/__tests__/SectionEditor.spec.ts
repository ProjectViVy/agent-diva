import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import SectionEditor from '../SectionEditor.vue';
import en from '../../../locales/en';

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: { en },
});

function factory(props: Record<string, unknown> = {}) {
  return mount(SectionEditor, {
    props: {
      sectionName: 'identity',
      ...props,
    },
    global: {
      plugins: [i18n],
    },
  });
}

describe('SectionEditor', () => {
  it('renders markdown content in the body', () => {
    const wrapper = factory({
      content: '# Identity\n\n- Trait one\n- Trait two',
      status: 'owned',
      lastUpdated: '2026-07-05T12:00:00Z',
    });

    const html = wrapper.html();
    expect(html).toContain('<h1>Identity</h1>');
    expect(html).toContain('<li>Trait one</li>');
  });

  it('shows the localized section title, status badge, and last updated time', () => {
    const wrapper = factory({
      sectionName: 'identity',
      status: 'owned',
      lastUpdated: '2026-07-05T12:00:00Z',
    });

    const text = wrapper.text();
    expect(text).toContain(en.laputa.sections.identity);
    expect(text).toContain(en.laputa.status.owned);
    expect(text).toContain('2026-07-05T12:00:00Z');
  });

  it('shows a loading skeleton instead of content while loading', () => {
    const wrapper = factory({ loading: true });

    expect(wrapper.find('.section-editor-skeleton').exists()).toBe(true);
    expect(wrapper.find('.section-editor-content').exists()).toBe(false);
    expect(wrapper.find('.section-editor-empty-state').exists()).toBe(false);
  });

  it('shows an error state and emits retry when the retry button is clicked', async () => {
    const wrapper = factory({ error: 'Backend unreachable' });

    expect(wrapper.text()).toContain(en.laputa.loadError);
    expect(wrapper.text()).toContain('Backend unreachable');

    await wrapper.find('.section-editor-error button').trigger('click');
    expect(wrapper.emitted('retry')).toHaveLength(1);
  });

  it('shows the empty state when the section exists but has no content', () => {
    const wrapper = factory({
      status: 'owned',
      lastUpdated: '2026-07-05T12:00:00Z',
    });

    expect(wrapper.text()).toContain(en.laputa.emptyTitle);
    expect(wrapper.text()).toContain(en.laputa.emptyDesc);
  });

  it('shows the uninitialized state when no section metadata is provided', () => {
    const wrapper = factory();

    expect(wrapper.text()).toContain(en.laputa.uninitializedTitle);
    expect(wrapper.text()).toContain(en.laputa.uninitializedDesc);
  });

  it('falls back to the tbd badge for unknown status values', () => {
    const wrapper = factory({ status: 'unknown', lastUpdated: '2026-07-05T12:00:00Z' });

    expect(wrapper.text()).toContain(en.laputa.status.tbd);
  });
});

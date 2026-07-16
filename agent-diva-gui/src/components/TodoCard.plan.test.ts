import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import TodoCard from './TodoCard.vue';
import type { UiCard } from '../api/desktop';

const enMessages = {
  todoCard: {
    title: '📋 Todo',
    toggle: 'Expand/Collapse',
    allDoneSummary: '✅ All done · {done}/{total}',
    expand: 'Expand',
    markAllDone: 'Mark all done',
  },
  planUpdateCard: {
    title: '📋 Updated Plan',
    pending: 'Pending',
    inProgress: 'In Progress',
    completed: 'Completed',
  },
};

function createTestI18n() {
  return createI18n({
    legacy: false,
    locale: 'en',
    fallbackLocale: 'en',
    messages: { en: enMessages },
  });
}

function buildPlanCard(overrides: Partial<UiCard> = {}): UiCard {
  return {
    id: 'plan-1',
    kind: 'plan',
    status: 'active',
    title: 'Updated Plan',
    summary: 'Plan update',
    body_markdown: '',
    actions: [],
    plan_items: [
      { step: 'Analyze request', status: 'Completed' },
      { step: 'Draft response', status: 'InProgress' },
      { step: 'Review output', status: 'Pending' },
    ],
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    ...overrides,
  };
}

describe('TodoCard plan update rendering', () => {
  it('renders the plan update title', () => {
    const card = buildPlanCard();
    const wrapper = mount(TodoCard, {
      props: { card },
      global: { plugins: [createTestI18n()] },
    });

    const title = wrapper.find('.todo-title');
    expect(title.exists()).toBe(true);
    expect(title.text()).toContain('Updated Plan');
  });

  it('renders the explanation when present', () => {
    const card = buildPlanCard({ explanation: 'Adapting the plan based on new context.' });
    const wrapper = mount(TodoCard, {
      props: { card },
      global: { plugins: [createTestI18n()] },
    });

    const explanation = wrapper.find('.plan-explanation');
    expect(explanation.exists()).toBe(true);
    expect(explanation.text()).toBe('Adapting the plan based on new context.');
  });

  it('does not render explanation when omitted', () => {
    const card = buildPlanCard({ explanation: undefined });
    const wrapper = mount(TodoCard, {
      props: { card },
      global: { plugins: [createTestI18n()] },
    });

    expect(wrapper.find('.plan-explanation').exists()).toBe(false);
  });

  it('renders all plan items with correct status classes', () => {
    const card = buildPlanCard();
    const wrapper = mount(TodoCard, {
      props: { card },
      global: { plugins: [createTestI18n()] },
    });

    const items = wrapper.findAll('.plan-item');
    expect(items.length).toBe(3);

    expect(items[0].classes()).toContain('status-completed');
    expect(items[0].find('.plan-step').text()).toBe('Analyze request');

    expect(items[1].classes()).toContain('status-in-progress');
    expect(items[1].find('.plan-step').text()).toBe('Draft response');

    expect(items[2].classes()).toContain('status-pending');
    expect(items[2].find('.plan-step').text()).toBe('Review output');
  });

  it('does not render interactive checkboxes for plan updates', () => {
    const card = buildPlanCard();
    const wrapper = mount(TodoCard, {
      props: { card },
      global: { plugins: [createTestI18n()] },
    });

    expect(wrapper.find('.todo-checkbox').exists()).toBe(false);
    expect(wrapper.find('.todo-check-icon').exists()).toBe(false);
  });

  it('hides the mark-all-done footer for plan updates', () => {
    const card = buildPlanCard();
    const wrapper = mount(TodoCard, {
      props: { card },
      global: { plugins: [createTestI18n()] },
    });

    const footer = wrapper.find('.todo-footer');
    expect(footer.exists()).toBe(true);
    expect((footer.element as HTMLElement).style.display).toBe('none');
  });

  it('renders empty plan update without crashing', () => {
    const card = buildPlanCard({ plan_items: [] });
    const wrapper = mount(TodoCard, {
      props: { card },
      global: { plugins: [createTestI18n()] },
    });

    expect(wrapper.find('.todo-title').text()).toContain('Updated Plan');
    expect(wrapper.findAll('.plan-item').length).toBe(0);
  });
});

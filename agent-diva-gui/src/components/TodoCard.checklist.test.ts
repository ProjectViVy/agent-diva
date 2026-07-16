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
  checklistCard: {
    title: '📋 Task Checklist',
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

function buildChecklistCard(overrides: Partial<UiCard> = {}): UiCard {
  return {
    id: 'plan-1',
    kind: 'checklist',
    status: 'active',
    title: 'Task Checklist',
    summary: 'Checklist update',
    body_markdown: '',
    actions: [],
    plan_items: [
      { step: 'Analyze request', status: 'completed' },
      { step: 'Draft response', status: 'in_progress' },
      { step: 'Review output', status: 'pending' },
    ],
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    ...overrides,
  };
}

describe('TodoCard checklist rendering', () => {
  it('renders the checklist title', () => {
    const card = buildChecklistCard();
    const wrapper = mount(TodoCard, {
      props: { card },
      global: { plugins: [createTestI18n()] },
    });

    const title = wrapper.find('.todo-title');
    expect(title.exists()).toBe(true);
    expect(title.text()).toContain('Task Checklist');
  });

  it('renders the explanation when present', () => {
    const card = buildChecklistCard({ explanation: 'Adapting the checklist based on new context.' });
    const wrapper = mount(TodoCard, {
      props: { card },
      global: { plugins: [createTestI18n()] },
    });

    const explanation = wrapper.find('.plan-explanation');
    expect(explanation.exists()).toBe(true);
    expect(explanation.text()).toBe('Adapting the checklist based on new context.');
  });

  it('does not render explanation when omitted', () => {
    const card = buildChecklistCard({ explanation: undefined });
    const wrapper = mount(TodoCard, {
      props: { card },
      global: { plugins: [createTestI18n()] },
    });

    expect(wrapper.find('.plan-explanation').exists()).toBe(false);
  });

  it('renders all plan items with correct status classes', () => {
    const card = buildChecklistCard();
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

  it('renders legacy status spellings for compatibility', () => {
    const card = buildChecklistCard({
      plan_items: [
        { step: 'Legacy done', status: 'Completed' },
        { step: 'Legacy active', status: 'InProgress' },
      ],
    });
    const wrapper = mount(TodoCard, {
      props: { card },
      global: { plugins: [createTestI18n()] },
    });

    const items = wrapper.findAll('.plan-item');
    expect(items[0].classes()).toContain('status-completed');
    expect(items[1].classes()).toContain('status-in-progress');
  });

  it('does not render interactive checkboxes for checklist updates', () => {
    const card = buildChecklistCard();
    const wrapper = mount(TodoCard, {
      props: { card },
      global: { plugins: [createTestI18n()] },
    });

    expect(wrapper.find('.todo-checkbox').exists()).toBe(false);
    expect(wrapper.find('.todo-check-icon').exists()).toBe(false);
  });

  it('hides the mark-all-done footer for checklist updates', () => {
    const card = buildChecklistCard();
    const wrapper = mount(TodoCard, {
      props: { card },
      global: { plugins: [createTestI18n()] },
    });

    const footer = wrapper.find('.todo-footer');
    expect(footer.exists()).toBe(true);
    expect((footer.element as HTMLElement).style.display).toBe('none');
  });

  it('renders an empty checklist without crashing', () => {
    const card = buildChecklistCard({ plan_items: [] });
    const wrapper = mount(TodoCard, {
      props: { card },
      global: { plugins: [createTestI18n()] },
    });

    expect(wrapper.find('.todo-title').text()).toContain('Task Checklist');
    expect(wrapper.findAll('.plan-item').length).toBe(0);
  });
});

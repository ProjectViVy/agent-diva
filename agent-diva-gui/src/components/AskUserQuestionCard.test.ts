import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import AskUserQuestionCard from './AskUserQuestionCard.vue';
import type { AskUserQuestionView } from './AskUserQuestionCard.vue';

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }));

const question: AskUserQuestionView = {
  question_id: 'q-1',
  question: 'Which backend?',
  choices: ['A', 'B'],
  allow_other: true,
  context: null,
  created_at: '2026-08-05T00:00:00Z',
  timeout_seconds: 600,
};

describe('AskUserQuestionCard', () => {
  it('renders the question and emits answer with the selected choice', async () => {
    const wrapper = mount(AskUserQuestionCard, { props: { question } });

    expect(wrapper.text()).toContain('Which backend?');
    expect(wrapper.text()).toContain('A');
    expect(wrapper.text()).toContain('B');

    const buttons = wrapper.findAll('button.ask-user-choice');
    await buttons[1].trigger('click');
    expect(wrapper.emitted('answer')?.[0]).toEqual([
      { question_id: 'q-1', selected_index: 1, other_text: null },
    ]);
  });

  it('emits answer with free text via Other input', async () => {
    const wrapper = mount(AskUserQuestionCard, { props: { question } });

    await wrapper.get('button.ask-user-choice-other').trigger('click');
    const input = wrapper.get('input.ask-user-other-input');
    await input.setValue('custom');
    await wrapper.get('button.ask-user-submit').trigger('click');
    expect(wrapper.emitted('answer')?.[0]).toEqual([
      { question_id: 'q-1', selected_index: null, other_text: 'custom' },
    ]);
  });

  it('emits cancel with the question id', async () => {
    const wrapper = mount(AskUserQuestionCard, { props: { question } });
    await wrapper.get('button.ask-user-card-close').trigger('click');
    expect(wrapper.emitted('cancel')?.[0]).toEqual(['q-1']);
  });

  it('shows context and allows free text without choices', () => {
    const noChoices: AskUserQuestionView = {
      ...question,
      question_id: 'q-2',
      choices: [],
      context: 'some context',
    };
    const wrapper = mount(AskUserQuestionCard, { props: { question: noChoices } });
    expect(wrapper.text()).toContain('some context');
    expect(wrapper.get('input.ask-user-other-input').exists()).toBe(true);
  });
});

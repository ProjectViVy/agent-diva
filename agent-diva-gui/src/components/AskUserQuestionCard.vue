<script setup lang="ts">
import { ref } from 'vue';
import { X } from '@lucide/vue';

export interface AskUserQuestionView {
  question_id: string;
  question: string;
  choices: string[];
  allow_other: boolean;
  context?: string | null;
  created_at: string;
  timeout_seconds: number;
}

const props = defineProps<{
  question: AskUserQuestionView;
  submitting?: boolean;
  error?: string | null;
}>();

const emit = defineEmits<{
  (event: 'answer', payload: { question_id: string; selected_index: number | null; other_text: string | null }): void;
  (event: 'cancel', questionId: string): void;
}>();

const otherText = ref('');
const showOther = ref(false);

function answerPayload(selected_index: number | null, other_text: string | null) {
  return {
    question_id: props.question.question_id,
    selected_index,
    other_text,
  };
}

function choose(index: number) {
  emit('answer', answerPayload(index, null));
}

function submitOther() {
  emit('answer', answerPayload(null, otherText.value));
}

function cancel() {
  emit('cancel', props.question.question_id);
}
</script>

<template>
  <article class="ask-user-question-card" aria-label="Ask user question">
    <header class="ask-user-card-header">
      <span class="ask-user-card-kicker">ask_user</span>
      <button
        class="ask-user-card-close"
        :aria-label="'Cancel question'"
        :disabled="submitting"
        @click="cancel"
      >
        <X :size="14" />
      </button>
    </header>
    <h3 class="ask-user-card-question">{{ question.question }}</h3>
    <p v-if="question.context" class="ask-user-card-context">{{ question.context }}</p>
    <div class="ask-user-card-actions">
      <template v-if="question.choices.length > 0">
        <button
          v-for="(choice, index) in question.choices"
          :key="choice"
          class="ask-user-choice"
          :disabled="submitting"
          @click="choose(index)"
        >
          {{ choice }}
        </button>
        <button
          v-if="question.allow_other && !showOther"
          class="ask-user-choice ask-user-choice-other"
          :disabled="submitting"
          @click="showOther = true"
        >
          Other...
        </button>
      </template>
      <div v-if="question.allow_other && (showOther || question.choices.length === 0)" class="ask-user-other">
        <input
          v-model="otherText"
          class="ask-user-other-input"
          :placeholder="'Free text answer'"
          :disabled="submitting"
          @keyup.enter="submitOther"
        />
        <button class="ask-user-choice ask-user-submit" :disabled="submitting || otherText.trim() === ''" @click="submitOther">
          Submit
        </button>
      </div>
      <p v-if="error" class="ask-user-card-error">{{ error }}</p>
    </div>
  </article>
</template>

<style scoped>
.ask-user-question-card {
  border: 1px solid var(--border-color, #e2e8f0);
  border-radius: 10px;
  padding: 12px 14px;
  background: var(--bg-card, #ffffff);
  margin: 8px 0;
}
.ask-user-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.ask-user-card-kicker {
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--text-dim, #64748b);
}
.ask-user-card-close {
  border: none;
  background: transparent;
  color: var(--text-dim, #64748b);
  cursor: pointer;
  padding: 2px;
}
.ask-user-card-question {
  margin: 6px 0 4px;
  font-size: 14px;
  line-height: 1.4;
}
.ask-user-card-context {
  margin: 0 0 8px;
  font-size: 12px;
  color: var(--text-dim, #64748b);
}
.ask-user-card-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 8px;
}
.ask-user-choice {
  border: 1px solid var(--border-color, #cbd5e1);
  border-radius: 999px;
  background: var(--bg-button, #f1f5f9);
  padding: 4px 12px;
  font-size: 13px;
  cursor: pointer;
}
.ask-user-choice:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.ask-user-choice-other {
  border-style: dashed;
}
.ask-user-other {
  display: flex;
  gap: 8px;
  width: 100%;
}
.ask-user-other-input {
  flex: 1;
  border: 1px solid var(--border-color, #cbd5e1);
  border-radius: 6px;
  padding: 4px 8px;
  font-size: 13px;
}
.ask-user-card-error {
  width: 100%;
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--danger-strong, #dc2626);
}
</style>

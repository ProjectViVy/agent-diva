<script setup lang="ts">
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ChevronDown, ChevronUp, CheckCircle, XCircle } from 'lucide-vue-next';
import type { UiCard, UiCardAction } from '../api/desktop';

const { t } = useI18n();

const props = defineProps<{
  card: UiCard;
  loading?: boolean;
}>();

const emit = defineEmits<{
  (e: 'action', payload: { id: string; decision: 'approved' | 'rejected' }): void;
}>();

const expanded = ref(false);

/** Only render for decision cards */
const isDecision = computed(() => props.card.kind === 'decision');

/** Risk level → CSS variable color */
const riskColor = computed(() => {
  switch (props.card.risk_level) {
    case 'low':
      return 'var(--success)';
    case 'medium':
      return 'var(--warning)';
    case 'high':
      return 'var(--danger)';
    default:
      return 'var(--warning)';
  }
});

/** Risk level label for i18n */
const riskLabel = computed(() => {
  switch (props.card.risk_level) {
    case 'low':
      return t('card.riskLow');
    case 'medium':
      return t('card.riskMedium');
    case 'high':
      return t('card.riskHigh');
    default:
      return t('card.riskMedium');
  }
});

/** Parsed ordered steps from body_markdown */
const steps = computed(() => {
  if (!props.card.body_markdown) return [];
  return props.card.body_markdown
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => /^\d+[\.\)]\s+/.test(line))
    .map((line) => line.replace(/^\d+[\.\)]\s+/, ''));
});

/** Pending step count (for approved collapsed state) */
const pendingCount = computed(() => steps.value.length);

/** Find the approve action from card.actions */
const approveAction = computed(() =>
  props.card.actions.find((a) => a.style === 'primary'),
);

/** Find the reject action from card.actions */
const rejectAction = computed(() =>
  props.card.actions.find((a) => a.style === 'secondary' || a.style === 'danger'),
);

/** Is the card in approved state */
const isApproved = computed(() => props.card.status === 'approved');

/** Is the card in rejected state */
const isRejected = computed(() => props.card.status === 'rejected');

/** Handle approve click */
const handleApprove = () => {
  if (props.loading) return;
  emit('action', { id: props.card.id, decision: 'approved' });
};

/** Handle reject click */
const handleReject = () => {
  if (props.loading) return;
  emit('action', { id: props.card.id, decision: 'rejected' });
};

/** Toggle expanded in approved collapsed state */
const toggleExpanded = () => {
  expanded.value = !expanded.value;
};
</script>

<template>
  <div v-if="isDecision" class="decision-card" :class="{ 'decision-card--approved': isApproved, 'decision-card--rejected': isRejected }">
    <!-- Approved Collapsed State -->
    <div v-if="isApproved && !expanded" class="decision-card__collapsed" @click="toggleExpanded">
      <div class="decision-card__collapsed-left">
        <CheckCircle :size="16" class="decision-card__check-icon" />
        <span class="decision-card__collapsed-text">
          {{ t('card.approved') }} · {{ pendingCount }} {{ t('card.itemsPending') }}
        </span>
      </div>
      <button class="decision-card__expand-btn" @click.stop="toggleExpanded">
        {{ t('card.expand') }}
        <ChevronDown :size="14" />
      </button>
    </div>

    <!-- Rejected Collapsed State -->
    <div v-else-if="isRejected && !expanded" class="decision-card__collapsed" @click="toggleExpanded">
      <div class="decision-card__collapsed-left">
        <XCircle :size="16" class="decision-card__reject-icon" />
        <span class="decision-card__collapsed-text decision-card__collapsed-text--rejected">
          {{ t('card.rejected') }}
        </span>
      </div>
      <button class="decision-card__expand-btn" @click.stop="toggleExpanded">
        {{ t('card.expand') }}
        <ChevronDown :size="14" />
      </button>
    </div>

    <!-- Full Card View -->
    <template v-if="!isApproved || expanded">
      <!-- Header: Title + Risk Dot -->
      <div class="decision-card__header">
        <span class="decision-card__title">{{ card.title }}</span>
        <span class="decision-card__risk-dot" :style="{ backgroundColor: riskColor }" :title="riskLabel" />
      </div>

      <!-- Summary -->
      <p v-if="card.summary" class="decision-card__summary">{{ card.summary }}</p>

      <!-- Ordered Steps -->
      <ol v-if="steps.length > 0" class="decision-card__steps">
        <li v-for="(step, index) in steps" :key="index" class="decision-card__step">
          <span class="decision-card__step-number">{{ index + 1 }}</span>
          <span class="decision-card__step-text">{{ step }}</span>
        </li>
      </ol>

      <!-- Evidence References -->
      <div v-if="card.evidence_refs && card.evidence_refs.length > 0" class="decision-card__evidence">
        <span class="decision-card__evidence-label">{{ t('card.evidence') }}</span>
        <span v-for="(ref, index) in card.evidence_refs" :key="index" class="decision-card__evidence-ref">{{ ref }}</span>
      </div>

      <!-- Collapse button when expanded in approved state -->
      <div v-if="isApproved" class="decision-card__collapse-row">
        <button class="decision-card__expand-btn" @click="toggleExpanded">
          {{ t('card.collapse') }}
          <ChevronUp :size="14" />
        </button>
      </div>

      <!-- Action Buttons (only in draft/pending state) -->
      <div v-if="!isApproved && !isRejected" class="decision-card__actions">
        <button
          class="decision-card__btn decision-card__btn--reject"
          :disabled="loading"
          @click="handleReject"
        >
          {{ rejectAction?.label || t('card.reject') }}
        </button>
        <button
          class="decision-card__btn decision-card__btn--approve"
          :disabled="loading"
          @click="handleApprove"
        >
          {{ approveAction?.label || t('card.approve') }}
        </button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.decision-card {
  background: var(--panel-solid, #fff0f6);
  border: 1px solid var(--line, rgba(255, 182, 193, 0.5));
  border-radius: 12px;
  padding: 12px 16px;
  box-shadow: 0 4px 16px rgba(236, 72, 153, 0.12);
  transition: box-shadow 0.15s ease;
}

.decision-card:hover {
  box-shadow: 0 6px 20px rgba(236, 72, 153, 0.18);
}

/* ── Collapsed Approved State ── */
.decision-card--approved {
  min-height: 48px;
}

.decision-card__collapsed {
  display: flex;
  align-items: center;
  justify-content: space-between;
  cursor: pointer;
  padding: 4px 0;
}

.decision-card__collapsed-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.decision-card__check-icon {
  color: var(--success, #22c55e);
  flex-shrink: 0;
}

.decision-card__collapsed-text {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text-muted, #7a2f3e);
}

.decision-card__expand-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--text-muted, #7a2f3e);
  background: none;
  border: none;
  cursor: pointer;
  padding: 2px 4px;
  border-radius: 4px;
  transition: color 0.15s ease, background 0.15s ease;
}

.decision-card__expand-btn:hover {
  color: var(--text, #6b2737);
  background: var(--accent-bg-light, rgba(236, 72, 153, 0.08));
}

/* ── Header ── */
.decision-card__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.decision-card__title {
  font-size: 0.9375rem;
  font-weight: 600;
  color: var(--text, #6b2737);
  line-height: 1.4;
}

.decision-card__risk-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
  margin-left: 8px;
}

/* ── Summary ── */
.decision-card__summary {
  font-size: 0.875rem;
  font-weight: 400;
  color: var(--text-muted, #7a2f3e);
  line-height: 1.6;
  margin-bottom: 12px;
}

/* ── Ordered Steps ── */
.decision-card__steps {
  list-style: none;
  padding: 0;
  margin: 0 0 12px 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.decision-card__step {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.decision-card__step-number {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--accent, #ec4899);
  color: #fff;
  font-size: 0.6875rem;
  font-weight: 600;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  line-height: 1;
}

.decision-card__step-text {
  font-size: 0.875rem;
  font-weight: 400;
  color: var(--text, #6b2737);
  line-height: 1.6;
  padding-top: 1px;
}

/* ── Evidence ── */
.decision-card__evidence {
  font-size: 0.75rem;
  font-weight: 400;
  color: var(--text-muted, #7a2f3e);
  margin-bottom: 12px;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}

.decision-card__evidence-label {
  font-weight: 500;
}

.decision-card__evidence-ref {
  background: var(--accent-bg-light, rgba(236, 72, 153, 0.08));
  padding: 1px 6px;
  border-radius: 4px;
  font-size: 0.6875rem;
}

/* ── Collapse Row ── */
.decision-card__collapse-row {
  display: flex;
  justify-content: center;
  margin-top: 4px;
}

/* ── Action Buttons ── */
.decision-card__actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}

.decision-card__btn {
  font-size: 0.8125rem;
  font-weight: 500;
  padding: 6px 16px;
  border-radius: 8px;
  cursor: pointer;
  border: none;
  transition: transform 0.15s ease, filter 0.15s ease, opacity 0.15s ease;
}

.decision-card__btn:hover {
  transform: scale(1.02);
  filter: brightness(1.1);
}

.decision-card__btn:active {
  transform: scale(0.98);
}

.decision-card__btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none;
  filter: none;
}

.decision-card__btn--reject {
  background: transparent;
  border: 1px solid var(--line, rgba(255, 182, 193, 0.5));
  color: var(--text-muted, #7a2f3e);
}

.decision-card__btn--reject:hover:not(:disabled) {
  background: var(--accent-bg-light, rgba(236, 72, 153, 0.08));
  border-color: var(--accent-border, rgba(236, 72, 153, 0.4));
  color: var(--text, #6b2737);
}

.decision-card__btn--approve {
  background: linear-gradient(135deg, var(--accent, #ec4899), var(--accent-light, #f472b6));
  color: #fff;
  box-shadow: 0 4px 12px var(--accent-glow, rgba(236, 72, 153, 0.15));
}

.decision-card__btn--approve:hover:not(:disabled) {
  box-shadow: 0 6px 16px var(--accent-glow, rgba(236, 72, 153, 0.25));
}
</style>

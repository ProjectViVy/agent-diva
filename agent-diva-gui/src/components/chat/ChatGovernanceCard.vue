<script setup lang="ts">
import { computed } from 'vue';
import {
  AlertTriangle,
  ArrowRight,
  CheckCircle2,
  Clock3,
  FileSearch,
  GitBranch,
  Loader2,
  ShieldAlert,
} from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import type { ChatGovernanceCard, ChatGovernanceDeepLink } from './governanceCards';

const props = defineProps<{
  card: ChatGovernanceCard;
}>();

const emit = defineEmits<{
  (event: 'open-evolution', payload: ChatGovernanceDeepLink): void;
}>();

const { t } = useI18n();

const isRunCard = computed(() => props.card.kind === 'autodream_run');
const cardError = computed(() => props.card.error?.trim() || null);

const runCard = computed(() =>
  props.card.kind === 'autodream_run' ? props.card : null,
);

const proposalCard = computed(() =>
  props.card.kind === 'skill_request' ? props.card : null,
);

const runTone = computed(() => {
  const state = runCard.value?.state;
  if (state === 'completed') return 'success';
  if (state === 'failed' || state === 'cancelled' || state === 'unavailable') return 'danger';
  return 'running';
});

const riskTone = computed(() => {
  if (proposalCard.value?.state === 'stale' || proposalCard.value?.state === 'rejected') return 'danger';
  if (proposalCard.value?.state === 'pending') return 'warning';
  return 'success';
});

const primaryLink = computed<ChatGovernanceDeepLink>(() => {
  if (runCard.value) {
    return runCard.value.proposal_ids?.length
      ? { tab: 'requests', proposalId: runCard.value.proposal_ids[0], sourceRunId: runCard.value.id }
      : { tab: 'autodream', sourceRunId: runCard.value.id };
  }

  return {
    tab: 'requests',
    proposalId: proposalCard.value?.id ?? null,
    sourceRunId: proposalCard.value?.source_run_id ?? null,
  };
});

const statusIcon = computed(() => {
  if (runTone.value === 'success') return CheckCircle2;
  if (runTone.value === 'danger') return AlertTriangle;
  return Loader2;
});

const proposalSummary = computed(() => {
  if (!proposalCard.value?.summary) return t('chatGovernance.proposalSummaryFallback');
  return proposalCard.value.summary;
});

function openEvolution() {
  emit('open-evolution', primaryLink.value);
}
</script>

<template>
  <article class="chat-governance-card" :data-kind="card.kind">
    <header class="chat-governance-card__header">
      <div class="chat-governance-card__icon" :class="isRunCard ? runTone : riskTone">
        <component :is="isRunCard ? statusIcon : GitBranch" :size="16" :class="{ 'animate-spin': runTone === 'running' }" />
      </div>
      <div class="chat-governance-card__title">
        <strong>{{ isRunCard ? t('chatGovernance.runTitle') : t('chatGovernance.proposalTitle') }}</strong>
        <span>
          {{
            isRunCard
              ? t(`chatGovernance.runState.${runCard?.state || 'running'}`)
              : `${proposalCard?.state} · ${proposalCard?.source}`
          }}
        </span>
      </div>
    </header>

    <div v-if="runCard" class="chat-governance-card__body">
      <p>{{ runCard.summary || t('chatGovernance.runSummaryFallback') }}</p>
      <dl class="chat-governance-card__meta">
        <div>
          <dt><Clock3 :size="12" />{{ t('chatGovernance.trigger') }}</dt>
          <dd>{{ runCard.trigger || 'manual' }}</dd>
        </div>
        <div>
          <dt><FileSearch :size="12" />{{ t('chatGovernance.proposals') }}</dt>
          <dd>{{ runCard.proposal_ids?.length || 0 }}</dd>
        </div>
      </dl>
    </div>

    <div v-else-if="proposalCard" class="chat-governance-card__body">
      <p>{{ proposalSummary }}</p>
      <dl class="chat-governance-card__meta">
        <div>
          <dt>{{ t('chatGovernance.type') }}</dt>
          <dd>{{ proposalCard.source }}</dd>
        </div>
        <div>
          <dt>{{ t('chatGovernance.target') }}</dt>
          <dd>{{ proposalCard.slug }}</dd>
        </div>
        <div>
          <dt>{{ t('chatGovernance.evidence') }}</dt>
          <dd>{{ proposalCard.evidence_count ?? 0 }}</dd>
        </div>
      </dl>
    </div>

    <div v-if="cardError" class="chat-governance-card__error" role="status">
      <ShieldAlert :size="14" />
      <span>{{ cardError }}</span>
    </div>

    <footer class="chat-governance-card__footer">
      <button type="button" class="chat-governance-card__action" @click="openEvolution">
        <span>{{ cardError ? t('chatGovernance.openForRetry') : t('chatGovernance.openEvolution') }}</span>
        <ArrowRight :size="14" />
      </button>
    </footer>
  </article>
</template>

<style scoped>
.chat-governance-card {
  width: min(420px, 100%);
  overflow: hidden;
  border: 1px solid #d8dee9;
  border-radius: 8px;
  background: var(--panel-solid, #ffffff);
  color: var(--text, #111827);
  box-shadow: 0 10px 24px rgba(15, 23, 42, 0.08);
}

.chat-governance-card__header,
.chat-governance-card__footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 10px 12px;
}

.chat-governance-card__header {
  border-bottom: 1px solid #eef2f7;
}

.chat-governance-card__icon {
  display: grid;
  height: 28px;
  width: 28px;
  flex: 0 0 auto;
  place-items: center;
  border-radius: 6px;
}

.chat-governance-card__icon.running {
  background: #eff6ff;
  color: #2563eb;
}

.chat-governance-card__icon.success {
  background: #ecfdf5;
  color: #059669;
}

.chat-governance-card__icon.warning {
  background: #fffbeb;
  color: var(--warning-strong, #d97706);
}

.chat-governance-card__icon.danger {
  background: #fef2f2;
  color: var(--danger-strong, #dc2626);
}

.chat-governance-card__title {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-direction: column;
  gap: 2px;
}

.chat-governance-card__title strong {
  font-size: 13px;
  line-height: 1.2;
}

.chat-governance-card__title span {
  overflow: hidden;
  color: #64748b;
  font-size: 11px;
  line-height: 1.2;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chat-governance-card__body {
  display: grid;
  gap: 10px;
  padding: 12px;
}

.chat-governance-card__body p {
  margin: 0;
  color: #334155;
  font-size: 12px;
  line-height: 1.5;
}

.chat-governance-card__meta {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(110px, 1fr));
  gap: 8px;
  margin: 0;
}

.chat-governance-card__meta div {
  min-width: 0;
  border: 1px solid #eef2f7;
  border-radius: 6px;
  padding: 7px 8px;
  background: #f8fafc;
}

.chat-governance-card__meta dt {
  display: flex;
  align-items: center;
  gap: 4px;
  color: #64748b;
  font-size: 10px;
  line-height: 1.2;
  text-transform: uppercase;
}

.chat-governance-card__meta dd {
  overflow: hidden;
  margin: 3px 0 0;
  color: var(--text, #111827);
  font-size: 12px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chat-governance-card__error {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  margin: 0 12px 10px;
  border: 1px solid #fecaca;
  border-radius: 6px;
  background: #fef2f2;
  padding: 8px;
  color: #991b1b;
  font-size: 12px;
  line-height: 1.35;
}

.chat-governance-card__footer {
  justify-content: flex-end;
  border-top: 1px solid #eef2f7;
  background: #f8fafc;
}

.chat-governance-card__action {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 1px solid #cbd5e1;
  border-radius: 6px;
  background: var(--panel-solid, #ffffff);
  padding: 6px 9px;
  color: var(--line, #1f2937);
  font-size: 12px;
  font-weight: 600;
  transition: border-color 0.16s ease, color 0.16s ease;
}

.chat-governance-card__action:hover {
  border-color: #2563eb;
  color: #2563eb;
}
</style>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { AlertTriangle, Check, Edit3, RotateCcw, ShieldAlert, X } from 'lucide-vue-next';
import type { EvolutionProposal } from '../../api/desktop';

const { t } = useI18n();

const props = defineProps<{
  proposal: EvolutionProposal;
  busyAction: string | null;
  disableApproval: boolean;
  disableRollback: boolean;
  missingEvidence: boolean;
  rollbackReason?: string | null;
}>();

const emit = defineEmits<{
  (event: 'approve-apply'): void;
  (event: 'approve-only'): void;
  (event: 'edit'): void;
  (event: 'reject'): void;
  (event: 'defer'): void;
  (event: 'rollback'): void;
}>();

const canApprove = computed(
  () => !props.disableApproval && props.proposal.state !== 'applied' && props.proposal.state !== 'rejected',
);

const approveHint = computed(() => {
  if (
    props.missingEvidence &&
    (props.proposal.risk_level === 'high' || props.proposal.risk_level === 'critical')
  ) {
    return t('evolution.actions.highRiskMissingEvidence');
  }
  if (props.disableApproval) {
    return t('evolution.actions.unavailable');
  }
  return '';
});

const rollbackHint = computed(() => {
  if (props.disableRollback) {
    return props.rollbackReason || t('evolution.actions.rollbackUnavailable');
  }
  return '';
});
</script>

<template>
  <div class="governance-action-bar">
    <div v-if="proposal.risk_level === 'high' || proposal.risk_level === 'critical'" class="governance-risk-banner">
      <ShieldAlert :size="16" />
      <span>{{ t('evolution.actions.highRiskBanner') }}</span>
    </div>
    <div v-if="missingEvidence" class="governance-warning-banner">
      <AlertTriangle :size="16" />
      <span>{{ t('evolution.actions.missingEvidence') }}</span>
    </div>

    <div class="governance-grid">
      <button
        class="governance-btn governance-btn--primary"
        type="button"
        :disabled="busyAction !== null || !canApprove"
        :title="approveHint"
        @click="emit('approve-apply')"
      >
        <Check :size="15" />
        <span>{{ t('evolution.actions.approveApply') }}</span>
      </button>
      <button
        class="governance-btn"
        type="button"
        :disabled="busyAction !== null || !canApprove"
        :title="approveHint"
        @click="emit('approve-only')"
      >
        <Check :size="15" />
        <span>{{ t('evolution.actions.approveOnly') }}</span>
      </button>
      <button class="governance-btn" type="button" :disabled="busyAction !== null" @click="emit('edit')">
        <Edit3 :size="15" />
        <span>{{ t('evolution.actions.edit') }}</span>
      </button>
      <button class="governance-btn governance-btn--danger" type="button" :disabled="busyAction !== null" @click="emit('reject')">
        <X :size="15" />
        <span>{{ t('evolution.actions.reject') }}</span>
      </button>
      <button class="governance-btn" type="button" :disabled="busyAction !== null" @click="emit('defer')">
        <AlertTriangle :size="15" />
        <span>{{ t('evolution.actions.defer') }}</span>
      </button>
      <button
        class="governance-btn"
        type="button"
        :disabled="busyAction !== null || disableRollback"
        :title="rollbackHint"
        @click="emit('rollback')"
      >
        <RotateCcw :size="15" />
        <span>{{ t('evolution.actions.rollback') }}</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.governance-action-bar {
  display: grid;
  gap: 12px;
}

.governance-risk-banner,
.governance-warning-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  border-radius: 8px;
  padding: 10px 12px;
  font-size: 13px;
}

.governance-risk-banner {
  background: #fff3c4;
  color: #854d0e;
}

.governance-warning-banner {
  background: #fee2e2;
  color: #991b1b;
}

.governance-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.governance-btn {
  display: inline-flex;
  min-width: 0;
  align-items: center;
  justify-content: center;
  gap: 8px;
  border: 1px solid #cbd5e1;
  border-radius: 8px;
  background: #ffffff;
  color: #0f172a;
  min-height: 40px;
  padding: 0 12px;
  font-size: 13px;
  font-weight: 600;
}

.governance-btn--primary {
  background: #0f766e;
  border-color: #0f766e;
  color: #f8fafc;
}

.governance-btn--danger {
  border-color: #dc2626;
  color: #b91c1c;
}

.governance-btn:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

@media (max-width: 880px) {
  .governance-grid {
    grid-template-columns: 1fr;
  }
}
</style>

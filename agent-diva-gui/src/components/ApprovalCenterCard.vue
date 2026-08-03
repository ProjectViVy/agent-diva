<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { AlertTriangle, Clock3, FileDiff, RefreshCw, ShieldCheck, ShieldX, X } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import type { ApprovalGrant, ApprovalView } from '../api/approvals';

const props = withDefaults(defineProps<{
  approval: ApprovalView;
  detail?: ApprovalView | null;
  compact?: boolean;
  submitting?: boolean;
  error?: string | null;
  outcomeUnknown?: boolean;
}>(), {
  detail: null,
  compact: false,
  submitting: false,
  error: null,
  outcomeUnknown: false,
});

const emit = defineEmits<{
  (event: 'inspect', requestId: string): void;
  (event: 'decide', payload: { approval: ApprovalView; decision: 'allow' | 'deny'; grant: ApprovalGrant }): void;
  (event: 'cancel', approval: ApprovalView): void;
  (event: 'edit', approval: ApprovalView): void;
  (event: 'refresh', requestId: string): void;
}>();

const { t } = useI18n();
const now = ref(Date.now());
const grant = ref<ApprovalGrant>('once');
let timer: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  timer = setInterval(() => { now.value = Date.now(); }, 1000);
});
onBeforeUnmount(() => {
  if (timer) clearInterval(timer);
});

const view = computed(() => props.detail ?? props.approval);
const presentation = computed(() => view.value.presentation ?? {});
const remainingSeconds = computed(() => Math.max(0, Math.ceil((Date.parse(view.value.expires_at) - now.value) / 1000)));
const terminal = computed(() => !['pending', 'allowed'].includes(view.value.status));
const highRiskMissingEvidence = computed(() =>
  view.value.domain === 'memory' && view.value.risk === 'high' && view.value.evidence.length === 0,
);
const allowDisabled = computed(() =>
  props.submitting || props.outcomeUnknown || terminal.value || remainingSeconds.value === 0 || highRiskMissingEvidence.value,
);
const title = computed(() => String(presentation.value.title ?? t(`approvalCenter.domain.${view.value.domain}`)));
const summary = computed(() => {
  const value = presentation.value.summary ?? presentation.value.command ?? presentation.value.target_section;
  return typeof value === 'string' ? value : '';
});
const diff = computed(() => typeof presentation.value.diff === 'string' ? presentation.value.diff : '');
const can = (action: string) => view.value.actions.includes(action);
const canUseRuleGrant = computed(() => presentation.value.suggested_prefix != null);
</script>

<template>
  <article
    class="approval-center-card"
    :class="[`risk-${view.risk}`, `status-${view.status}`, { compact }]"
    :aria-label="t('approvalCenter.cardLabel', { domain: view.domain, title })"
  >
    <header class="approval-card-header">
      <div>
        <div class="approval-card-kicker">
          <span>{{ t(`approvalCenter.domain.${view.domain}`) }}</span>
          <span class="approval-status-text">{{ t(`approvalCenter.status.${view.status}`) }}</span>
        </div>
        <h3>{{ title }}</h3>
      </div>
      <span class="approval-risk" :aria-label="t('approvalCenter.riskLabel', { risk: view.risk })">
        {{ t(`approvalCenter.risk.${view.risk}`) }}
      </span>
    </header>

    <p v-if="summary" class="approval-summary">{{ summary }}</p>
    <dl class="approval-meta">
      <div><dt>{{ t('approvalCenter.scope') }}</dt><dd>{{ view.resource.session_id ?? view.resource.workspace_id }}</dd></div>
      <div><dt>{{ t('approvalCenter.capability') }}</dt><dd>{{ view.capability }}</dd></div>
      <div><dt>{{ t('approvalCenter.version') }}</dt><dd>{{ view.version }}</dd></div>
      <div><dt><Clock3 :size="13" />{{ t('approvalCenter.ttl') }}</dt><dd>{{ remainingSeconds }}s</dd></div>
    </dl>

    <button v-if="!detail && !compact" type="button" class="approval-link" @click="emit('inspect', view.request_id)">
      {{ t('approvalCenter.inspect') }}
    </button>

    <div v-if="detail && !compact" class="approval-detail">
      <p>{{ t('approvalCenter.evidenceCount', { count: view.evidence.length }) }}</p>
      <pre v-if="diff" class="approval-diff"><FileDiff :size="14" />{{ diff }}</pre>
      <p v-if="view.reason_code" class="approval-reason">{{ view.reason_code }}</p>
    </div>

    <p v-if="highRiskMissingEvidence" class="approval-warning"><AlertTriangle :size="15" />{{ t('approvalCenter.missingEvidence') }}</p>
    <p v-if="outcomeUnknown" class="approval-warning" role="status"><AlertTriangle :size="15" />{{ t('approvalCenter.outcomeUnknown') }}</p>
    <p v-if="error" class="approval-error" role="alert">{{ error }}</p>

    <div v-if="view.status === 'pending'" class="approval-actions">
      <label v-if="view.domain === 'command' && can('allow')" class="approval-grant">
        <span>{{ t('approvalCenter.grant') }}</span>
        <select v-model="grant" :disabled="submitting || outcomeUnknown">
          <option value="once">{{ t('approvalCenter.grants.once') }}</option>
          <option value="session">{{ t('approvalCenter.grants.session') }}</option>
          <option v-if="canUseRuleGrant" value="rule">{{ t('approvalCenter.grants.rule') }}</option>
        </select>
      </label>
      <button v-if="can('deny')" type="button" class="deny" :disabled="submitting || outcomeUnknown" @click="emit('decide', { approval: view, decision: 'deny', grant: 'once' })">
        <ShieldX :size="16" />{{ t('approvalCenter.deny') }}
      </button>
      <button v-if="can('allow')" type="button" class="allow" :disabled="allowDisabled" @click="emit('decide', { approval: view, decision: 'allow', grant })">
        <ShieldCheck :size="16" />{{ t('approvalCenter.allow') }}
      </button>
      <button v-if="can('edit')" type="button" :disabled="submitting || outcomeUnknown" @click="emit('edit', view)">{{ t('approvalCenter.edit') }}</button>
      <button v-if="can('cancel')" type="button" :disabled="submitting || outcomeUnknown" @click="emit('cancel', view)">
        <X :size="16" />{{ t('approvalCenter.cancel') }}
      </button>
    </div>
    <div v-else-if="view.status === 'allowed' && can('apply')" class="approval-actions">
      <button type="button" class="allow" :disabled="submitting || outcomeUnknown" @click="emit('edit', view)">
        <ShieldCheck :size="16" />{{ t('approvalCenter.applyAtSource') }}
      </button>
    </div>
    <button v-if="outcomeUnknown || error" type="button" class="approval-refresh" @click="emit('refresh', view.request_id)">
      <RefreshCw :size="15" />{{ t('approvalCenter.refreshOnly') }}
    </button>
  </article>
</template>

<style scoped>
.approval-center-card { border: 1px solid var(--line, #d9dce3); border-left-width: 4px; border-radius: 14px; padding: 14px; background: var(--panel-solid, #fff); color: var(--text, #20242d); box-shadow: 0 8px 24px rgba(15, 23, 42, .06); }
.approval-center-card.risk-high { border-left-color: #dc2626; }.approval-center-card.risk-medium { border-left-color: #d97706; }.approval-center-card.risk-low { border-left-color: #059669; }
.approval-card-header, .approval-card-kicker, .approval-actions, .approval-warning, .approval-refresh, .approval-meta dt { display: flex; align-items: center; }
.approval-card-header { justify-content: space-between; gap: 12px; }.approval-card-header h3 { margin: 3px 0 0; font-size: 15px; }.approval-card-kicker { gap: 8px; color: var(--text-muted, #667085); font-size: 11px; text-transform: uppercase; letter-spacing: .05em; }
.approval-status-text { font-weight: 700; color: var(--text, #20242d); }.approval-risk { border: 1px solid currentColor; border-radius: 999px; padding: 4px 8px; font-size: 11px; font-weight: 700; }
.approval-summary { margin: 10px 0; overflow-wrap: anywhere; font-family: ui-monospace, monospace; font-size: 12px; }.approval-meta { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 7px; margin: 10px 0; }.approval-meta div { min-width: 0; }.approval-meta dt { gap: 4px; color: var(--text-muted, #667085); font-size: 10px; }.approval-meta dd { margin: 2px 0 0; overflow: hidden; text-overflow: ellipsis; font-size: 12px; white-space: nowrap; }
.approval-link { border: 0; padding: 4px 0; color: #2563eb; background: transparent; cursor: pointer; }.approval-detail { margin-top: 10px; font-size: 12px; }.approval-diff { display: flex; max-height: 180px; overflow: auto; gap: 6px; padding: 10px; border-radius: 8px; background: #111827; color: #e5e7eb; white-space: pre-wrap; }.approval-reason { font-family: ui-monospace, monospace; }
.approval-warning, .approval-error { gap: 7px; margin: 9px 0; border-radius: 8px; padding: 9px; font-size: 12px; }.approval-warning { background: #fff7ed; color: #9a3412; }.approval-error { background: #fef2f2; color: #991b1b; }
.approval-actions { flex-wrap: wrap; gap: 8px; margin-top: 12px; }.approval-actions button, .approval-refresh { min-height: 44px; border: 1px solid var(--line, #d9dce3); border-radius: 10px; padding: 8px 11px; background: var(--panel-solid, #fff); cursor: pointer; }.approval-actions button:disabled { cursor: not-allowed; opacity: .55; }.approval-actions .allow { border-color: #047857; background: #047857; color: #fff; }.approval-actions .deny { border-color: #b91c1c; color: #b91c1c; }.approval-grant { display: flex; min-height: 44px; align-items: center; gap: 6px; font-size: 11px; }.approval-grant select { min-height: 36px; border: 1px solid var(--line, #d9dce3); border-radius: 8px; background: var(--panel-solid, #fff); }.approval-refresh { gap: 6px; margin-top: 8px; }.compact { box-shadow: none; }
@media (max-width: 520px) { .approval-meta { grid-template-columns: 1fr; } }
</style>

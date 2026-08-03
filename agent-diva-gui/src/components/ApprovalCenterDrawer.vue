<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { Inbox, RefreshCw, ShieldCheck, X } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import type { ApprovalGrant, ApprovalView } from '../api/approvals';
import ApprovalCenterCard from './ApprovalCenterCard.vue';

const props = withDefaults(defineProps<{
  open: boolean;
  approvals: ApprovalView[];
  details: Record<string, ApprovalView>;
  loading?: boolean;
  error?: string | null;
  submittingIds?: string[];
  outcomeUnknownIds?: string[];
  actionErrors?: Record<string, string>;
}>(), {
  loading: false,
  error: null,
  submittingIds: () => [],
  outcomeUnknownIds: () => [],
  actionErrors: () => ({}),
});

const emit = defineEmits<{
  (event: 'update:open', open: boolean): void;
  (event: 'refresh'): void;
  (event: 'inspect', requestId: string): void;
  (event: 'decide', payload: { approval: ApprovalView; decision: 'allow' | 'deny'; grant: ApprovalGrant }): void;
  (event: 'cancel', approval: ApprovalView): void;
  (event: 'edit', approval: ApprovalView): void;
  (event: 'refresh-one', requestId: string): void;
}>();

const { t } = useI18n();
const domain = ref('all');
const status = ref('pending');
const session = ref('all');
const closeButton = ref<HTMLButtonElement | null>(null);
const drawer = ref<HTMLElement | null>(null);

const pendingCount = computed(() => props.approvals.filter((item) => item.status === 'pending').length);
const sessions = computed(() => [...new Set(props.approvals.map((item) => item.resource.session_id).filter(Boolean) as string[])].sort());
const riskRank: Record<string, number> = { prohibited: 0, critical: 1, high: 2, moderate: 3, low: 4, unknown: 5 };
const visible = computed(() => props.approvals
  .filter((item) => domain.value === 'all' || item.domain === domain.value)
  .filter((item) => status.value === 'all' || item.status === status.value)
  .filter((item) => session.value === 'all' || item.resource.session_id === session.value)
  .sort((left, right) => (riskRank[left.risk] ?? 9) - (riskRank[right.risk] ?? 9)
    || Date.parse(left.expires_at) - Date.parse(right.expires_at)));

watch(() => props.open, async (open) => {
  if (open) {
    await nextTick();
    closeButton.value?.focus();
  }
});

function trapFocus(event: KeyboardEvent) {
  if (event.key !== 'Tab' || !drawer.value) return;
  const focusable = [...drawer.value.querySelectorAll<HTMLElement>('button:not([disabled]), select:not([disabled]), [tabindex="0"]')];
  if (!focusable.length) return;
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}
</script>

<template>
  <button
    type="button"
    class="approval-center-trigger"
    :aria-label="t('approvalCenter.open', { count: pendingCount })"
    :aria-expanded="open"
    @click="emit('update:open', !open)"
  >
    <ShieldCheck :size="18" />
    <span>{{ t('approvalCenter.title') }}</span>
    <strong v-if="pendingCount" aria-live="polite">{{ pendingCount }}</strong>
  </button>

  <Teleport to="body">
    <div v-if="open" class="approval-center-backdrop" @click.self="emit('update:open', false)">
      <aside ref="drawer" class="approval-center-drawer" role="dialog" aria-modal="true" :aria-label="t('approvalCenter.title')" @keydown.esc="emit('update:open', false)" @keydown="trapFocus">
        <header class="drawer-header">
          <div><span>{{ t('approvalCenter.eyebrow') }}</span><h2>{{ t('approvalCenter.title') }}</h2></div>
          <div class="drawer-header-actions">
            <button type="button" :aria-label="t('approvalCenter.refresh')" :disabled="loading" @click="emit('refresh')"><RefreshCw :size="18" :class="{ spin: loading }" /></button>
            <button ref="closeButton" type="button" :aria-label="t('approvalCenter.close')" @click="emit('update:open', false)"><X :size="20" /></button>
          </div>
        </header>

        <div class="drawer-filters" :aria-label="t('approvalCenter.filters')">
          <label>{{ t('approvalCenter.domainFilter') }}<select v-model="domain"><option value="all">{{ t('approvalCenter.all') }}</option><option value="command">Command</option><option value="plan">Plan</option><option value="memory">Memory</option></select></label>
          <label>{{ t('approvalCenter.statusFilter') }}<select v-model="status"><option value="all">{{ t('approvalCenter.all') }}</option><option value="pending">{{ t('approvalCenter.status.pending') }}</option><option value="allowed">{{ t('approvalCenter.status.allowed') }}</option><option value="denied">{{ t('approvalCenter.status.denied') }}</option><option value="revoked">{{ t('approvalCenter.status.revoked') }}</option><option value="consumed">{{ t('approvalCenter.status.consumed') }}</option><option value="expired">{{ t('approvalCenter.status.expired') }}</option></select></label>
          <label>{{ t('approvalCenter.sessionFilter') }}<select v-model="session"><option value="all">{{ t('approvalCenter.all') }}</option><option v-for="value in sessions" :key="value" :value="value">{{ value }}</option></select></label>
        </div>

        <p v-if="error" class="drawer-error" role="alert">{{ error }}</p>
        <div v-if="visible.length" class="drawer-list">
          <ApprovalCenterCard
            v-for="approval in visible"
            :key="approval.request_id"
            :approval="approval"
            :detail="details[approval.request_id]"
            :submitting="submittingIds.includes(approval.request_id)"
            :outcome-unknown="outcomeUnknownIds.includes(approval.request_id)"
            :error="actionErrors[approval.request_id]"
            @inspect="emit('inspect', $event)"
            @decide="emit('decide', $event)"
            @cancel="emit('cancel', $event)"
            @edit="emit('edit', $event)"
            @refresh="emit('refresh-one', $event)"
          />
        </div>
        <div v-else class="drawer-empty"><Inbox :size="28" /><p>{{ loading ? t('approvalCenter.loading') : t('approvalCenter.empty') }}</p></div>
      </aside>
    </div>
  </Teleport>
</template>

<style scoped>
.approval-center-trigger { position: fixed; z-index: 180; top: 10px; right: 76px; display: inline-flex; min-height: 44px; align-items: center; gap: 7px; border: 1px solid #f59e0b; border-radius: 12px; padding: 8px 11px; background: var(--panel-solid, #fff); color: var(--text, #20242d); box-shadow: 0 8px 24px rgba(15, 23, 42, .14); cursor: pointer; }.approval-center-trigger strong { display: grid; min-width: 21px; height: 21px; place-items: center; border-radius: 999px; background: #b91c1c; color: #fff; font-size: 11px; }
.approval-center-backdrop { position: fixed; z-index: 1000; inset: 0; background: rgba(15, 23, 42, .34); }.approval-center-drawer { position: absolute; top: 0; right: 0; display: flex; width: min(520px, 100vw); height: 100%; flex-direction: column; background: var(--panel-solid, #fff); box-shadow: -18px 0 50px rgba(15, 23, 42, .18); }
.drawer-header { display: flex; align-items: center; justify-content: space-between; padding: 18px; border-bottom: 1px solid var(--line, #d9dce3); }.drawer-header span { color: #b45309; font-size: 11px; font-weight: 700; letter-spacing: .08em; text-transform: uppercase; }.drawer-header h2 { margin: 3px 0 0; font-size: 20px; }.drawer-header-actions { display: flex; gap: 8px; }.drawer-header-actions button { display: grid; width: 44px; height: 44px; place-items: center; border: 1px solid var(--line, #d9dce3); border-radius: 10px; background: transparent; cursor: pointer; }
.drawer-filters { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; padding: 12px 18px; border-bottom: 1px solid var(--line, #d9dce3); }.drawer-filters label { display: grid; gap: 4px; color: var(--text-muted, #667085); font-size: 10px; }.drawer-filters select { min-width: 0; min-height: 38px; border: 1px solid var(--line, #d9dce3); border-radius: 8px; background: var(--panel-solid, #fff); color: var(--text, #20242d); }
.drawer-list { display: grid; flex: 1; align-content: start; gap: 12px; overflow-y: auto; padding: 16px 18px 28px; }.drawer-empty { display: grid; flex: 1; place-content: center; justify-items: center; color: var(--text-muted, #667085); }.drawer-error { margin: 12px 18px 0; border-radius: 8px; padding: 9px; background: #fef2f2; color: #991b1b; font-size: 12px; }.spin { animation: spin .8s linear infinite; }@keyframes spin { to { transform: rotate(360deg); } }
@media (max-width: 560px) { .approval-center-trigger span { display: none; }.approval-center-trigger { right: 58px; }.drawer-filters { grid-template-columns: 1fr; } }
</style>

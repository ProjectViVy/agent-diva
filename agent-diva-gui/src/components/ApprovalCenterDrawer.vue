<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { Inbox, RefreshCw, X } from '@lucide/vue';
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

function close() {
  emit('update:open', false);
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="approval-center-backdrop"
      @click.self="close"
    >
      <aside
        ref="drawer"
        class="approval-center-drawer"
        role="dialog"
        aria-modal="true"
        :aria-label="t('approvalCenter.title')"
        @keydown.esc="close"
        @keydown="trapFocus"
      >
        <header class="drawer-header">
          <div class="drawer-title-block">
            <span class="drawer-eyebrow">{{ t('approvalCenter.eyebrow') }}</span>
            <h2>{{ t('approvalCenter.title') }}</h2>
          </div>
          <div class="drawer-header-actions">
            <button
              type="button"
              class="drawer-icon-btn"
              :aria-label="t('approvalCenter.refresh')"
              :disabled="loading"
              @click="emit('refresh')"
            >
              <RefreshCw :size="18" :class="{ spin: loading }" />
            </button>
            <button
              ref="closeButton"
              type="button"
              class="drawer-icon-btn"
              :aria-label="t('approvalCenter.close')"
              @click="close"
            >
              <X :size="20" />
            </button>
          </div>
        </header>

        <div class="drawer-filters" :aria-label="t('approvalCenter.filters')">
          <label class="drawer-filter-field">
            <span>{{ t('approvalCenter.domainFilter') }}</span>
            <select v-model="domain">
              <option value="all">{{ t('approvalCenter.all') }}</option>
              <option value="command">Command</option>
              <option value="plan">Plan</option>
            </select>
          </label>
          <label class="drawer-filter-field">
            <span>{{ t('approvalCenter.statusFilter') }}</span>
            <select v-model="status">
              <option value="all">{{ t('approvalCenter.all') }}</option>
              <option value="pending">{{ t('approvalCenter.status.pending') }}</option>
              <option value="allowed">{{ t('approvalCenter.status.allowed') }}</option>
              <option value="denied">{{ t('approvalCenter.status.denied') }}</option>
              <option value="revoked">{{ t('approvalCenter.status.revoked') }}</option>
              <option value="consumed">{{ t('approvalCenter.status.consumed') }}</option>
              <option value="expired">{{ t('approvalCenter.status.expired') }}</option>
            </select>
          </label>
          <label class="drawer-filter-field">
            <span>{{ t('approvalCenter.sessionFilter') }}</span>
            <select v-model="session">
              <option value="all">{{ t('approvalCenter.all') }}</option>
              <option v-for="value in sessions" :key="value" :value="value">{{ value }}</option>
            </select>
          </label>
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
        <div v-else class="drawer-empty">
          <Inbox :size="28" />
          <p>{{ loading ? t('approvalCenter.loading') : t('approvalCenter.empty') }}</p>
        </div>
      </aside>
    </div>
  </Teleport>
</template>

<style scoped>
.approval-center-backdrop {
  position: fixed;
  z-index: 1000;
  inset: 0;
  display: flex;
  justify-content: flex-end;
  background: rgba(15, 23, 42, 0.34);
}

.approval-center-drawer {
  display: flex;
  width: min(520px, 100vw);
  height: 100%;
  flex-direction: column;
  background: var(--panel-solid, #fff);
  color: var(--text, #20242d);
  box-shadow: -18px 0 50px rgba(15, 23, 42, 0.18);
}

.drawer-header {
  display: flex;
  flex-shrink: 0;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3, 12px);
  padding: 18px 18px 14px;
  border-bottom: 1px solid var(--line, #d9dce3);
}

.drawer-title-block {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: var(--space-1, 4px);
}

.drawer-eyebrow {
  color: #b45309;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.drawer-header h2 {
  margin: 0;
  font-size: var(--font-size-2xl, 20px);
  font-weight: 700;
  line-height: 1.25;
  color: var(--text, #20242d);
}

.drawer-header-actions {
  display: flex;
  flex-shrink: 0;
  gap: var(--space-2, 8px);
}

.drawer-icon-btn {
  display: grid;
  width: 40px;
  height: 40px;
  place-items: center;
  border: 1px solid var(--line, #d9dce3);
  border-radius: 10px;
  color: var(--text, #20242d);
  background: transparent;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease;
}

.drawer-icon-btn:hover:not(:disabled) {
  background: var(--nav-hover, rgba(0, 0, 0, 0.04));
  border-color: var(--brand, #ec4899);
}

.drawer-icon-btn:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.drawer-filters {
  display: grid;
  flex-shrink: 0;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
  padding: var(--space-3, 12px) 18px;
  border-bottom: 1px solid var(--line, #d9dce3);
}

.drawer-filter-field {
  display: grid;
  gap: 5px;
  min-width: 0;
  color: var(--text-muted, #667085);
  font-size: 11px;
  font-weight: 600;
}

.drawer-filter-field select {
  width: 100%;
  min-width: 0;
  min-height: 38px;
  border: 1px solid var(--line, #d9dce3);
  border-radius: 8px;
  padding: 0 10px;
  background: var(--panel-solid, #fff);
  color: var(--text, #20242d);
  font-size: var(--font-size-xs, 12px);
}

.drawer-error {
  flex-shrink: 0;
  margin: var(--space-3, 12px) 18px 0;
  border-radius: 8px;
  padding: 10px var(--space-3, 12px);
  background: var(--danger-bg, #fef2f2);
  color: var(--danger, #991b1b);
  font-size: var(--font-size-xs, 12px);
  line-height: 1.45;
}

.drawer-list {
  display: grid;
  flex: 1;
  min-height: 0;
  align-content: start;
  gap: var(--space-3, 12px);
  overflow-x: hidden;
  overflow-y: auto;
  padding: var(--space-4, 16px) 18px 28px;
}

.drawer-empty {
  display: grid;
  flex: 1;
  min-height: 0;
  place-content: center;
  justify-items: center;
  gap: 10px;
  padding: var(--space-6, 32px) 18px;
  color: var(--text-muted, #667085);
  text-align: center;
}

.drawer-empty p {
  margin: 0;
  font-size: var(--font-size-sm, 13px);
  line-height: 1.5;
}

.spin {
  animation: approval-spin 0.8s linear infinite;
}

@keyframes approval-spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 560px) {
  .drawer-filters {
    grid-template-columns: 1fr;
  }
}
</style>

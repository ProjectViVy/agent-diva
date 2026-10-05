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
              class="ui-button ui-button--ghost ui-button--compact drawer-icon-btn"
              :aria-label="t('approvalCenter.refresh')"
              :disabled="loading"
              @click="emit('refresh')"
            >
              <RefreshCw :size="18" :class="{ spin: loading }" />
            </button>
            <button
              ref="closeButton"
              type="button"
              class="ui-button ui-button--ghost ui-button--compact drawer-icon-btn"
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
            <select class="ui-input" v-model="domain">
              <option value="all">{{ t('approvalCenter.all') }}</option>
              <option value="command">Command</option>
              <option value="plan">Plan</option>
            </select>
          </label>
          <label class="drawer-filter-field">
            <span>{{ t('approvalCenter.statusFilter') }}</span>
            <select class="ui-input" v-model="status">
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
            <select class="ui-input" v-model="session">
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
  background: var(--overlay);
}

.approval-center-drawer {
  display: flex;
  width: min(520px, 100vw);
  height: 100%;
  flex-direction: column;
  background: var(--card);
  color: var(--foreground);
  box-shadow: var(--shadow-sm);
}

.drawer-header {
  display: flex;
  flex-shrink: 0;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 18px 18px 14px;
  border-bottom: 1px solid var(--border);
}

.drawer-title-block {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 4px;
}

.drawer-eyebrow {
  color: var(--destructive);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.drawer-header h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 700;
  line-height: 1.25;
  color: var(--foreground);
}

.drawer-header-actions {
  display: flex;
  flex-shrink: 0;
  gap: 8px;
}

.drawer-icon-btn {
  display: grid;
  width: 40px;
  height: 40px;
  place-items: center;
  transition: background 0.15s ease, border-color 0.15s ease;
}

.drawer-filters {
  display: grid;
  flex-shrink: 0;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
  padding: 12px 18px;
  border-bottom: 1px solid var(--border);
}

.drawer-filter-field {
  display: grid;
  gap: 5px;
  min-width: 0;
  color: var(--muted-foreground);
  font-size: 11px;
  font-weight: 600;
}

.drawer-filter-field select {
  width: 100%;
  min-width: 0;
  min-height: 38px;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 0 10px;
  background: var(--card);
  color: var(--foreground);
  font-size: 12px;
}

.drawer-error {
  flex-shrink: 0;
  margin: 12px 18px 0;
  border-radius: 8px;
  padding: 10px 12px;
  background: var(--destructive-soft);
  color: var(--destructive);
  font-size: 12px;
  line-height: 1.45;
}

.drawer-list {
  display: grid;
  flex: 1;
  min-height: 0;
  align-content: start;
  gap: 12px;
  overflow-x: hidden;
  overflow-y: auto;
  padding: 16px 18px 28px;
}

.drawer-empty {
  display: grid;
  flex: 1;
  min-height: 0;
  place-content: center;
  justify-items: center;
  gap: 10px;
  padding: 32px 18px;
  color: var(--muted-foreground);
  text-align: center;
}

.drawer-empty p {
  margin: 0;
  font-size: 13px;
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

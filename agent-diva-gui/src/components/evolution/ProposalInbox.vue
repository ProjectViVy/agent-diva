<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  Archive,
  Check,
  Clock3,
  Eye,
  EyeOff,
  Filter,
  Inbox,
  Pencil,
  RefreshCw,
  Search,
  X,
} from 'lucide-vue-next';
import type {
  EvolutionProposal,
  LaputaSectionName,
  ProposalState,
  ProposalType,
  RiskLevel,
} from '../../api/desktop';

type BatchAction = 'approve' | 'reject' | 'read' | 'unread' | 'defer';

const props = defineProps<{
  proposals: EvolutionProposal[];
  selectedProposalId: string | null;
  loading: boolean;
  loadError: string | null;
  readIds: string[];
  deferredIds: string[];
  busyAction: string | null;
}>();

const emit = defineEmits<{
  (event: 'select', id: string | null): void;
  (event: 'open', id: string): void;
  (event: 'approve', ids: string[]): void;
  (event: 'reject', ids: string[]): void;
  (event: 'edit', id: string): void;
  (event: 'defer', ids: string[]): void;
  (event: 'mark-read', ids: string[]): void;
  (event: 'mark-unread', ids: string[]): void;
  (event: 'retry'): void;
}>();

const { t } = useI18n();

const statusFilter = ref<'all' | ProposalState>('all');
const typeFilter = ref<'all' | ProposalType>('all');
const riskFilter = ref<'all' | RiskLevel>('all');
const sourceFilter = ref('all');
const targetFilter = ref<'all' | LaputaSectionName>('all');
const unreadOnly = ref(false);
const searchText = ref('');
const checkedIds = ref<string[]>([]);
const searchInput = ref<HTMLInputElement | null>(null);

const terminalStates = new Set<ProposalState>(['approved', 'rejected', 'applied', 'reverted', 'superseded']);
const approveStates = new Set<ProposalState>(['pending_review', 'edited']);
const rejectStates = new Set<ProposalState>(['pending_review', 'edited', 'needs_attention', 'run_failed']);

const readableSource = (proposal: EvolutionProposal) => proposal.source_run_id || proposal.created_by;
const isRead = (id: string) => props.readIds.includes(id);
const isDeferred = (id: string) => props.deferredIds.includes(id);

const statusOptions = computed(() => uniqueOptions(props.proposals.map((proposal) => proposal.state)));
const typeOptions = computed(() => uniqueOptions(props.proposals.map((proposal) => proposal.proposal_type)));
const riskOptions = computed(() => uniqueOptions(props.proposals.map((proposal) => proposal.risk_level)));
const sourceOptions = computed(() => uniqueOptions(props.proposals.map(readableSource)));
const targetOptions = computed(() => uniqueOptions(props.proposals.map((proposal) => proposal.target_section)));

const filteredProposals = computed(() => {
  const query = searchText.value.trim().toLowerCase();
  return props.proposals.filter((proposal) => {
    if (statusFilter.value !== 'all' && proposal.state !== statusFilter.value) return false;
    if (typeFilter.value !== 'all' && proposal.proposal_type !== typeFilter.value) return false;
    if (riskFilter.value !== 'all' && proposal.risk_level !== riskFilter.value) return false;
    if (sourceFilter.value !== 'all' && readableSource(proposal) !== sourceFilter.value) return false;
    if (targetFilter.value !== 'all' && proposal.target_section !== targetFilter.value) return false;
    if (unreadOnly.value && isRead(proposal.id)) return false;
    if (!query) return true;

    const haystack = [
      proposal.id,
      proposal.proposal_type,
      proposal.state,
      proposal.risk_level,
      proposal.target_section,
      readableSource(proposal),
      proposal.proposed_patch,
    ]
      .join(' ')
      .toLowerCase();
    return haystack.includes(query);
  });
});

const checkedProposals = computed(() =>
  filteredProposals.value.filter((proposal) => checkedIds.value.includes(proposal.id)),
);

const selectedIndex = computed(() =>
  filteredProposals.value.findIndex((proposal) => proposal.id === props.selectedProposalId),
);

const allFilteredChecked = computed(
  () =>
    filteredProposals.value.length > 0 &&
    filteredProposals.value.every((proposal) => checkedIds.value.includes(proposal.id)),
);

const selectedActionIds = computed(() => {
  if (checkedIds.value.length > 0) return checkedProposals.value.map((proposal) => proposal.id);
  return props.selectedProposalId ? [props.selectedProposalId] : [];
});

const selectedActionProposals = computed(() =>
  selectedActionIds.value
    .map((id) => props.proposals.find((proposal) => proposal.id === id))
    .filter((proposal): proposal is EvolutionProposal => Boolean(proposal)),
);

const canApproveSelection = computed(
  () =>
    selectedActionProposals.value.length > 0 &&
    selectedActionProposals.value.every((proposal) => approveStates.has(proposal.state)),
);

const canRejectSelection = computed(
  () =>
    selectedActionProposals.value.length > 0 &&
    selectedActionProposals.value.every((proposal) => rejectStates.has(proposal.state)),
);

const canDeferSelection = computed(
  () =>
    selectedActionProposals.value.length > 0 &&
    selectedActionProposals.value.every((proposal) => !terminalStates.has(proposal.state)),
);

const canMarkReadSelection = computed(
  () => selectedActionIds.value.length > 0 && selectedActionIds.value.some((id) => !isRead(id)),
);

const canMarkUnreadSelection = computed(
  () => selectedActionIds.value.length > 0 && selectedActionIds.value.some((id) => isRead(id)),
);

function uniqueOptions<T extends string>(values: T[]) {
  return Array.from(new Set(values)).sort((a, b) => a.localeCompare(b));
}

function ageLabel(createdAt: string) {
  const created = Date.parse(createdAt);
  if (Number.isNaN(created)) return createdAt;
  const minutes = Math.max(0, Math.floor((Date.now() - created) / 60000));
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `${hours}h`;
  return `${Math.floor(hours / 24)}d`;
}

function blockedReason(proposal: EvolutionProposal) {
  if (proposal.state === 'needs_attention') return t('evolution.inbox.needsAttentionReason');
  if (proposal.state === 'run_failed') return t('evolution.inbox.runFailedReason');
  return '';
}

function toggleAllFiltered() {
  if (allFilteredChecked.value) {
    const filteredIds = new Set(filteredProposals.value.map((proposal) => proposal.id));
    checkedIds.value = checkedIds.value.filter((id) => !filteredIds.has(id));
  } else {
    checkedIds.value = Array.from(
      new Set([...checkedIds.value, ...filteredProposals.value.map((proposal) => proposal.id)]),
    );
  }
}

function toggleChecked(id: string) {
  checkedIds.value = checkedIds.value.includes(id)
    ? checkedIds.value.filter((checkedId) => checkedId !== id)
    : [...checkedIds.value, id];
}

function moveSelection(delta: number) {
  if (filteredProposals.value.length === 0) return;
  const current = selectedIndex.value >= 0 ? selectedIndex.value : 0;
  const nextIndex = Math.min(Math.max(current + delta, 0), filteredProposals.value.length - 1);
  emit('select', filteredProposals.value[nextIndex].id);
}

function runBatch(action: BatchAction) {
  const ids = selectedActionIds.value;
  if (ids.length === 0) return;
  if (action === 'approve' && canApproveSelection.value) emit('approve', ids);
  if (action === 'reject' && canRejectSelection.value) emit('reject', ids);
  if (action === 'read' && canMarkReadSelection.value) emit('mark-read', ids);
  if (action === 'unread' && canMarkUnreadSelection.value) emit('mark-unread', ids);
  if (action === 'defer' && canDeferSelection.value) emit('defer', ids);
}

function isTextTarget(target: EventTarget | null) {
  const element = target as HTMLElement | null;
  if (!element) return false;
  const tag = typeof element.tagName === 'string' ? element.tagName.toLowerCase() : '';
  return tag === 'input' || tag === 'textarea' || tag === 'select' || element.isContentEditable;
}

async function focusSearch() {
  await nextTick();
  searchInput.value?.focus();
  searchInput.value?.select();
}

function handleKeydown(event: KeyboardEvent) {
  if (isTextTarget(event.target)) {
    if (event.key === 'Escape') {
      (event.target as HTMLElement).blur();
      event.preventDefault();
    }
    return;
  }

  const key = event.key.toLowerCase();
  if (key === '/') {
    event.preventDefault();
    void focusSearch();
  } else if (key === 'j') {
    event.preventDefault();
    moveSelection(1);
  } else if (key === 'k') {
    event.preventDefault();
    moveSelection(-1);
  } else if (key === 'enter' && props.selectedProposalId) {
    event.preventDefault();
    emit('open', props.selectedProposalId);
  } else if (key === 'a') {
    event.preventDefault();
    runBatch('approve');
  } else if (key === 'e' && props.selectedProposalId) {
    event.preventDefault();
    emit('edit', props.selectedProposalId);
  } else if (key === 'r') {
    event.preventDefault();
    runBatch('reject');
  } else if (key === 'd') {
    event.preventDefault();
    runBatch('defer');
  } else if (key === 'escape') {
    event.preventDefault();
    checkedIds.value = [];
  }
}

watch(filteredProposals, (next) => {
  const validIds = new Set(next.map((proposal) => proposal.id));
  checkedIds.value = checkedIds.value.filter((id) => validIds.has(id));
  if (next.length > 0 && (!props.selectedProposalId || !validIds.has(props.selectedProposalId))) {
    emit('select', next[0].id);
  }
  if (next.length === 0 && props.selectedProposalId) {
    emit('select', null);
  }
});

onMounted(() => {
  window.addEventListener('keydown', handleKeydown);
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown);
});
</script>

<template>
  <aside class="proposal-inbox">
    <div class="proposal-inbox__header">
      <div>
        <h2>{{ t('evolution.inbox.title') }}</h2>
        <p>{{ t('evolution.inbox.count', { count: filteredProposals.length }) }}</p>
      </div>
      <span class="proposal-inbox__count">{{ proposals.length }}</span>
    </div>

    <section class="proposal-inbox__filters" aria-label="Proposal filters">
      <label class="proposal-inbox__search">
        <Search :size="14" />
        <input
          ref="searchInput"
          v-model="searchText"
          data-testid="proposal-search"
          type="search"
          :placeholder="t('evolution.inbox.search')"
        />
      </label>
      <div class="proposal-inbox__filter-grid">
        <label>
          <span>{{ t('evolution.inbox.status') }}</span>
          <select v-model="statusFilter" data-testid="proposal-status-filter">
            <option value="all">{{ t('evolution.inbox.all') }}</option>
            <option v-for="status in statusOptions" :key="status" :value="status">{{ status }}</option>
          </select>
        </label>
        <label>
          <span>{{ t('evolution.inbox.type') }}</span>
          <select v-model="typeFilter" data-testid="proposal-type-filter">
            <option value="all">{{ t('evolution.inbox.all') }}</option>
            <option v-for="type in typeOptions" :key="type" :value="type">{{ type }}</option>
          </select>
        </label>
        <label>
          <span>{{ t('evolution.inbox.risk') }}</span>
          <select v-model="riskFilter" data-testid="proposal-risk-filter">
            <option value="all">{{ t('evolution.inbox.all') }}</option>
            <option v-for="risk in riskOptions" :key="risk" :value="risk">{{ risk }}</option>
          </select>
        </label>
        <label>
          <span>{{ t('evolution.inbox.source') }}</span>
          <select v-model="sourceFilter" data-testid="proposal-source-filter">
            <option value="all">{{ t('evolution.inbox.all') }}</option>
            <option v-for="source in sourceOptions" :key="source" :value="source">{{ source }}</option>
          </select>
        </label>
        <label>
          <span>{{ t('evolution.inbox.target') }}</span>
          <select v-model="targetFilter" data-testid="proposal-target-filter">
            <option value="all">{{ t('evolution.inbox.all') }}</option>
            <option v-for="target in targetOptions" :key="target" :value="target">{{ target }}</option>
          </select>
        </label>
        <label class="proposal-inbox__toggle">
          <input v-model="unreadOnly" data-testid="proposal-unread-filter" type="checkbox" />
          <span>{{ t('evolution.inbox.unreadOnly') }}</span>
        </label>
      </div>
    </section>

    <section class="proposal-inbox__batch" aria-label="Batch proposal actions">
      <button type="button" :disabled="filteredProposals.length === 0" @click="toggleAllFiltered">
        <Filter :size="14" />
        <span>{{ allFilteredChecked ? t('evolution.inbox.clearSelection') : t('evolution.inbox.selectVisible') }}</span>
      </button>
      <button
        type="button"
        data-testid="batch-approve"
        :disabled="!canApproveSelection || busyAction !== null"
        @click="runBatch('approve')"
      >
        <Check :size="14" />
        <span>{{ t('evolution.actions.approveOnly') }}</span>
      </button>
      <button
        type="button"
        data-testid="batch-reject"
        :disabled="!canRejectSelection || busyAction !== null"
        @click="runBatch('reject')"
      >
        <X :size="14" />
        <span>{{ t('evolution.actions.reject') }}</span>
      </button>
      <button type="button" :disabled="!canMarkReadSelection" @click="runBatch('read')">
        <Eye :size="14" />
        <span>{{ t('evolution.inbox.markRead') }}</span>
      </button>
      <button type="button" :disabled="!canMarkUnreadSelection" @click="runBatch('unread')">
        <EyeOff :size="14" />
        <span>{{ t('evolution.inbox.markUnread') }}</span>
      </button>
      <button
        type="button"
        data-testid="batch-defer"
        :disabled="!canDeferSelection"
        @click="runBatch('defer')"
      >
        <Clock3 :size="14" />
        <span>{{ t('evolution.actions.defer') }}</span>
      </button>
    </section>

    <div v-if="loading" class="proposal-inbox__state" role="status">
      <Inbox :size="24" />
      <strong>{{ t('evolution.inbox.loadingTitle') }}</strong>
      <span>{{ t('evolution.inbox.loadingDesc') }}</span>
    </div>

    <div v-else-if="loadError" class="proposal-inbox__state proposal-inbox__state--error" role="status">
      <Archive :size="24" />
      <strong>{{ t('evolution.errorTitle') }}</strong>
      <span>{{ loadError }}</span>
      <button type="button" @click="emit('retry')">
        <RefreshCw :size="14" />
        <span>{{ t('evolution.retry') }}</span>
      </button>
    </div>

    <div v-else-if="proposals.length === 0" class="proposal-inbox__state">
      <Archive :size="24" />
      <strong>{{ t('evolution.inbox.emptyTitle') }}</strong>
      <span>{{ t('evolution.inbox.emptyDesc') }}</span>
    </div>

    <div v-else-if="filteredProposals.length === 0" class="proposal-inbox__state">
      <Search :size="24" />
      <strong>{{ t('evolution.inbox.noMatchesTitle') }}</strong>
      <span>{{ t('evolution.inbox.noMatchesDesc') }}</span>
    </div>

    <div v-else class="proposal-inbox__list" data-testid="proposal-list">
      <article
        v-for="proposal in filteredProposals"
        :key="proposal.id"
        class="proposal-inbox__row"
        :class="{
          active: selectedProposalId === proposal.id,
          unread: !isRead(proposal.id),
          deferred: isDeferred(proposal.id),
          high: proposal.risk_level === 'high' || proposal.risk_level === 'critical',
          blocked: Boolean(blockedReason(proposal)),
        }"
        :data-testid="`proposal-row-${proposal.id}`"
      >
        <input
          class="proposal-inbox__row-check"
          type="checkbox"
          :checked="checkedIds.includes(proposal.id)"
          :aria-label="`Select ${proposal.id}`"
          @change="toggleChecked(proposal.id)"
        />
        <button class="proposal-inbox__row-button" type="button" @click="emit('select', proposal.id)">
          <span class="proposal-inbox__row-top">
            <strong>{{ proposal.proposal_type }}</strong>
            <span class="proposal-inbox__state-pill">{{ proposal.state }}</span>
            <span class="proposal-inbox__risk" :data-risk="proposal.risk_level">{{ proposal.risk_level }}</span>
          </span>
          <span class="proposal-inbox__row-meta">
            <span>{{ proposal.target_section }}</span>
            <span>{{ readableSource(proposal) }}</span>
            <span>{{ t('evolution.inbox.evidenceCount', { count: proposal.evidence_refs.length }) }}</span>
            <span>{{ ageLabel(proposal.created_at) }}</span>
          </span>
          <span v-if="blockedReason(proposal)" class="proposal-inbox__blocked">
            {{ blockedReason(proposal) }}
          </span>
        </button>
        <button
          class="proposal-inbox__edit"
          type="button"
          :aria-label="`Edit ${proposal.id}`"
          @click="emit('edit', proposal.id)"
        >
          <Pencil :size="14" />
        </button>
      </article>
    </div>
  </aside>
</template>

<style scoped>
.proposal-inbox {
  display: flex;
  min-width: 0;
  overflow: hidden;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  background: #ffffff;
  flex-direction: column;
}

.proposal-inbox__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
  border-bottom: 1px solid #eef2f7;
  padding: 14px;
}

.proposal-inbox__header h2 {
  margin: 0;
  overflow-wrap: anywhere;
  font-size: 14px;
  font-weight: 700;
}

.proposal-inbox__header p {
  margin: 4px 0 0;
  overflow-wrap: anywhere;
  color: #6b7280;
  font-size: 12px;
}

.proposal-inbox__count {
  display: inline-flex;
  min-width: 28px;
  height: 24px;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  background: #f3f4f6;
  color: #374151;
  font-size: 12px;
  font-weight: 700;
}

.proposal-inbox__filters,
.proposal-inbox__batch {
  display: grid;
  gap: 10px;
  border-bottom: 1px solid #eef2f7;
  padding: 12px;
}

.proposal-inbox__search {
  display: flex;
  min-height: 34px;
  align-items: center;
  gap: 8px;
  border: 1px solid #d1d5db;
  border-radius: 7px;
  background: #ffffff;
  padding: 0 10px;
  color: #6b7280;
}

.proposal-inbox__search input {
  min-width: 0;
  flex: 1;
  border: 0;
  background: transparent;
  color: #111827;
  font-size: 12px;
  outline: 0;
}

.proposal-inbox__filter-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.proposal-inbox__filter-grid label {
  display: grid;
  gap: 4px;
  min-width: 0;
}

.proposal-inbox__filter-grid span {
  color: #6b7280;
  font-size: 11px;
  font-weight: 600;
}

.proposal-inbox__filter-grid select {
  min-width: 0;
  height: 32px;
  border: 1px solid #d1d5db;
  border-radius: 7px;
  background: #ffffff;
  padding: 0 8px;
  color: #111827;
  font-size: 12px;
}

.proposal-inbox__filter-grid .proposal-inbox__toggle {
  display: flex;
  min-height: 32px;
  align-items: center;
  gap: 8px;
}

.proposal-inbox__batch {
  display: flex;
  flex-wrap: wrap;
}

.proposal-inbox__batch button,
.proposal-inbox__state button,
.proposal-inbox__edit {
  display: inline-flex;
  min-height: 30px;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border: 1px solid #d1d5db;
  border-radius: 7px;
  background: #ffffff;
  color: #374151;
  font-size: 11px;
  font-weight: 700;
}

.proposal-inbox__batch button {
  padding: 0 9px;
}

.proposal-inbox__batch button:disabled,
.proposal-inbox__state button:disabled,
.proposal-inbox__edit:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}

.proposal-inbox__state {
  display: flex;
  min-height: 220px;
  flex: 1;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 20px;
  text-align: center;
  color: #6b7280;
}

.proposal-inbox__state strong {
  max-width: 100%;
  overflow-wrap: anywhere;
  color: #111827;
  font-size: 14px;
}

.proposal-inbox__state span {
  max-width: 520px;
  overflow-wrap: anywhere;
  font-size: 12px;
  line-height: 1.5;
}

.proposal-inbox__state--error {
  color: #991b1b;
}

.proposal-inbox__list {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  gap: 10px;
  overflow: auto;
  padding: 12px;
}

.proposal-inbox__row {
  position: relative;
  display: grid;
  min-height: 82px;
  grid-template-columns: 20px minmax(0, 1fr) 30px;
  align-items: center;
  gap: 8px;
  border: 1px solid #e5e7eb;
  border-radius: 7px;
  background: #ffffff;
  padding: 10px;
}

.proposal-inbox__row.high::before {
  position: absolute;
  inset: 0 auto 0 0;
  width: 4px;
  border-radius: 7px 0 0 7px;
  background: #dc2626;
  content: "";
}

.proposal-inbox__row:hover,
.proposal-inbox__row.active {
  border-color: #bfdbfe;
  background: #f8fafc;
}

.proposal-inbox__row.unread .proposal-inbox__row-button strong::after {
  display: inline-block;
  width: 7px;
  height: 7px;
  margin-left: 7px;
  border-radius: 999px;
  background: #2563eb;
  content: "";
}

.proposal-inbox__row.deferred {
  background: #f9fafb;
}

.proposal-inbox__row-check {
  width: 14px;
  height: 14px;
}

.proposal-inbox__row-button {
  display: grid;
  min-width: 0;
  gap: 6px;
  border: 0;
  background: transparent;
  padding: 0;
  text-align: left;
}

.proposal-inbox__row-top,
.proposal-inbox__row-meta {
  display: flex;
  min-width: 0;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}

.proposal-inbox__row-top strong {
  overflow-wrap: anywhere;
  color: #111827;
  font-size: 13px;
}

.proposal-inbox__state-pill,
.proposal-inbox__risk,
.proposal-inbox__row-meta span {
  display: inline-flex;
  min-height: 22px;
  align-items: center;
  border-radius: 6px;
  background: #f3f4f6;
  padding: 0 7px;
  color: #4b5563;
  font-size: 11px;
  font-weight: 700;
}

.proposal-inbox__risk[data-risk="high"],
.proposal-inbox__risk[data-risk="critical"] {
  background: #fef2f2;
  color: #991b1b;
}

.proposal-inbox__blocked {
  display: block;
  min-width: 0;
  overflow: hidden;
  color: #991b1b;
  font-size: 11px;
  line-height: 1.4;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.proposal-inbox__edit {
  width: 30px;
  padding: 0;
}

@media (max-width: 900px) {
  .proposal-inbox__filter-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>

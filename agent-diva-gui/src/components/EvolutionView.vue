<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  AlertTriangle,
  ClipboardList,
  FileClock,
  GitBranch,
  History,
  RefreshCw,
  ShieldCheck,
} from 'lucide-vue-next';
import {
  applyLaputaProposal,
  editLaputaProposal,
  getLaputaSection,
  getSelfEvolutionConfig,
  listAutoDreamRunRecords,
  listLaputaChangelog,
  listLaputaProposals,
  pollLaputaEvents,
  rollbackLaputaChangelog,
  transitionLaputaProposal,
} from '../api/desktop';
import type {
  ChangelogRecord,
  AutoDreamRunRecord,
  EvolutionProposal,
  LaputaEvent,
  LaputaSection,
  SelfEvolutionConfig,
} from '../api/desktop';
import ProposalDetail from './evolution/ProposalDetail.vue';
import ProposalInbox from './evolution/ProposalInbox.vue';
import { appConfirm } from '../utils/appDialog';
import { showAppToast } from '../utils/appToast';

type EvolutionTab = 'inbox' | 'runs' | 'audit' | 'policy';
type CountTone = 'none' | 'accent' | 'warning' | 'danger';

const props = withDefaults(
  defineProps<{
    initialTab?: EvolutionTab;
    initialProposalId?: string | null;
    initialSourceRunId?: string | null;
    requestKey?: string | null;
  }>(),
  {
    initialTab: 'inbox',
    initialProposalId: null,
    initialSourceRunId: null,
    requestKey: null,
  },
);

interface EvolutionCountPayload {
  total: number;
  tone: CountTone;
  tooltip: string;
}

const emit = defineEmits<{
  (event: 'count-change', payload: EvolutionCountPayload): void;
  (event: 'open-settings', view: 'self-evolution'): void;
}>();

const { t } = useI18n();

const tabs = [
  { key: 'inbox', labelKey: 'evolution.tabs.inbox', icon: ClipboardList },
  { key: 'runs', labelKey: 'evolution.tabs.runs', icon: FileClock },
  { key: 'audit', labelKey: 'evolution.tabs.audit', icon: History },
  { key: 'policy', labelKey: 'evolution.tabs.policy', icon: ShieldCheck },
] as const;

const activeTab = ref<EvolutionTab>(props.initialTab);
const proposals = ref<EvolutionProposal[]>([]);
const proposalEvents = ref<LaputaEvent[]>([]);
const changelogEvents = ref<LaputaEvent[]>([]);
const errorEvents = ref<LaputaEvent[]>([]);
const selectedProposalId = ref<string | null>(null);
const selectedSection = ref<LaputaSection | null>(null);
const selectedChangelog = ref<ChangelogRecord | null>(null);
const detailLoading = ref(false);
const loading = ref(false);
const loadError = ref<string | null>(null);
const detailError = ref<string | null>(null);
const actionError = ref<string | null>(null);
const busyAction = ref<string | null>(null);
const evidenceOpen = ref(false);
const activeSourceRunId = ref<string | null>(props.initialSourceRunId ?? null);
const pendingDeepLinkProposalId = ref<string | null>(null);
const readProposalIds = ref<string[]>([]);
const deferredProposalIds = ref<string[]>([]);
const auditRecords = ref<ChangelogRecord[]>([]);
const auditLoading = ref(false);
const auditError = ref<string | null>(null);
const runs = ref<AutoDreamRunRecord[]>([]);
const runsLoading = ref(false);
const runsError = ref<string | null>(null);
const runsLoaded = ref(false);
const policyConfig = ref<SelfEvolutionConfig | null>(null);
const policyLoading = ref(false);
const policyError = ref<string | null>(null);
const policyLoaded = ref(false);

const REQUIRED_POLICY_COPY =
  'Durable personality, memory, SOP, skill, and policy changes require review before they are applied.';
const READ_MARKERS_KEY = 'agent-diva:evolution:proposal-read-markers';
const DEFERRED_MARKERS_KEY = 'agent-diva:evolution:proposal-deferred-markers';

const selectedProposal = computed(() =>
  proposals.value.find((proposal) => proposal.id === selectedProposalId.value) ?? null,
);

const pendingProposals = computed(() =>
  proposals.value.filter((proposal) => proposal.state === 'pending_review')
);

const visibleProposals = computed(() => {
  if (!activeSourceRunId.value) return proposals.value;
  return proposals.value.filter((proposal) => proposal.source_run_id === activeSourceRunId.value);
});

const attentionProposals = computed(() =>
  proposals.value.filter(
    (proposal) => proposal.state === 'needs_attention' || proposal.state === 'run_failed',
  )
);

const missingEvidence = computed(() => (selectedProposal.value?.evidence_refs.length ?? 0) === 0);

const rollbackEligible = computed(
  () =>
    Boolean(
      selectedChangelog.value &&
        isRollbackEligible(selectedChangelog.value) &&
        !selectedChangelog.value.reverted &&
        !selectedChangelog.value.stale,
    ),
);

const rollbackReason = computed(() => {
  if (!selectedProposal.value) return null;
  if (!selectedChangelog.value) {
    return t('evolution.actions.rollbackRequiresApplied');
  }
  if (selectedChangelog.value.reverted) {
    return t('evolution.actions.rollbackAlreadyUsed');
  }
  if (selectedChangelog.value.stale) {
    return t('evolution.actions.rollbackStale');
  }
  if (!isRollbackActionEligible(selectedChangelog.value)) {
    return t('evolution.actions.rollbackActionUnsupported');
  }
  return null;
});

const policySummaryRows = computed(() => {
  const config = policyConfig.value;
  const requiredFor = Array.isArray(config?.require_confirmation_for)
    ? config.require_confirmation_for
    : [];
  return [
    {
      label: t('evolution.policy.enabled'),
      value: config ? String(config.enabled) : t('evolution.policy.unavailableValue'),
    },
    {
      label: t('evolution.policy.frequency'),
      value: config?.autodream_frequency ?? t('evolution.policy.unavailableValue'),
    },
    {
      label: t('evolution.policy.sessionThreshold'),
      value: config ? String(config.trigger_threshold_sessions) : t('evolution.policy.unavailableValue'),
    },
    {
      label: t('evolution.policy.messageThreshold'),
      value: config ? String(config.trigger_threshold_messages) : t('evolution.policy.unavailableValue'),
    },
    {
      label: t('evolution.policy.reviewRequiredFor'),
      value:
        requiredFor.length > 0
          ? requiredFor.join(', ')
          : t('evolution.policy.reviewAllDurable'),
    },
  ];
});

const countPayload = computed<EvolutionCountPayload>(() => {
  const dangerCount = attentionProposals.value.length + errorEvents.value.length;
  const pendingCount = pendingProposals.value.length;
  const infoCount = proposalEvents.value.length + changelogEvents.value.length;
  const total = dangerCount + pendingCount + infoCount;

  if (dangerCount > 0) {
    return {
      total,
      tone: 'danger',
      tooltip: t('evolution.badge.danger', { count: dangerCount }),
    };
  }
  if (pendingCount > 0) {
    return {
      total,
      tone: 'warning',
      tooltip: t('evolution.badge.warning', { count: pendingCount }),
    };
  }
  if (infoCount > 0) {
    return {
      total,
      tone: 'accent',
      tooltip: t('evolution.badge.accent', { count: infoCount }),
    };
  }
  return {
    total: 0,
    tone: 'none',
    tooltip: t('evolution.badge.empty'),
  };
});

function emitCount() {
  emit('count-change', countPayload.value);
}

function normalizeError(error: unknown) {
  if (error && typeof error === 'object' && 'message' in error) {
    return String((error as { message: unknown }).message);
  }
  return error instanceof Error ? error.message : String(error);
}

function loadMarkerList(key: string) {
  if (typeof window === 'undefined') return [];
  try {
    const parsed = JSON.parse(window.localStorage.getItem(key) ?? '[]');
    return Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === 'string') : [];
  } catch {
    return [];
  }
}

function persistMarkerList(key: string, ids: string[]) {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(key, JSON.stringify(Array.from(new Set(ids))));
  } catch (error) {
    showAppToast(normalizeError(error), 'error', 3000);
  }
}

function addMarkers(current: string[], ids: string[]) {
  return Array.from(new Set([...current, ...ids]));
}

function removeMarkers(current: string[], ids: string[]) {
  const remove = new Set(ids);
  return current.filter((id) => !remove.has(id));
}

function isRollbackActionEligible(record: ChangelogRecord) {
  return record.action === 'apply';
}

function isRollbackEligible(record: ChangelogRecord) {
  return isRollbackActionEligible(record) && !record.reverted && !record.stale;
}

function rollbackAvailabilityLabel(record: ChangelogRecord) {
  if (isRollbackEligible(record)) {
    return t('evolution.audit.rollbackAvailable');
  }
  if (record.reverted) {
    return t('evolution.audit.rollbackAlreadyUsed');
  }
  if (record.stale) {
    return t('evolution.audit.rollbackStale');
  }
  if (!isRollbackActionEligible(record)) {
    return t('evolution.audit.rollbackActionUnsupported');
  }
  return t('evolution.audit.rollbackUnavailable');
}

function changelogSummary(record: ChangelogRecord) {
  const trimmedDiff = record.diff.trim();
  if (trimmedDiff.length > 0) {
    return trimmedDiff.length > 120 ? `${trimmedDiff.slice(0, 117)}...` : trimmedDiff;
  }
  return `${record.action} ${record.target_section}`;
}

function formatDuration(run: AutoDreamRunRecord) {
  if (!run.completed_at) {
    return t('evolution.runs.durationUnavailable');
  }
  const started = Date.parse(run.started_at);
  const completed = Date.parse(run.completed_at);
  if (!Number.isFinite(started) || !Number.isFinite(completed) || completed < started) {
    return t('evolution.runs.durationUnavailable');
  }
  const seconds = Math.round((completed - started) / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  return `${minutes}m ${remainingSeconds}s`;
}

function runInputSummary(run: AutoDreamRunRecord) {
  if (!run.input_summary) {
    return t('evolution.runs.inputsUnavailable');
  }
  return t('evolution.runs.inputSummary', {
    count: run.input_summary.total_items,
    bytes: run.input_summary.total_bytes,
  });
}

function runOutputSummary(run: AutoDreamRunRecord) {
  return run.summary || t('evolution.runs.outputsUnavailable');
}

async function loadAudit() {
  auditLoading.value = true;
  auditError.value = null;
  try {
    const page = await listLaputaChangelog(1, 25);
    auditRecords.value = page.items;
  } catch (error) {
    auditRecords.value = [];
    auditError.value = normalizeError(error);
  } finally {
    auditLoading.value = false;
  }
}

async function loadRuns() {
  runsLoading.value = true;
  runsError.value = null;
  try {
    runs.value = await listAutoDreamRunRecords();
    runsLoaded.value = true;
  } catch (error) {
    runs.value = [];
    runsError.value = normalizeError(error);
    runsLoaded.value = false;
  } finally {
    runsLoading.value = false;
  }
}

async function loadPolicy() {
  policyLoading.value = true;
  policyError.value = null;
  try {
    policyConfig.value = await getSelfEvolutionConfig();
    policyLoaded.value = true;
  } catch (error) {
    policyConfig.value = null;
    policyError.value = normalizeError(error);
    policyLoaded.value = false;
  } finally {
    policyLoading.value = false;
  }
}

async function ensureActiveTabLoaded(tab: EvolutionTab) {
  if (tab === 'audit') {
    await loadAudit();
  } else if (tab === 'runs' && !runsLoaded.value) {
    await loadRuns();
  } else if (tab === 'policy' && !policyLoaded.value) {
    await loadPolicy();
  }
}

let detailRequestId = 0;

async function loadDetail(id: string) {
  const proposal = proposals.value.find((item) => item.id === id);
  if (!proposal) return;

  const requestId = ++detailRequestId;
  detailLoading.value = true;
  detailError.value = null;
  actionError.value = null;
  evidenceOpen.value = proposal.risk_level === 'high' || proposal.risk_level === 'critical';

  try {
    const [section, changelogPage] = await Promise.all([
      getLaputaSection(proposal.target_section),
      listLaputaChangelog(1, 5, proposal.id),
    ]);
    if (requestId !== detailRequestId) return;
    selectedSection.value = section;
    selectedChangelog.value = changelogPage.items[0] ?? null;
  } catch (error) {
    if (requestId !== detailRequestId) return;
    detailError.value = normalizeError(error);
    selectedSection.value = null;
    selectedChangelog.value = null;
  } finally {
    if (requestId === detailRequestId) {
      detailLoading.value = false;
    }
  }
}

async function refresh() {
  loading.value = true;
  loadError.value = null;

  try {
    const [proposalList, proposalEventList, changelogEventList, errorEventList] =
      await Promise.all([
        listLaputaProposals(),
        pollLaputaEvents('proposals'),
        pollLaputaEvents('changelog'),
        pollLaputaEvents('errors'),
      ]);
    proposals.value = proposalList;
    proposalEvents.value = proposalEventList;
    changelogEvents.value = changelogEventList;
    errorEvents.value = errorEventList;

    const candidateList = activeSourceRunId.value
      ? proposalList.filter((proposal) => proposal.source_run_id === activeSourceRunId.value)
      : proposalList;

    const deepLinkId = pendingDeepLinkProposalId.value ?? props.initialProposalId;

    if (deepLinkId && candidateList.some((proposal) => proposal.id === deepLinkId)) {
      selectedProposalId.value = deepLinkId;
      pendingDeepLinkProposalId.value = null;
    } else if (!selectedProposalId.value && candidateList.length > 0) {
      selectedProposalId.value = candidateList[0].id;
    } else if (
      selectedProposalId.value &&
      !candidateList.some((proposal) => proposal.id === selectedProposalId.value)
    ) {
      selectedProposalId.value = candidateList[0]?.id ?? null;
    }
  } catch (error) {
    loadError.value = normalizeError(error);
    proposals.value = [];
    proposalEvents.value = [];
    changelogEvents.value = [];
    errorEvents.value = [];
  } finally {
    loading.value = false;
    emitCount();
  }

  if (activeTab.value !== 'inbox') {
    await ensureActiveTabLoaded(activeTab.value);
  }
}

function updateLocalProposal(next: EvolutionProposal) {
  proposals.value = proposals.value.map((proposal) =>
    proposal.id === next.id ? next : proposal,
  );
}

function applyDeepLink() {
  activeTab.value = props.initialTab;
  activeSourceRunId.value = props.initialSourceRunId ?? null;
  if (props.initialProposalId) {
    pendingDeepLinkProposalId.value = props.initialProposalId;
  }
}

async function withAction(name: string, run: () => Promise<void>) {
  busyAction.value = name;
  actionError.value = null;
  try {
    await run();
  } catch (error) {
    actionError.value = normalizeError(error);
    showAppToast(actionError.value, 'error', 3600);
  } finally {
    busyAction.value = null;
  }
}

async function transitionProposalIds(ids: string[], state: 'approved' | 'rejected' | 'deferred') {
  if (ids.length === 0) return;
  if (state === 'rejected') {
    const targets = ids
      .map((id) => proposals.value.find((proposal) => proposal.id === id))
      .filter((proposal): proposal is EvolutionProposal => Boolean(proposal))
      .map((proposal) => `${proposal.id} (${proposal.target_section})`)
      .join(', ');
    const confirmed = await appConfirm(t('evolution.confirm.batchReject', { count: ids.length, target: targets }), {
      title: t('evolution.confirm.title'),
    });
    if (!confirmed) return;
  }
  await withAction(`batch-${state}`, async () => {
    const results = await Promise.allSettled(
      ids.map(async (id) => {
        const proposal = proposals.value.find((item) => item.id === id);
        if (!proposal) return null;
        return transitionLaputaProposal(id, { state });
      }),
    );
    const updated = results
      .filter((result): result is PromiseFulfilledResult<EvolutionProposal | null> => result.status === 'fulfilled')
      .map((result) => result.value)
      .filter((proposal): proposal is EvolutionProposal => Boolean(proposal));
    updated.forEach(updateLocalProposal);
    const failed = results.length - updated.length;
    if (failed > 0) {
      actionError.value = t('evolution.actions.batchPartialFailure', {
        total: results.length,
        success: updated.length,
        failed,
      });
      showAppToast(actionError.value, 'error', 3600);
    } else {
      showAppToast(
        t(
          state === 'approved'
            ? 'evolution.actions.approveOnlySuccess'
            : state === 'rejected'
              ? 'evolution.actions.rejectSuccess'
              : 'evolution.actions.deferSuccess',
        ),
        'success',
      );
    }
    if (selectedProposalId.value) {
      await loadDetail(selectedProposalId.value);
    }
    await refresh();
  });
}

function markRead(ids: string[]) {
  readProposalIds.value = addMarkers(readProposalIds.value, ids);
  persistMarkerList(READ_MARKERS_KEY, readProposalIds.value);
}

function markUnread(ids: string[]) {
  readProposalIds.value = removeMarkers(readProposalIds.value, ids);
  persistMarkerList(READ_MARKERS_KEY, readProposalIds.value);
}

async function handleApproveOnly() {
  if (!selectedProposal.value) return;
  if (
    missingEvidence.value &&
    (selectedProposal.value.risk_level === 'high' || selectedProposal.value.risk_level === 'critical')
  ) {
    actionError.value = t('evolution.actions.highRiskMissingEvidence');
    return;
  }
  await withAction('approve-only', async () => {
    const next = await transitionLaputaProposal(selectedProposal.value!.id, {
      state: 'approved',
    });
    updateLocalProposal(next);
    showAppToast(t('evolution.actions.approveOnlySuccess'), 'success');
    await loadDetail(next.id);
  });
}

async function handleApproveAndApply() {
  if (!selectedProposal.value) return;
  if (
    missingEvidence.value &&
    (selectedProposal.value.risk_level === 'high' || selectedProposal.value.risk_level === 'critical')
  ) {
    actionError.value = t('evolution.actions.highRiskMissingEvidence');
    return;
  }
  const target = selectedProposal.value.target_section;
  const confirmed = await appConfirm(t('evolution.confirm.apply', { target }), {
    title: t('evolution.confirm.title'),
  });
  if (!confirmed) return;

  await withAction('approve-apply', async () => {
    if (selectedProposal.value!.state !== 'approved') {
      const approved = await transitionLaputaProposal(selectedProposal.value!.id, {
        state: 'approved',
      });
      updateLocalProposal(approved);
    }
    const result = await applyLaputaProposal(selectedProposal.value!.id, {});
    updateLocalProposal(result.proposal);
    selectedChangelog.value = result.changelog;
    showAppToast(t('evolution.actions.approveApplySuccess'), 'success');
    await loadDetail(result.proposal.id);
    await refresh();
  });
}

async function handleReject() {
  if (!selectedProposal.value) return;
  const target = selectedProposal.value.target_section;
  const confirmed = await appConfirm(t('evolution.confirm.reject', { target }), {
    title: t('evolution.confirm.title'),
  });
  if (!confirmed) return;

  await withAction('reject', async () => {
    const next = await transitionLaputaProposal(selectedProposal.value!.id, {
      state: 'rejected',
    });
    updateLocalProposal(next);
    showAppToast(t('evolution.actions.rejectSuccess'), 'success');
    await loadDetail(next.id);
  });
}

async function handleRollback() {
  if (!selectedProposal.value || !selectedChangelog.value) return;
  const target = selectedProposal.value.target_section;
  const confirmed = await appConfirm(t('evolution.confirm.rollback', { target }), {
    title: t('evolution.confirm.title'),
  });
  if (!confirmed) return;

  await withAction('rollback', async () => {
    await rollbackLaputaChangelog(selectedChangelog.value!.id, {
      reason: `GUI rollback for ${target}`,
      expected_current:
        typeof selectedSection.value?.content === 'string'
          ? selectedSection.value.content
          : JSON.stringify(selectedSection.value?.content ?? null),
    });
    showAppToast(t('evolution.actions.rollbackSuccess'), 'success');
    await refresh();
    if (selectedProposal.value) {
      await loadDetail(selectedProposal.value.id);
    }
  });
}

async function handleEdit() {
  if (!selectedProposal.value) return;
  await withAction('edit', async () => {
    const next = await editLaputaProposal(selectedProposal.value!.id, {
      updated_at: selectedProposal.value!.updated_at,
    });
    updateLocalProposal(next);
    showAppToast(t('evolution.actions.editRouted'), 'success');
    await loadDetail(next.id);
  });
}

async function handleDefer() {
  if (!selectedProposal.value) return;
  await transitionProposalIds([selectedProposal.value.id], 'deferred');
  actionError.value = null;
}

watch(selectedProposalId, async (id) => {
  if (id) {
    await loadDetail(id);
  } else {
    selectedSection.value = null;
    selectedChangelog.value = null;
  }
});

watch(activeTab, async (tab) => {
  await ensureActiveTabLoaded(tab);
});

watch(
  () => [props.initialTab, props.initialProposalId, props.initialSourceRunId, props.requestKey] as const,
  async () => {
    applyDeepLink();
    await refresh();
  }
);

onMounted(async () => {
  readProposalIds.value = loadMarkerList(READ_MARKERS_KEY);
  deferredProposalIds.value = loadMarkerList(DEFERRED_MARKERS_KEY);
  applyDeepLink();
  await refresh();
});
</script>

<template>
  <section class="evolution-view">
    <header class="evolution-header">
      <div class="evolution-title-block">
        <div class="evolution-title-icon">
          <GitBranch :size="18" />
        </div>
        <div class="min-w-0">
          <h1 class="evolution-title">{{ t('evolution.title') }}</h1>
          <p class="evolution-subtitle">{{ t('evolution.subtitle') }}</p>
        </div>
      </div>
      <button class="evolution-refresh" type="button" @click="refresh">
        <RefreshCw :size="15" />
        <span>{{ t('evolution.refresh') }}</span>
      </button>
    </header>

    <div class="evolution-tabs" role="tablist">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        class="evolution-tab"
        :class="{ active: activeTab === tab.key }"
        :data-testid="`evolution-tab-${tab.key}`"
        type="button"
        role="tab"
        :aria-selected="activeTab === tab.key"
        @click="activeTab = tab.key"
      >
        <component :is="tab.icon" :size="15" />
        <span>{{ t(tab.labelKey) }}</span>
      </button>
    </div>

    <div v-if="loadError" class="evolution-error" role="status">
      <AlertTriangle :size="17" />
      <div>
        <strong>{{ t('evolution.errorTitle') }}</strong>
        <p>{{ loadError }}</p>
      </div>
    </div>

    <div v-if="activeTab === 'inbox'" class="evolution-panel">
      <div v-if="activeSourceRunId" class="evolution-source-filter">
        <span>{{ t('evolution.inbox.sourceRunFilter', { runId: activeSourceRunId }) }}</span>
        <button type="button" @click="activeSourceRunId = null; refresh()">
          {{ t('evolution.inbox.clearSourceRunFilter') }}
        </button>
      </div>
      <div class="evolution-inbox-shell evolution-inbox-responsive" data-testid="evolution-inbox-shell">
        <ProposalInbox
          :proposals="visibleProposals"
          :selected-proposal-id="selectedProposalId"
          :loading="loading"
          :load-error="loadError"
          :read-ids="readProposalIds"
          :deferred-ids="deferredProposalIds"
          :busy-action="busyAction"
          @select="selectedProposalId = $event"
          @open="selectedProposalId = $event"
          @approve="transitionProposalIds($event, 'approved')"
          @reject="transitionProposalIds($event, 'rejected')"
          @edit="selectedProposalId = $event; handleEdit()"
          @defer="transitionProposalIds($event, 'deferred')"
          @mark-read="markRead"
          @mark-unread="markUnread"
          @retry="refresh"
        />

        <section class="evolution-detail-pane">
          <div class="evolution-pane-header">
            <div>
              <h2>{{ t('evolution.detail.title') }}</h2>
              <p>{{ t('evolution.detail.desc') }}</p>
            </div>
          </div>
          <div v-if="detailLoading" class="evolution-detail-loading">{{ t('evolution.detail.loading') }}</div>
          <div v-else class="evolution-detail-body">
            <ProposalDetail
              :proposal="selectedProposal"
              :section="selectedSection"
              :changelog="selectedChangelog"
              :busy-action="busyAction"
              :load-error="detailError"
              :action-error="actionError"
              :evidence-open="evidenceOpen"
              :missing-evidence="missingEvidence"
              :rollback-eligible="rollbackEligible"
              :rollback-reason="rollbackReason"
              @toggle-evidence="evidenceOpen = !evidenceOpen"
              @approve-apply="handleApproveAndApply"
              @approve-only="handleApproveOnly"
              @edit="handleEdit"
              @reject="handleReject"
              @defer="handleDefer"
              @rollback="handleRollback"
            />
          </div>
        </section>
      </div>
    </div>

    <div v-else-if="activeTab === 'runs'" class="evolution-panel">
      <section class="evolution-data-panel" data-testid="evolution-runs-panel">
        <div class="evolution-pane-header">
          <div>
            <h2>{{ t('evolution.runs.title') }}</h2>
            <p>{{ t('evolution.runs.desc') }}</p>
          </div>
        </div>

        <div v-if="runsLoading" class="evolution-detail-loading">{{ t('evolution.runs.loading') }}</div>
        <div v-else-if="runsError" class="evolution-empty-state">
          <FileClock :size="24" />
          <strong>{{ t('evolution.runs.unavailableTitle') }}</strong>
          <span>{{ runsError }}</span>
        </div>
        <div v-else-if="runs.length === 0" class="evolution-empty-state">
          <FileClock :size="24" />
          <strong>{{ t('evolution.runs.emptyTitle') }}</strong>
          <span>{{ t('evolution.runs.emptyDesc') }}</span>
        </div>
        <div v-else class="evolution-card-grid">
          <article v-for="run in runs" :key="run.id" class="evolution-record-card">
            <div class="evolution-record-card__header">
              <strong>{{ run.trigger }}</strong>
              <span>{{ run.state }}</span>
            </div>
            <dl class="evolution-record-grid">
              <div>
                <dt>{{ t('evolution.runs.startedAt') }}</dt>
                <dd>{{ run.started_at }}</dd>
              </div>
              <div>
                <dt>{{ t('evolution.runs.completedAt') }}</dt>
                <dd>{{ run.completed_at || t('evolution.runs.inProgress') }}</dd>
              </div>
              <div>
                <dt>{{ t('evolution.runs.duration') }}</dt>
                <dd>{{ formatDuration(run) }}</dd>
              </div>
              <div>
                <dt>{{ t('evolution.runs.proposalCount') }}</dt>
                <dd>{{ Array.isArray(run.proposal_ids) ? run.proposal_ids.length : 0 }}</dd>
              </div>
              <div>
                <dt>{{ t('evolution.runs.inputs') }}</dt>
                <dd>{{ runInputSummary(run) }}</dd>
              </div>
              <div>
                <dt>{{ t('evolution.runs.outputs') }}</dt>
                <dd>{{ runOutputSummary(run) }}</dd>
              </div>
              <div>
                <dt>{{ t('evolution.runs.errors') }}</dt>
                <dd>{{ run.error || t('evolution.runs.noErrors') }}</dd>
              </div>
            </dl>
          </article>
        </div>
      </section>
    </div>

    <div v-else-if="activeTab === 'audit'" class="evolution-panel">
      <section class="evolution-data-panel" data-testid="evolution-audit-panel">
        <div class="evolution-pane-header">
          <div>
            <h2>{{ t('evolution.audit.title') }}</h2>
            <p>{{ t('evolution.audit.desc') }}</p>
          </div>
        </div>

        <div v-if="auditLoading" class="evolution-detail-loading">{{ t('evolution.audit.loading') }}</div>
        <div v-else-if="auditError" class="evolution-error evolution-error-inline" role="status">
          <AlertTriangle :size="17" />
          <div>
            <strong>{{ t('evolution.audit.errorTitle') }}</strong>
            <p>{{ auditError }}</p>
          </div>
        </div>
        <div v-else-if="auditRecords.length === 0" class="evolution-empty-state">
          <History :size="24" />
          <strong>{{ t('evolution.audit.emptyTitle') }}</strong>
          <span>{{ t('evolution.audit.emptyDesc') }}</span>
        </div>
        <div v-else class="evolution-audit-list">
          <article v-for="record in auditRecords" :key="record.id" class="evolution-audit-row">
            <div class="evolution-audit-row__main">
              <div class="evolution-audit-row__title">
                <strong>{{ record.action }}</strong>
                <span>{{ record.target_section }}</span>
              </div>
              <p>{{ changelogSummary(record) }}</p>
            </div>
            <dl class="evolution-audit-meta">
              <div>
                <dt>{{ t('evolution.audit.timestamp') }}</dt>
                <dd>{{ record.created_at }}</dd>
              </div>
              <div>
                <dt>{{ t('evolution.audit.actor') }}</dt>
                <dd>{{ record.applied_by }}</dd>
              </div>
              <div>
                <dt>{{ t('evolution.audit.sourceProposal') }}</dt>
                <dd>{{ record.proposal_id || '-' }}</dd>
              </div>
              <div>
                <dt>{{ t('evolution.audit.rollbackAvailability') }}</dt>
                <dd>{{ rollbackAvailabilityLabel(record) }}</dd>
              </div>
            </dl>
          </article>
        </div>
      </section>
    </div>

    <div v-else-if="activeTab === 'policy'" class="evolution-panel">
      <section class="evolution-data-panel" data-testid="evolution-policy-panel">
        <div class="evolution-pane-header">
          <div>
            <h2>{{ t('evolution.policy.title') }}</h2>
            <p>{{ t('evolution.policy.desc') }}</p>
          </div>
        </div>

        <div v-if="policyLoading" class="evolution-detail-loading">{{ t('evolution.policy.loading') }}</div>
        <div v-else class="evolution-policy-body">
          <div class="evolution-policy-copy">
            <ShieldCheck :size="20" />
            <p>{{ REQUIRED_POLICY_COPY }}</p>
          </div>

          <div v-if="policyError" class="evolution-error evolution-error-inline" role="status">
            <AlertTriangle :size="17" />
            <div>
              <strong>{{ t('evolution.policy.errorTitle') }}</strong>
              <p>{{ policyError }}</p>
            </div>
          </div>

          <dl class="evolution-policy-grid">
            <div v-for="row in policySummaryRows" :key="row.label">
              <dt>{{ row.label }}</dt>
              <dd>{{ row.value }}</dd>
            </div>
          </dl>

          <button class="evolution-settings-link" type="button" @click="emit('open-settings', 'self-evolution')">
            {{ t('evolution.policy.settingsLink') }}
          </button>
        </div>
      </section>
    </div>
  </section>
</template>

<style scoped>
.evolution-view {
  display: flex;
  min-height: 100%;
  min-width: 0;
  flex-direction: column;
  background: var(--panel);
  color: var(--text);
}

.evolution-header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  border-bottom: 1px solid var(--line);
  background: var(--panel-solid);
  padding: 18px 22px;
}

.evolution-title-block {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 12px;
}

.evolution-title-icon {
  display: grid;
  height: 34px;
  width: 34px;
  flex: 0 0 auto;
  place-items: center;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel);
  color: var(--text-muted);
}

.evolution-title {
  margin: 0;
  overflow-wrap: anywhere;
  font-size: 20px;
  font-weight: 700;
  line-height: 1.2;
}

.evolution-subtitle {
  margin: 4px 0 0;
  overflow-wrap: anywhere;
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1.4;
}

.evolution-refresh,
.evolution-tab {
  display: inline-flex;
  min-height: 34px;
  align-items: center;
  justify-content: center;
  gap: 7px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
  color: var(--text);
  font-size: 12px;
  font-weight: 600;
  transition: background 120ms ease, border-color 120ms ease, color 120ms ease;
}

.evolution-refresh {
  padding: 0 12px;
}

.evolution-refresh:hover,
.evolution-tab:hover {
  border-color: var(--text-muted);
  background: var(--panel);
}

.evolution-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  border-bottom: 1px solid var(--line);
  background: var(--panel-solid);
  padding: 10px 22px;
}

.evolution-tab {
  padding: 0 11px;
}

.evolution-tab.active {
  border-color: var(--accent);
  background: var(--accent-bg-light);
  color: var(--accent);
}

.evolution-error {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  border-bottom: 1px solid var(--danger-bg);
  background: var(--danger-bg);
  padding: 12px 22px;
  color: var(--danger);
  font-size: 12px;
}

.evolution-error p {
  margin: 2px 0 0;
  overflow-wrap: anywhere;
}

.evolution-error-inline {
  margin: 12px;
  border: 1px solid var(--danger-bg);
  border-radius: var(--radius-sm);
}

.evolution-panel {
  min-height: 0;
  flex: 1;
  padding: 18px;
}

.evolution-source-filter {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 12px;
  border: 1px solid var(--accent-border);
  border-radius: var(--radius-sm);
  background: var(--accent-bg-light);
  padding: 10px 12px;
  color: var(--accent);
  font-size: 12px;
  font-weight: 700;
}

.evolution-source-filter span {
  min-width: 0;
  overflow-wrap: anywhere;
}

.evolution-source-filter button {
  min-height: 28px;
  border: 1px solid var(--accent-border);
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
  padding: 0 10px;
  color: var(--accent);
  cursor: pointer;
  font-size: 12px;
  font-weight: 700;
}

.evolution-inbox-shell {
  display: grid;
  height: 100%;
  min-height: 480px;
  min-width: 0;
  grid-template-columns: minmax(320px, 0.85fr) minmax(520px, 1.4fr);
  gap: 14px;
}

.evolution-list-pane,
.evolution-detail-pane,
.evolution-data-panel,
.evolution-placeholder-panel {
  min-width: 0;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
}

.evolution-list-pane,
.evolution-detail-pane,
.evolution-data-panel {
  display: flex;
  flex-direction: column;
}

.evolution-pane-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
  border-bottom: 1px solid var(--line);
  padding: 14px;
}

.evolution-pane-header h2 {
  margin: 0;
  overflow-wrap: anywhere;
  font-size: 14px;
  font-weight: 700;
}

.evolution-pane-header p {
  margin: 4px 0 0;
  overflow-wrap: anywhere;
  color: var(--text-muted);
  font-size: 12px;
}

.evolution-count-pill {
  display: inline-flex;
  min-width: 28px;
  height: 24px;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  background: var(--panel);
  color: var(--text-muted);
  font-size: 12px;
  font-weight: 700;
}

.evolution-skeleton-list,
.evolution-proposal-list,
.evolution-detail-body,
.evolution-audit-list,
.evolution-card-grid,
.evolution-policy-body {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  gap: 10px;
  overflow: auto;
  padding: 12px;
}

.evolution-detail-loading {
  padding: 18px;
  color: var(--text-muted);
  font-size: 13px;
}

.evolution-skeleton-row {
  height: 58px;
  border-radius: 7px;
  background: linear-gradient(90deg, var(--panel), var(--line), var(--panel));
}

.evolution-empty-state,
.evolution-placeholder-panel {
  display: flex;
  min-height: 220px;
  flex: 1;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 20px;
  text-align: center;
  color: var(--text-muted);
}

.evolution-empty-state strong,
.evolution-placeholder-panel strong {
  max-width: 100%;
  overflow-wrap: anywhere;
  color: var(--text);
  font-size: 14px;
}

.evolution-empty-state span,
.evolution-placeholder-panel span {
  max-width: 520px;
  overflow-wrap: anywhere;
  font-size: 12px;
  line-height: 1.5;
}

.evolution-proposal-row {
  display: flex;
  width: 100%;
  min-height: 62px;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
  padding: 10px 12px;
  text-align: left;
}

.evolution-proposal-row:hover,
.evolution-proposal-row.active {
  border-color: var(--accent-border);
  background: var(--panel);
}

.evolution-proposal-main,
.evolution-proposal-meta {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 3px;
  overflow-wrap: anywhere;
  font-size: 12px;
}

.evolution-proposal-main strong {
  color: var(--text);
  font-size: 13px;
}

.evolution-proposal-meta {
  flex: 0 0 auto;
  align-items: flex-end;
  color: var(--text-muted);
}

.evolution-record-card,
.evolution-audit-row {
  min-width: 0;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
  padding: 14px;
}

.evolution-record-card__header,
.evolution-audit-row__title {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  color: var(--text);
  font-size: 13px;
}

.evolution-record-card__header span,
.evolution-audit-row__title span {
  border-radius: 999px;
  background: var(--panel);
  padding: 3px 8px;
  color: var(--text-muted);
  font-size: 11px;
  font-weight: 700;
}

.evolution-record-grid,
.evolution-audit-meta,
.evolution-policy-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 10px;
  margin: 12px 0 0;
}

.evolution-record-grid div,
.evolution-audit-meta div,
.evolution-policy-grid div {
  min-width: 0;
}

.evolution-record-grid dt,
.evolution-audit-meta dt,
.evolution-policy-grid dt {
  margin: 0 0 3px;
  color: var(--text-muted);
  font-size: 11px;
  font-weight: 700;
}

.evolution-record-grid dd,
.evolution-audit-meta dd,
.evolution-policy-grid dd {
  margin: 0;
  overflow-wrap: anywhere;
  color: var(--text);
  font-size: 12px;
  line-height: 1.45;
}

.evolution-audit-row__main p {
  margin: 8px 0 0;
  overflow-wrap: anywhere;
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1.45;
}

.evolution-policy-copy {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  border: 1px solid var(--accent-border);
  border-radius: var(--radius-sm);
  background: var(--accent-bg-light);
  padding: 12px;
  color: var(--accent);
}

.evolution-policy-copy p {
  margin: 0;
  overflow-wrap: anywhere;
  color: var(--accent);
  font-size: 13px;
  font-weight: 700;
  line-height: 1.45;
}

.evolution-settings-link {
  display: inline-flex;
  width: fit-content;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
  padding: 8px 11px;
  color: var(--text);
  cursor: pointer;
  font-size: 12px;
  font-weight: 700;
  text-decoration: none;
}

.evolution-settings-link:hover {
  border-color: var(--text-muted);
  background: var(--panel);
}

@media (max-width: 1180px) {
  .evolution-inbox-shell {
    grid-template-columns: minmax(320px, 0.95fr) minmax(420px, 1.1fr);
  }
}

@media (max-width: 900px) {
  .evolution-panel {
    padding: 12px;
  }

  .evolution-inbox-shell {
    min-height: 0;
    grid-template-columns: minmax(0, 1fr);
  }

  .evolution-detail-pane {
    min-height: 300px;
  }
}
</style>

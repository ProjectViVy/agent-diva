<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  AlertTriangle,
  Archive,
  ClipboardList,
  FileClock,
  GitBranch,
  History,
  RefreshCw,
  ShieldCheck,
} from 'lucide-vue-next';
import {
  listLaputaProposals,
  pollLaputaEvents,
} from '../api/desktop';
import type { EvolutionProposal, LaputaEvent } from '../api/desktop';

type EvolutionTab = 'inbox' | 'runs' | 'audit' | 'policy';
type CountTone = 'none' | 'accent' | 'warning' | 'danger';

interface EvolutionCountPayload {
  total: number;
  tone: CountTone;
  tooltip: string;
}

const emit = defineEmits<{
  (event: 'count-change', payload: EvolutionCountPayload): void;
}>();

const { t } = useI18n();

const tabs = [
  { key: 'inbox', labelKey: 'evolution.tabs.inbox', icon: ClipboardList },
  { key: 'runs', labelKey: 'evolution.tabs.runs', icon: FileClock },
  { key: 'audit', labelKey: 'evolution.tabs.audit', icon: History },
  { key: 'policy', labelKey: 'evolution.tabs.policy', icon: ShieldCheck },
] as const;

const activeTab = ref<EvolutionTab>('inbox');
const proposals = ref<EvolutionProposal[]>([]);
const proposalEvents = ref<LaputaEvent[]>([]);
const changelogEvents = ref<LaputaEvent[]>([]);
const errorEvents = ref<LaputaEvent[]>([]);
const loading = ref(false);
const loadError = ref<string | null>(null);

const pendingProposals = computed(() =>
  proposals.value.filter((proposal) => proposal.state === 'pending_review')
);

const attentionProposals = computed(() =>
  proposals.value.filter((proposal) =>
    proposal.state === 'needs_attention' || proposal.state === 'run_failed'
  )
);

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

const emitCount = () => {
  emit('count-change', countPayload.value);
};

const refresh = async () => {
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
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    loadError.value = message;
    proposals.value = [];
    proposalEvents.value = [];
    changelogEvents.value = [];
    errorEvents.value = [];
  } finally {
    loading.value = false;
    emitCount();
  }
};

onMounted(() => {
  refresh();
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
      <div
        class="evolution-inbox-shell evolution-inbox-responsive"
        data-testid="evolution-inbox-shell"
      >
        <aside class="evolution-list-pane">
          <div class="evolution-pane-header">
            <div>
              <h2>{{ t('evolution.inbox.title') }}</h2>
              <p>{{ t('evolution.inbox.count', { count: pendingProposals.length }) }}</p>
            </div>
            <span class="evolution-count-pill">{{ countPayload.total }}</span>
          </div>

          <div v-if="loading" class="evolution-skeleton-list">
            <div v-for="row in 3" :key="row" class="evolution-skeleton-row" />
          </div>

          <div v-else-if="proposals.length === 0" class="evolution-empty-state">
            <Archive :size="24" />
            <strong>{{ t('evolution.inbox.emptyTitle') }}</strong>
            <span>{{ t('evolution.inbox.emptyDesc') }}</span>
          </div>

          <div v-else class="evolution-proposal-list">
            <button
              v-for="proposal in proposals"
              :key="proposal.id"
              class="evolution-proposal-row"
              type="button"
            >
              <span class="evolution-proposal-main">
                <strong>{{ proposal.proposal_type }}</strong>
                <span>{{ proposal.target_section }}</span>
              </span>
              <span class="evolution-proposal-meta">
                {{ proposal.state }} · {{ proposal.risk_level }}
              </span>
            </button>
          </div>
        </aside>

        <section class="evolution-detail-pane">
          <div class="evolution-pane-header">
            <div>
              <h2>{{ t('evolution.detail.title') }}</h2>
              <p>{{ t('evolution.detail.desc') }}</p>
            </div>
          </div>
          <div class="evolution-detail-placeholder">
            <ClipboardList :size="28" />
            <strong>{{ t('evolution.detail.placeholderTitle') }}</strong>
            <span>{{ t('evolution.detail.placeholderDesc') }}</span>
          </div>
        </section>
      </div>
    </div>

    <div v-else class="evolution-panel">
      <div class="evolution-placeholder-panel">
        <component
          :is="tabs.find((tab) => tab.key === activeTab)?.icon"
          :size="30"
        />
        <strong>{{ t(`evolution.placeholders.${activeTab}.title`) }}</strong>
        <span>{{ t(`evolution.placeholders.${activeTab}.desc`) }}</span>
      </div>
    </div>
  </section>
</template>

<style scoped>
.evolution-view {
  display: flex;
  min-height: 100%;
  min-width: 0;
  flex-direction: column;
  background: #f8fafc;
  color: #111827;
}

.evolution-header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  border-bottom: 1px solid #e5e7eb;
  background: #ffffff;
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
  border: 1px solid #d1d5db;
  border-radius: 8px;
  background: #f9fafb;
  color: #4b5563;
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
  color: #6b7280;
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
  border: 1px solid #d1d5db;
  border-radius: 7px;
  background: #ffffff;
  color: #374151;
  font-size: 12px;
  font-weight: 600;
  transition: background 120ms ease, border-color 120ms ease, color 120ms ease;
}

.evolution-refresh {
  padding: 0 12px;
}

.evolution-refresh:hover,
.evolution-tab:hover {
  border-color: #9ca3af;
  background: #f9fafb;
}

.evolution-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  border-bottom: 1px solid #e5e7eb;
  background: #ffffff;
  padding: 10px 22px;
}

.evolution-tab {
  padding: 0 11px;
}

.evolution-tab.active {
  border-color: #2563eb;
  background: #eff6ff;
  color: #1d4ed8;
}

.evolution-error {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  border-bottom: 1px solid #fecaca;
  background: #fff1f2;
  padding: 12px 22px;
  color: #991b1b;
  font-size: 12px;
}

.evolution-error p {
  margin: 2px 0 0;
  overflow-wrap: anywhere;
}

.evolution-panel {
  min-height: 0;
  flex: 1;
  padding: 18px;
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
.evolution-placeholder-panel {
  min-width: 0;
  overflow: hidden;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  background: #ffffff;
}

.evolution-list-pane,
.evolution-detail-pane {
  display: flex;
  flex-direction: column;
}

.evolution-pane-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
  border-bottom: 1px solid #eef2f7;
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
  color: #6b7280;
  font-size: 12px;
}

.evolution-count-pill {
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

.evolution-skeleton-list,
.evolution-proposal-list {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  gap: 10px;
  overflow: auto;
  padding: 12px;
}

.evolution-skeleton-row {
  height: 58px;
  border-radius: 7px;
  background: linear-gradient(90deg, #f3f4f6, #e5e7eb, #f3f4f6);
}

.evolution-empty-state,
.evolution-detail-placeholder,
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
  color: #6b7280;
}

.evolution-empty-state strong,
.evolution-detail-placeholder strong,
.evolution-placeholder-panel strong {
  max-width: 100%;
  overflow-wrap: anywhere;
  color: #111827;
  font-size: 14px;
}

.evolution-empty-state span,
.evolution-detail-placeholder span,
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
  border: 1px solid #e5e7eb;
  border-radius: 7px;
  background: #ffffff;
  padding: 10px 12px;
  text-align: left;
}

.evolution-proposal-row:hover {
  border-color: #bfdbfe;
  background: #f8fafc;
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
  color: #111827;
  font-size: 13px;
}

.evolution-proposal-meta {
  flex: 0 0 auto;
  align-items: flex-end;
  color: #6b7280;
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

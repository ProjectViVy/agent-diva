<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { AlertCircle, ExternalLink, FileDiff, FileSearch, ShieldAlert } from 'lucide-vue-next';
import type { ChangelogRecord, EvolutionProposal, LaputaSection } from '../../api/desktop';
import GovernanceActionBar from './GovernanceActionBar.vue';

const { t } = useI18n();

const props = defineProps<{
  proposal: EvolutionProposal | null;
  section: LaputaSection | null;
  changelog: ChangelogRecord | null;
  busyAction: string | null;
  loadError: string | null;
  actionError: string | null;
  evidenceOpen: boolean;
  missingEvidence: boolean;
  rollbackEligible: boolean;
  rollbackReason: string | null;
}>();

const emit = defineEmits<{
  (event: 'toggle-evidence'): void;
  (event: 'approve-apply'): void;
  (event: 'approve-only'): void;
  (event: 'edit'): void;
  (event: 'reject'): void;
  (event: 'defer'): void;
  (event: 'rollback'): void;
}>();

const targetLabel = computed(() => props.proposal?.target_section ?? 'n/a');

const summary = computed(() => {
  if (!props.proposal) return '';
  const patch = props.proposal.proposed_patch.trim();
  if (!patch) return t('evolution.detail.emptyPatch');
  const firstLine = patch.split('\n').find((line) => line.trim().length > 0) ?? patch;
  return firstLine.slice(0, 180);
});

const currentContent = computed(() => {
  if (!props.section) return '';
  if (typeof props.section.content === 'string') {
    return props.section.content;
  }
  return JSON.stringify(props.section.content, null, 2);
});

const proposedContent = computed(() => props.proposal?.proposed_patch ?? '');

const safetyChecks = computed(() => {
  if (!props.proposal) return [];
  return [
    {
      key: 'evidence',
      ok: !props.missingEvidence,
      label: t('evolution.detail.safetyEvidence'),
    },
    {
      key: 'state',
      ok: props.proposal.state !== 'run_failed' && props.proposal.state !== 'needs_attention',
      label: t('evolution.detail.safetyState'),
    },
    {
      key: 'rollback',
      ok: props.rollbackEligible || !props.changelog,
      label: t('evolution.detail.safetyRollback'),
    },
  ];
});
</script>

<template>
  <div v-if="proposal" class="proposal-detail">
    <header class="proposal-detail__header">
      <div>
        <p class="proposal-detail__eyebrow">{{ targetLabel }}</p>
        <h3 class="proposal-detail__title">{{ t('evolution.detail.proposalTitle', { id: proposal.id }) }}</h3>
      </div>
      <div class="proposal-detail__risk" :data-risk="proposal.risk_level">{{ proposal.risk_level }}</div>
    </header>

    <div v-if="loadError || actionError" class="proposal-detail__error" role="status">
      <AlertCircle :size="16" />
      <span>{{ actionError || loadError }}</span>
    </div>

    <section class="proposal-detail__meta">
      <div><span>{{ t('evolution.detail.status') }}</span><strong>{{ proposal.state }}</strong></div>
      <div><span>{{ t('evolution.detail.source') }}</span><strong>{{ proposal.created_by }}</strong></div>
      <div><span>{{ t('evolution.detail.createdAt') }}</span><strong>{{ proposal.created_at }}</strong></div>
      <div><span>{{ t('evolution.detail.updatedAt') }}</span><strong>{{ proposal.updated_at }}</strong></div>
    </section>

    <section class="proposal-detail__block">
      <h4>{{ t('evolution.detail.summary') }}</h4>
      <p>{{ summary }}</p>
    </section>

    <section class="proposal-detail__block">
      <div class="proposal-detail__block-header">
        <h4>{{ t('evolution.detail.evidence') }}</h4>
        <button class="proposal-detail__link" type="button" @click="emit('toggle-evidence')">
          <FileSearch :size="14" />
          <span>{{ evidenceOpen ? t('evolution.detail.hideEvidence') : t('evolution.detail.showEvidence') }}</span>
        </button>
      </div>
      <div v-if="evidenceOpen" class="proposal-detail__evidence">
        <div v-if="proposal.evidence_refs.length === 0" class="proposal-detail__missing-evidence">
          <ShieldAlert :size="16" />
          <span>{{ t('evolution.detail.noEvidence') }}</span>
        </div>
        <article v-for="evidence in proposal.evidence_refs" :key="evidence.id" class="proposal-detail__evidence-item">
          <div class="proposal-detail__evidence-head">
            <strong>{{ evidence.source }}</strong>
            <span>{{ evidence.created_at }}</span>
          </div>
          <p>{{ evidence.excerpt || t('evolution.detail.evidenceUnavailable') }}</p>
          <a class="proposal-detail__uri" :href="evidence.uri" target="_blank" rel="noreferrer">
            <ExternalLink :size="13" />
            <span>{{ evidence.uri }}</span>
          </a>
        </article>
      </div>
    </section>

    <section class="proposal-detail__block">
      <h4>{{ t('evolution.detail.affectedModules') }}</h4>
      <p>{{ proposal.target_section }} / {{ proposal.proposal_type }}</p>
    </section>

    <section class="proposal-detail__block">
      <h4>{{ t('evolution.detail.safetyChecks') }}</h4>
      <ul class="proposal-detail__checks">
        <li v-for="check in safetyChecks" :key="check.key" :class="{ bad: !check.ok }">{{ check.label }}</li>
      </ul>
    </section>

    <section class="proposal-detail__block">
      <div class="proposal-detail__block-header">
        <h4>{{ t('evolution.detail.diff') }}</h4>
        <FileDiff :size="15" />
      </div>
      <pre class="proposal-detail__code">{{ changelog?.diff || t('evolution.detail.diffFallback') }}</pre>
    </section>

    <section class="proposal-detail__compare">
      <article class="proposal-detail__column">
        <h4>{{ t('evolution.detail.current') }}</h4>
        <pre class="proposal-detail__code">{{ currentContent || t('evolution.detail.currentUnavailable') }}</pre>
      </article>
      <article class="proposal-detail__column">
        <h4>{{ t('evolution.detail.proposed') }}</h4>
        <pre class="proposal-detail__code">{{ proposedContent || t('evolution.detail.emptyPatch') }}</pre>
      </article>
    </section>

    <GovernanceActionBar
      :proposal="proposal"
      :busy-action="busyAction"
      :disable-approval="missingEvidence && (proposal.risk_level === 'high' || proposal.risk_level === 'critical')"
      :disable-rollback="!rollbackEligible"
      :missing-evidence="missingEvidence"
      :rollback-reason="rollbackReason"
      @approve-apply="emit('approve-apply')"
      @approve-only="emit('approve-only')"
      @edit="emit('edit')"
      @reject="emit('reject')"
      @defer="emit('defer')"
      @rollback="emit('rollback')"
    />
  </div>

  <div v-else class="proposal-detail proposal-detail--empty">
    <FileSearch :size="28" />
    <strong>{{ t('evolution.detail.placeholderTitle') }}</strong>
    <span>{{ t('evolution.detail.placeholderDesc') }}</span>
  </div>
</template>

<style scoped>
.proposal-detail {
  display: grid;
  gap: 16px;
}

.proposal-detail--empty {
  place-items: center;
  align-content: center;
  min-height: 360px;
  text-align: center;
  color: var(--text-muted);
}

.proposal-detail__header,
.proposal-detail__block-header,
.proposal-detail__evidence-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.proposal-detail__eyebrow {
  margin: 0 0 4px;
  color: var(--text-muted);
  font-size: 12px;
  text-transform: uppercase;
}

.proposal-detail__title {
  margin: 0;
  font-size: 20px;
  line-height: 1.2;
}

.proposal-detail__risk {
  border-radius: 999px;
  padding: 4px 10px;
  font-size: 12px;
  font-weight: 700;
  text-transform: uppercase;
}

.proposal-detail__risk[data-risk='low'] {
  background: color-mix(in srgb, var(--success) 12%, transparent);
  color: var(--success);
}
.proposal-detail__risk[data-risk='medium'] {
  background: color-mix(in srgb, var(--warning) 15%, transparent);
  color: var(--warning);
}
.proposal-detail__risk[data-risk='high'],
.proposal-detail__risk[data-risk='critical'] {
  background: var(--danger-bg);
  color: var(--danger);
}

.proposal-detail__error {
  display: flex;
  align-items: center;
  gap: 8px;
  border-radius: 8px;
  background: var(--danger-bg);
  color: var(--danger);
  padding: 10px 12px;
}

.proposal-detail__meta {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.proposal-detail__meta div,
.proposal-detail__block {
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--panel);
  padding: 12px;
}

.proposal-detail__meta span {
  display: block;
  color: var(--text-muted);
  font-size: 12px;
  margin-bottom: 4px;
}

.proposal-detail__link,
.proposal-detail__uri {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--accent);
  background: transparent;
  border: 0;
  padding: 0;
  font-size: 13px;
}

.proposal-detail__evidence {
  display: grid;
  gap: 10px;
}

.proposal-detail__missing-evidence {
  display: flex;
  align-items: center;
  gap: 8px;
  border-radius: 8px;
  background: var(--danger-bg);
  color: var(--danger);
  padding: 10px 12px;
}

.proposal-detail__evidence-item {
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 10px 12px;
  background: var(--panel);
}

.proposal-detail__checks {
  margin: 0;
  padding-left: 18px;
}

.proposal-detail__checks .bad {
  color: var(--danger);
}

.proposal-detail__compare {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.proposal-detail__column {
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--panel);
  padding: 12px;
}

.proposal-detail__code {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-size: 12px;
  line-height: 1.55;
  color: var(--text);
  background: var(--panel-solid);
  border-radius: 6px;
  padding: 12px;
  max-height: 260px;
  overflow: auto;
}

@media (max-width: 880px) {
  .proposal-detail__meta,
  .proposal-detail__compare {
    grid-template-columns: 1fr;
  }
}
</style>

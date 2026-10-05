<script setup lang="ts">
/**
 * OBS-07 read-only trajectory consumer. Rows are keyed by stable
 * backend IDs (record.id / request.request_id / run_id) — array indices
 * are never identities. Child/parent references render as refs only:
 * they assert linkage, not authorization or terminal state.
 */
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { TrajectoryView } from '../../state/vivy-trajectory';
import type { TrajectoryRecord, TrajectoryRequest } from '../../api/vivy/contracts';

const props = defineProps<{
  view: TrajectoryView | null;
  error: string | null;
  connectionState: string;
}>();

const { t } = useI18n();

const requestsByRun = computed(() => {
  const map = new Map<string, TrajectoryRequest[]>();
  for (const req of props.view?.requests ?? []) {
    const list = map.get(req.run_id) ?? [];
    list.push(req);
    map.set(req.run_id, list);
  }
  return map;
});

const recordGroups = computed(() => {
  // run-scoped ids look like `run_id:seq:kind`; session-level rows use
  // `rec-N` or `runId:user` — land under the run when the prefix matches
  // a folded activity run, else the unowned bucket (no row is dropped).
  const runs = new Set((props.view?.activity ?? []).map((a) => a.run_id));
  const byRun = new Map<string, TrajectoryRecord[]>();
  const unowned: TrajectoryRecord[] = [];
  for (const rec of props.view?.records ?? []) {
    const prefix = rec.id.split(':')[0] ?? '';
    if (runs.has(prefix)) {
      const list = byRun.get(prefix) ?? [];
      list.push(rec);
      byRun.set(prefix, list);
    } else {
      unowned.push(rec);
    }
  }
  return { byRun, unowned };
});

function usageText(req: TrajectoryRequest): string {
  const ev = req.usage_evidence;
  if (!ev) return t('trajectory.usageMissing');
  const total = ev.total_tokens;
  return `${total} tok${ev.partial ? ' · ' + t('trajectory.partialEvidence') : ''}`;
}

function shortId(id: string): string {
  return id.length > 18 ? `${id.slice(0, 10)}…${id.slice(-6)}` : id;
}
</script>

<template>
  <div class="trajectory-panel">
    <div v-if="error && !view" class="traj-banner traj-banner--error">{{ error }}</div>
    <div v-else-if="!view" class="traj-banner traj-banner--notice">
      {{ t('trajectory.loading') }}
    </div>
    <template v-else>
      <div v-if="view.hasOlderRuns" class="traj-banner traj-banner--notice">
        {{ t('trajectory.hasOlderRuns') }}
      </div>
      <div v-if="view.incompleteRuns.length > 0" class="traj-banner traj-banner--gap">
        {{ t('trajectory.incomplete', { count: view.incompleteRuns.length }) }}
      </div>
      <div v-else-if="view.stale" class="traj-banner traj-banner--notice">
        {{ t('trajectory.stale') }}
      </div>
      <div v-else-if="connectionState === 'gap' || connectionState === 'lost'" class="traj-banner traj-banner--gap">
        {{ t('trajectory.connectionGap') }}
      </div>

      <div v-for="activity in view.activity" :key="activity.run_id" class="run-block">
        <div class="run-header">
          <span class="run-id" :title="activity.run_id">{{ shortId(activity.run_id) }}</span>
          <span class="run-status">{{ activity.status }}</span>
          <span class="activity-chip" :class="`activity-chip--${activity.activity_state}`">
            {{ activity.activity_state }}
          </span>
          <span v-if="activity.wait_kind" class="wait-chip">
            {{ t(`trajectory.wait.${activity.wait_kind}`, activity.wait_kind) }}
          </span>
          <span v-if="activity.workflow_id" class="ref-chip">
            {{ t('trajectory.workflow') }} {{ shortId(activity.workflow_id) }}
          </span>
        </div>
        <div v-if="activity.parent_run_id" class="run-refs">
          {{ t('trajectory.parentRun') }}:
          <span class="ref-chip" :title="activity.parent_run_id">{{ shortId(activity.parent_run_id) }}</span>
        </div>
        <div v-if="activity.child_run_ids && activity.child_run_ids.length > 0" class="run-refs">
          {{ t('trajectory.childRuns') }}:
          <span v-for="cid in activity.child_run_ids" :key="cid" class="ref-chip" :title="cid">
            {{ shortId(cid) }}
          </span>
        </div>

        <div class="request-table" v-if="requestsByRun.get(activity.run_id)?.length">
          <div class="request-row request-row--head">
            <span>{{ t('trajectory.request') }}</span>
            <span>{{ t('trajectory.callStatus') }}</span>
            <span>{{ t('trajectory.usage') }}</span>
          </div>
          <div
            v-for="req in requestsByRun.get(activity.run_id)"
            :key="req.request_id"
            class="request-row"
            :title="req.request_id"
          >
            <span class="req-id">{{ shortId(req.request_id) }}</span>
            <span class="call-chip" :class="`call-chip--${req.call_status}`">{{ req.call_status }}</span>
            <span class="req-usage">{{ usageText(req) }}</span>
          </div>
        </div>

        <div class="record-list">
          <div
            v-for="rec in recordGroups.byRun.get(activity.run_id)"
            :key="rec.id"
            class="record-row"
            :class="{ 'record-row--error': rec.is_error }"
          >
            <span class="record-kind">{{ rec.kind }}</span>
            <span class="record-group">{{ rec.group }}</span>
            <span class="record-text">{{ rec.text || rec.result }}</span>
          </div>
        </div>
      </div>

      <div v-if="recordGroups.unowned.length > 0" class="run-block">
        <div class="run-header">
          <span class="run-id">{{ view.sessionId }}</span>
          <span class="run-status">{{ t('trajectory.sessionRecords') }}</span>
        </div>
        <div class="record-list">
          <div
            v-for="rec in recordGroups.unowned"
            :key="rec.id"
            class="record-row"
            :class="{ 'record-row--error': rec.is_error }"
          >
            <span class="record-kind">{{ rec.kind }}</span>
            <span class="record-group">{{ rec.group }}</span>
            <span class="record-text">{{ rec.text || rec.result }}</span>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.trajectory-panel {
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.traj-banner {
  padding: 0.625rem 1rem;
  border-radius: var(--radius-sm);
  font-size: 0.8125rem;
}

.traj-banner--notice {
  background: var(--accent-bg-light);
  border: 1px solid var(--line);
  color: var(--text-muted);
}

.traj-banner--gap {
  background: rgba(245, 158, 11, 0.1);
  border: 1px solid rgba(245, 158, 11, 0.3);
  color: #f59e0b;
}

.traj-banner--error {
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.3);
  color: #ef4444;
}

.run-block {
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--accent-bg-light);
  padding: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.run-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.run-id {
  font-family: var(--font-mono, monospace);
  font-size: 0.75rem;
  color: var(--text);
  font-weight: 600;
}

.run-status {
  font-size: 0.75rem;
  color: var(--text-muted);
}

.activity-chip,
.call-chip,
.wait-chip,
.ref-chip {
  font-size: 0.6875rem;
  padding: 0.125rem 0.5rem;
  border-radius: 9999px;
  background: var(--panel-solid);
  color: var(--text-muted);
}

.activity-chip--active {
  background: rgba(59, 130, 246, 0.15);
  color: #3b82f6;
}

.activity-chip--waiting {
  background: rgba(245, 158, 11, 0.15);
  color: #f59e0b;
}

.activity-chip--completed {
  background: rgba(16, 185, 129, 0.15);
  color: #10b981;
}

.activity-chip--failed,
.activity-chip--cancelled {
  background: rgba(239, 68, 68, 0.15);
  color: #ef4444;
}

.wait-chip {
  background: rgba(245, 158, 11, 0.15);
  color: #f59e0b;
}

.run-refs {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  flex-wrap: wrap;
  font-size: 0.75rem;
  color: var(--text-muted);
}

.request-table {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.request-row {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr;
  gap: 0.5rem;
  padding: 0.375rem 0.5rem;
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
  font-size: 0.75rem;
  align-items: center;
}

.request-row--head {
  background: transparent;
  color: var(--text-muted);
  font-weight: 600;
  padding-bottom: 0;
}

.req-id {
  font-family: var(--font-mono, monospace);
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.req-usage {
  color: var(--text-muted);
  text-align: right;
}

.call-chip--completed {
  background: rgba(16, 185, 129, 0.15);
  color: #10b981;
}

.call-chip--active {
  background: rgba(59, 130, 246, 0.15);
  color: #3b82f6;
}

.call-chip--failed,
.call-chip--cancelled,
.call-chip--interrupted {
  background: rgba(239, 68, 68, 0.15);
  color: #ef4444;
}

.record-list {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.record-row {
  display: grid;
  grid-template-columns: 4rem 5rem 1fr;
  gap: 0.5rem;
  font-size: 0.75rem;
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-sm);
}

.record-row--error .record-text {
  color: #ef4444;
}

.record-kind {
  color: var(--text-muted);
  text-transform: uppercase;
  font-size: 0.625rem;
  letter-spacing: 0.05em;
}

.record-group {
  color: var(--text-muted);
}

.record-text {
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>

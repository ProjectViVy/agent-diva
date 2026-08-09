<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Check, Clock3, Loader2, RotateCcw, ShieldCheck, X } from '@lucide/vue';
import {
  applyLaputaProposal,
  decideLaputaProposal,
  editLaputaProposal,
  rollbackLaputaChangelog,
  transitionLaputaProposal,
} from '../../api/desktop';
import type { ChangelogRecord, EvolutionProposal, LaputaSectionName } from '../../api/desktop';
import { appConfirm } from '../../utils/appDialog';
import { showAppToast } from '../../utils/appToast';

const props = defineProps<{
  sectionName: LaputaSectionName;
  authorityVersion?: string;
  sessionVersion?: string;
  proposal?: EvolutionProposal | null;
  changelog?: ChangelogRecord | null;
}>();

const emit = defineEmits<{ (event: 'changed'): void }>();
const { t } = useI18n();
const busy = ref('');
const editing = ref(false);
const editedPatch = ref('');

const sessionState = computed(() => {
  if (!props.sessionVersion) return 'not-captured';
  return props.sessionVersion === props.authorityVersion ? 'effective' : 'next-session';
});

async function run(name: string, action: () => Promise<unknown>) {
  busy.value = name;
  try {
    await action();
    emit('changed');
  } catch (error) {
    showAppToast(error instanceof Error ? error.message : String(error), 'error');
  } finally {
    busy.value = '';
  }
}

function idempotencyKey(action: string, proposal: EvolutionProposal) {
  return `${action}:${proposal.id}:${proposal.governance?.request_version ?? 0}`;
}

async function decide(decision: 'allow' | 'deny') {
  const proposal = props.proposal;
  const governance = proposal?.governance;
  if (!proposal || !governance) return;
  await run(decision, () => decideLaputaProposal(proposal.id, {
    decision,
    grant: 'once',
    expected_version: governance.request_version,
    idempotency_key: idempotencyKey(`decision-${decision}`, proposal),
  }));
}

async function approveAndApply() {
  const proposal = props.proposal;
  if (!proposal) return;
  const confirmed = await appConfirm(t('evolution.confirm.apply', { target: proposal.target_section }), {
    title: t('evolution.confirm.title'),
  });
  if (!confirmed) return;
  await run('apply', async () => {
    let active = proposal;
    if (active.state !== 'approved') {
      const governance = active.governance;
      if (!governance) throw new Error('Governance request is unavailable.');
      const decision = await decideLaputaProposal(active.id, {
        decision: 'allow',
        grant: 'once',
        expected_version: governance.request_version,
        idempotency_key: idempotencyKey('decision-allow', active),
      });
      active = decision.proposal;
      active.governance = decision.governance;
    }
    if (!active.governance) throw new Error('Governance receipt is unavailable.');
    await applyLaputaProposal(active.id, {
      governance_request_id: active.governance.request_id,
      expected_version: active.governance.request_version,
      idempotency_key: idempotencyKey('apply', active),
    });
  });
}

async function deferProposal() {
  if (!props.proposal) return;
  await run('defer', () => transitionLaputaProposal(props.proposal!.id, { state: 'deferred' }));
}

function beginEdit() {
  editedPatch.value = props.proposal?.proposed_patch ?? '';
  editing.value = true;
}

async function saveEdit() {
  if (!props.proposal || !editedPatch.value.trim()) return;
  await run('edit', () => editLaputaProposal(props.proposal!.id, {
    proposed_patch: editedPatch.value,
    updated_at: props.proposal!.updated_at,
  }));
  editing.value = false;
}

async function rollback() {
  if (!props.changelog) return;
  const confirmed = await appConfirm(t('evolution.confirm.rollback', { target: props.sectionName }), {
    title: t('evolution.confirm.title'),
  });
  if (!confirmed) return;
  await run('rollback', () => rollbackLaputaChangelog(props.changelog!.id, {
    reason: `Persona workspace rollback for ${props.sectionName}`,
  }));
}
</script>

<template>
  <aside class="lifecycle-panel">
    <header>
      <div><ShieldCheck :size="16" /> {{ t('laputa.lifecycle.title') }}</div>
      <span>{{ sectionName }}</span>
    </header>

    <ol class="lifecycle-rail">
      <li class="done"><Check :size="14" /><div><b>{{ t('laputa.lifecycle.authority') }}</b><small>{{ authorityVersion?.slice(0, 12) || '—' }}</small></div></li>
      <li :class="proposal ? 'active' : ''"><Clock3 :size="14" /><div><b>{{ t('laputa.lifecycle.proposal') }}</b><small>{{ proposal ? proposal.state : t('laputa.lifecycle.none') }}</small></div></li>
      <li :class="sessionState"><Check :size="14" /><div><b>{{ t('laputa.lifecycle.session') }}</b><small>{{ t(`laputa.lifecycle.${sessionState}`) }}</small></div></li>
      <li :class="changelog ? 'done' : ''"><RotateCcw :size="14" /><div><b>{{ t('laputa.lifecycle.audit') }}</b><small>{{ changelog ? new Date(changelog.created_at).toLocaleString() : '—' }}</small></div></li>
    </ol>

    <div v-if="proposal && !['applied', 'rejected'].includes(proposal.state)" class="lifecycle-actions">
      <button :disabled="Boolean(busy)" @click="approveAndApply"><Loader2 v-if="busy === 'apply'" :size="13" class="spin" /><Check v-else :size="13" />{{ t('evolution.actions.approveApply') }}</button>
      <button :disabled="Boolean(busy)" @click="decide('allow')">{{ t('evolution.actions.approveOnly') }}</button>
      <button :disabled="Boolean(busy)" @click="deferProposal">{{ t('evolution.actions.defer') }}</button>
      <button :disabled="Boolean(busy)" @click="beginEdit">{{ t('evolution.actions.edit') }}</button>
      <button class="danger" :disabled="Boolean(busy)" @click="decide('deny')"><X :size="13" />{{ t('evolution.actions.reject') }}</button>
    </div>
    <div v-if="editing" class="proposal-edit">
      <textarea v-model="editedPatch" :aria-label="t('evolution.actions.edit')" />
      <div><button @click="editing = false">{{ t('common.cancel') }}</button><button :disabled="!editedPatch.trim() || Boolean(busy)" @click="saveEdit">{{ t('common.save') }}</button></div>
    </div>
    <button v-if="changelog && !changelog.reverted" class="rollback" :disabled="Boolean(busy)" @click="rollback">
      <RotateCcw :size="13" />{{ t('evolution.actions.rollback') }}
    </button>
  </aside>
</template>

<style scoped>
.lifecycle-panel { border-left: 1px solid var(--line); background: var(--panel-solid); padding: 16px; overflow-y: auto; }
header { display: flex; justify-content: space-between; gap: 8px; color: var(--text-muted); font-size: 11px; text-transform: uppercase; letter-spacing: .06em; }
header div { display: flex; align-items: center; gap: 6px; color: var(--text); font-weight: 700; }
.lifecycle-rail { list-style: none; padding: 18px 0; margin: 0; }
.lifecycle-rail li { display: grid; grid-template-columns: 22px 1fr; gap: 8px; position: relative; padding-bottom: 22px; color: var(--text-muted); }
.lifecycle-rail li:not(:last-child)::after { content: ''; position: absolute; left: 6px; top: 18px; bottom: 4px; width: 1px; background: var(--line); }
.lifecycle-rail li.done, .lifecycle-rail li.effective { color: var(--success, #16a34a); }
.lifecycle-rail li.active, .lifecycle-rail li.next-session { color: var(--accent); }
.lifecycle-rail b, .lifecycle-rail small { display: block; }
.lifecycle-rail b { color: var(--text); font-size: 12px; }
.lifecycle-rail small { margin-top: 3px; font-size: 11px; overflow-wrap: anywhere; }
.lifecycle-actions { display: grid; gap: 7px; }
.proposal-edit { display: grid; gap: 7px; margin-top: 10px; }
.proposal-edit textarea { min-height: 140px; resize: vertical; border: 1px solid var(--line); border-radius: var(--radius-sm); background: var(--panel); color: var(--text); padding: 8px; font-family: ui-monospace, monospace; font-size: 11px; }
.proposal-edit div { display: flex; justify-content: flex-end; gap: 6px; }
button { display: inline-flex; align-items: center; justify-content: center; gap: 5px; border: 1px solid var(--line); border-radius: var(--radius-sm); background: var(--panel); color: var(--text); padding: 7px 9px; cursor: pointer; font-size: 12px; }
button:first-child { background: var(--accent); border-color: var(--accent); color: white; }
button.danger { color: var(--danger); }
button:disabled { opacity: .5; cursor: not-allowed; }
.rollback { width: 100%; margin-top: 8px; }
.spin { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
</style>

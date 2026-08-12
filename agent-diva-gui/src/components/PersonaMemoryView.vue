<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { BookUser, CircleDot, Loader2, RefreshCw, ScrollText, ShieldCheck } from '@lucide/vue';
import SectionGroupList, { type PersonaMenuItem } from './persona-memory/SectionGroupList.vue';
import SectionEditor from './persona-memory/SectionEditor.vue';
import PersonaLifecyclePanel from './persona-memory/PersonaLifecyclePanel.vue';
import {
  getLaputaPersonaWorkspace,
  isTauriRuntime,
} from '../api/desktop';
import type {
  ChangelogRecord,
  EvolutionProposal,
  LaputaCognitiveKind,
  LaputaSectionName,
  PersonaWorkspaceProjection,
} from '../api/desktop';
import { appConfirm } from '../utils/appDialog';
import { showAppToast } from '../utils/appToast';
import { errorMessage } from '../utils/errorMessage';

const props = defineProps<{ sessionKey?: string }>();
const emit = defineEmits<{ (event: 'proposal-created', proposalId: string): void }>();
const { t } = useI18n();

const selectedSection = ref<PersonaMenuItem>('identity');
const workspace = ref<PersonaWorkspaceProjection | null>(null);
const governedProposals = ref<EvolutionProposal[]>([]);
const loading = ref(false);
const error = ref('');
const draftContent = ref('');
const originalContent = ref('');
const isDirty = ref(false);
const editorRevision = ref(0);

const snapshot = computed(() => workspace.value?.snapshot ?? null);
const isCognitive = computed(() => selectedSection.value === 'memrules' || selectedSection.value === 'world');
const sectionName = computed(() => isCognitive.value ? null : selectedSection.value as LaputaSectionName);
const section = computed(() => sectionName.value ? snapshot.value?.sections[sectionName.value] : null);
const populatedCount = computed(() => ['identity', 'relationship', 'commitment', 'preferences']
  .filter((name) => snapshot.value?.sections[name]?.status === 'owned').length);
const pendingCount = computed(() => governedProposals.value.filter((proposal) => !['applied', 'rejected'].includes(proposal.state)).length);
const selectedProposal = computed(() => governedProposals.value
  .filter((proposal) => proposal.target_section === sectionName.value)
  .sort((a, b) => Date.parse(b.updated_at) - Date.parse(a.updated_at))[0] ?? null);
const selectedChangelog = computed(() => workspace.value?.changelog
  .find((record) => record.target_section === sectionName.value && !record.reverted) ?? null);
const sessionVersion = computed(() => sectionName.value
  ? workspace.value?.session?.section_versions[sectionName.value]
  : undefined);
const authorityVersion = computed(() => sectionName.value
  ? workspace.value?.authority_versions[sectionName.value]
  : undefined);
const sessionStatus = computed(() => {
  if (!workspace.value?.session) return t('laputa.workspace.noActiveSession');
  return sessionVersion.value === authorityVersion.value
    ? t('laputa.workspace.currentEffective')
    : t('laputa.workspace.nextSessionEffective');
});
const cognitiveContent = computed(() => {
  if (!workspace.value || !isCognitive.value) return '';
  return workspace.value.cognitive[selectedSection.value as LaputaCognitiveKind] ?? '';
});

function formatContent(value: unknown): string {
  return JSON.stringify(value ?? {}, null, 2);
}

function syncEditor() {
  const text = formatContent(section.value?.content);
  draftContent.value = text;
  originalContent.value = text;
  isDirty.value = false;
}

async function load() {
  loading.value = true;
  error.value = '';
  try {
    if (!isTauriRuntime()) {
      workspace.value = null;
      governedProposals.value = [];
      return;
    }
    const projection = await getLaputaPersonaWorkspace(props.sessionKey);
    workspace.value = projection;
    governedProposals.value = projection.proposals;
    syncEditor();
  } catch (cause) {
    error.value = errorMessage(cause, t('laputa.loadError'));
    showAppToast(error.value, 'error');
  } finally {
    loading.value = false;
  }
}

async function selectSection(next: PersonaMenuItem) {
  if (next === selectedSection.value) return;
  if (isDirty.value && !await appConfirm(t('laputa.confirmDiscard.message'), {
    title: t('laputa.confirmDiscard.title'),
  })) return;
  selectedSection.value = next;
  syncEditor();
}

async function onProposalCreated(_name: LaputaSectionName, result: { proposal_id: string }) {
  editorRevision.value += 1;
  emit('proposal-created', result.proposal_id);
  showAppToast(t('laputa.proposalCreated'), 'success');
  await load();
}

onMounted(load);
</script>

<template>
  <div class="persona-workspace">
    <header class="workspace-header">
      <div class="title"><BookUser :size="20" /><div><b>{{ t('laputa.workspace.title') }}</b><small>{{ t('laputa.workspace.subtitle') }}</small></div></div>
      <button :disabled="loading" @click="load"><Loader2 v-if="loading" :size="15" class="spin" /><RefreshCw v-else :size="15" />{{ t('laputa.refresh') }}</button>
    </header>

    <div class="status-strip">
      <div><ShieldCheck :size="16" /><span>{{ t('laputa.workspace.frozenCore') }}</span><b>{{ populatedCount }}/4</b></div>
      <div><CircleDot :size="16" /><span>{{ t('laputa.workspace.pending') }}</span><b>{{ pendingCount }}</b></div>
      <div><ScrollText :size="16" /><span>{{ t('laputa.workspace.session') }}</span><b>{{ sessionStatus }}</b></div>
    </div>

    <div v-if="error" class="workspace-error" role="alert">{{ error }} <button @click="load">{{ t('laputa.retry') }}</button></div>
    <div v-if="loading && !workspace" class="loading"><Loader2 :size="22" class="spin" />{{ t('laputa.loading') }}</div>
    <div v-if="workspace" class="workspace-grid">
      <SectionGroupList :snapshot="snapshot" :selected-section="selectedSection" @select="selectSection" />

      <main class="workspace-main">
        <section v-if="isCognitive" class="cognitive-panel">
          <header><ScrollText :size="16" />{{ t(`laputa.sections.${selectedSection}`) }}<span>{{ t('laputa.cognitiveReadOnly') }}</span></header>
          <pre>{{ cognitiveContent }}</pre>
        </section>
        <SectionEditor
          v-else-if="sectionName"
          :key="`${sectionName}-${editorRevision}`"
          v-model="draftContent"
          :section-name="sectionName"
          :display-name="t(`laputa.sections.${sectionName}`)"
          :initial-content="originalContent"
          :status="section?.status === 'owned' ? 'owned' : 'tbd'"
          :last-updated="section?.last_modified"
          :pending-proposal="Boolean(selectedProposal && !['applied', 'rejected'].includes(selectedProposal.state))"
          @proposal-created="onProposalCreated"
          @save-failed="(_name, message) => showAppToast(message, 'error')"
          @update:dirty="(value) => isDirty = value"
        />
      </main>

      <PersonaLifecyclePanel
        v-if="sectionName"
        :section-name="sectionName"
        :authority-version="authorityVersion"
        :session-version="sessionVersion"
        :proposal="selectedProposal"
        :changelog="selectedChangelog as ChangelogRecord | null"
        @changed="load"
      />
      <aside v-else class="cognitive-boundary">{{ t('laputa.workspace.cognitiveBoundary') }}</aside>
    </div>
  </div>
</template>

<style scoped>
.persona-workspace { height: 100%; display: flex; flex-direction: column; background: var(--panel); overflow: hidden; }
.workspace-header { min-height: 62px; padding: 10px 18px; display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid var(--line); }
.title { display: flex; align-items: center; gap: 10px; }
.title b, .title small { display: block; }
.title b { color: var(--text); font-size: 15px; }
.title small { color: var(--text-muted); margin-top: 2px; font-size: 11px; }
button { display: inline-flex; align-items: center; gap: 6px; border: 1px solid var(--line); border-radius: var(--radius-sm); background: var(--panel-solid); color: var(--text); padding: 7px 10px; cursor: pointer; }
.status-strip { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); border-bottom: 1px solid var(--line); background: var(--panel-solid); }
.status-strip div { display: grid; grid-template-columns: auto 1fr auto; align-items: center; gap: 8px; padding: 9px 16px; color: var(--text-muted); font-size: 11px; }
.status-strip div:not(:last-child) { border-right: 1px solid var(--line); }
.status-strip b { color: var(--text); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.workspace-grid { flex: 1; min-height: 0; display: grid; grid-template-columns: 230px minmax(420px, 1fr) 250px; }
.workspace-grid > :first-child { border-right: 1px solid var(--line); overflow-y: auto; }
.workspace-main { min-width: 0; overflow: hidden; }
.loading { height: 100%; display: grid; place-content: center; gap: 8px; color: var(--text-muted); font-size: 12px; }
.cognitive-panel { padding: 18px; height: 100%; display: flex; flex-direction: column; gap: 10px; }
.cognitive-panel header { display: flex; align-items: center; gap: 7px; color: var(--text); font-weight: 600; }
.cognitive-panel header span { margin-left: auto; color: var(--text-muted); font-size: 11px; }
.cognitive-panel pre { flex: 1; overflow: auto; padding: 14px; margin: 0; border: 1px solid var(--line); border-radius: var(--radius-sm); background: var(--panel-solid); color: var(--text); white-space: pre-wrap; }
.cognitive-boundary { border-left: 1px solid var(--line); padding: 18px; color: var(--text-muted); font-size: 12px; line-height: 1.6; }
.workspace-error { margin: 16px; padding: 12px; border: 1px solid var(--danger); color: var(--danger); border-radius: var(--radius-sm); }
.spin { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
@media (max-width: 1050px) { .workspace-grid { grid-template-columns: 210px minmax(0, 1fr); } .workspace-grid > aside { display: none; } }
</style>

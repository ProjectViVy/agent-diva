<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { BookOpen, Check, Clock3, Code2, Eye, FileClock, Loader2, RefreshCw, Save, X } from '@lucide/vue';
import MarkdownIt from 'markdown-it';
import PersonaMarkdownEditor from './persona-memory/PersonaMarkdownEditor.vue';
import {
  acceptPersonaRequest, getPersonaDocument, getPersonaHistoryRevision, isTauriRuntime,
  listPersonaHistory, listPersonaRequests, rejectPersonaRequest, savePersonaDocument,
} from '../api/desktop';
import type { PersonaChangeRequest, PersonaDocument, PersonaHistoryEntry, PersonaHistoryRevision, PersonaKind } from '../api/desktop';
import { appConfirm } from '../utils/appDialog';
import { showAppToast } from '../utils/appToast';
import { errorMessage } from '../utils/errorMessage';

defineProps<{ sessionKey?: string }>();
const { t } = useI18n();
const kinds: PersonaKind[] = ['identity', 'relationship', 'redline', 'user', 'world', 'dream', 'dark'];
const labels: Record<PersonaKind, string> = {
  identity: 'IDENTITY.MD', relationship: 'RELATIONSHIP.MD', redline: 'REDLINE.MD',
  user: 'USER.MD', world: 'WORLD.MD', dream: 'DREAM.MD', dark: 'DARK.MD',
};
const selectedKind = ref<PersonaKind>('identity');
const tab = ref<'current' | 'pending' | 'history'>('current');
const mode = ref<'source' | 'preview'>('source');
const document = ref<PersonaDocument | null>(null);
const draft = ref('');
const requests = ref<PersonaChangeRequest[]>([]);
const history = ref<PersonaHistoryEntry[]>([]);
const selectedRevision = ref<PersonaHistoryRevision | null>(null);
const loading = ref(false);
const saving = ref(false);
const error = ref('');
const dirty = computed(() => document.value !== null && draft.value !== document.value.content);
const markdown = new MarkdownIt({ html: false, linkify: true, breaks: true });
const rendered = computed(() => markdown.render(draft.value));
const pending = computed(() => requests.value.filter((request) => request.state === 'pending'));

async function loadCurrent(kind = selectedKind.value) {
  if (!isTauriRuntime()) return;
  loading.value = true; error.value = '';
  try {
    const [nextDocument, nextRequests, nextHistory] = await Promise.all([
      getPersonaDocument(kind), listPersonaRequests(kind), listPersonaHistory(kind),
    ]);
    document.value = nextDocument; draft.value = nextDocument.content;
    requests.value = nextRequests; history.value = nextHistory; selectedRevision.value = null;
  } catch (cause) {
    error.value = errorMessage(cause, t('personaWorkspace.loadFailed'));
  } finally { loading.value = false; }
}

async function selectKind(kind: PersonaKind) {
  if (kind === selectedKind.value) return;
  if (dirty.value && !await appConfirm(t('personaWorkspace.discardFile'), { title: t('personaWorkspace.discardTitle') })) return;
  selectedKind.value = kind; tab.value = 'current'; mode.value = 'source';
  await loadCurrent(kind);
}

async function selectTab(next: typeof tab.value) {
  if (next === tab.value) return;
  if (tab.value === 'current' && dirty.value && !await appConfirm(t('personaWorkspace.discardView'), { title: t('personaWorkspace.discardTitle') })) return;
  if (tab.value === 'current' && dirty.value && document.value) draft.value = document.value.content;
  tab.value = next;
}

async function save() {
  if (!document.value || !dirty.value) return;
  saving.value = true; error.value = '';
  try {
    const outcome = await savePersonaDocument(selectedKind.value, draft.value, document.value.revision, t('personaWorkspace.directSaveReason'));
    document.value = outcome.document; draft.value = outcome.document.content;
    history.value = await listPersonaHistory(selectedKind.value);
    showAppToast(outcome.changed ? t('personaWorkspace.saved') : t('personaWorkspace.unchanged'), 'success');
  } catch (cause) {
    error.value = errorMessage(cause, t('personaWorkspace.saveFailed'));
    if (error.value.includes('persona_revision_conflict')) await loadCurrent();
  } finally { saving.value = false; }
}

async function decide(request: PersonaChangeRequest, accept: boolean) {
  try {
    await (accept ? acceptPersonaRequest(request.id) : rejectPersonaRequest(request.id));
    await loadCurrent();
    tab.value = 'pending';
  } catch (cause) { error.value = errorMessage(cause, t('personaWorkspace.decideFailed')); }
}

async function openRevision(entry: PersonaHistoryEntry) {
  selectedRevision.value = await getPersonaHistoryRevision(selectedKind.value, entry.revision);
}

function restoreRevision() {
  if (!selectedRevision.value) return;
  draft.value = selectedRevision.value.content; tab.value = 'current'; mode.value = 'source';
}

onMounted(loadCurrent);
</script>

<template>
  <div class="persona-workspace">
    <header class="workspace-header">
      <div><BookOpen :size="19" /><span><b>{{ t('personaWorkspace.title') }}</b><small>{{ t('personaWorkspace.subtitle') }}</small></span></div>
      <button :disabled="loading" @click="loadCurrent()"><Loader2 v-if="loading" :size="15" class="spin" /><RefreshCw v-else :size="15" />{{ t('personaWorkspace.refresh') }}</button>
    </header>
    <div v-if="error" class="workspace-error" role="alert">{{ error }} <button @click="loadCurrent()">{{ t('personaWorkspace.retry') }}</button></div>
    <div class="workspace-grid">
      <nav aria-label="Persona files">
        <button v-for="kind in kinds" :key="kind" :class="{ active: selectedKind === kind }" @click="selectKind(kind)">
          <span>{{ labels[kind] }}</span><small v-if="kind === selectedKind && document">r{{ document.revision }}</small>
        </button>
      </nav>
      <main>
        <div class="document-bar">
          <div class="tabs">
            <button :class="{ active: tab === 'current' }" @click="selectTab('current')">{{ t('personaWorkspace.current') }}</button>
            <button :class="{ active: tab === 'pending' }" @click="selectTab('pending')">{{ t('personaWorkspace.pending') }} <i v-if="pending.length">{{ pending.length }}</i></button>
            <button :class="{ active: tab === 'history' }" @click="selectTab('history')">{{ t('personaWorkspace.history') }}</button>
          </div>
          <div v-if="tab === 'current'" class="actions">
            <button :class="{ active: mode === 'source' }" @click="mode = 'source'"><Code2 :size="14" />{{ t('personaWorkspace.source') }}</button>
            <button :class="{ active: mode === 'preview' }" @click="mode = 'preview'"><Eye :size="14" />{{ t('personaWorkspace.preview') }}</button>
            <button class="save" :disabled="!dirty || saving" @click="save"><Loader2 v-if="saving" :size="14" class="spin" /><Save v-else :size="14" />{{ t('personaWorkspace.save') }}</button>
          </div>
        </div>
        <section v-if="tab === 'current'" class="current-pane">
          <PersonaMarkdownEditor v-if="mode === 'source'" v-model="draft" />
          <article v-else class="markdown-preview" v-html="rendered" />
        </section>
        <section v-else-if="tab === 'pending'" class="list-pane">
          <div v-if="!requests.length" class="empty"><Clock3 :size="24" />{{ t('personaWorkspace.emptyRequests') }}</div>
          <article v-for="request in requests" :key="request.id" class="request-card">
            <header><b>{{ request.actor }} · {{ request.state }}</b><time>{{ new Date(request.created_at).toLocaleString() }}</time></header>
            <p>{{ request.reason }}</p><pre>{{ request.proposed_markdown }}</pre>
            <footer v-if="request.state === 'pending'"><button @click="decide(request, false)"><X :size="14" />{{ t('personaWorkspace.reject') }}</button><button class="accept" @click="decide(request, true)"><Check :size="14" />{{ t('personaWorkspace.accept') }}</button></footer>
          </article>
        </section>
        <section v-else class="history-pane">
          <aside><button v-for="entry in history" :key="entry.revision" :class="{ active: selectedRevision?.revision === entry.revision }" @click="openRevision(entry)"><b>r{{ entry.revision }}</b><span>{{ entry.reason || entry.source }}</span><time>{{ new Date(entry.created_at).toLocaleString() }}</time></button></aside>
          <div v-if="selectedRevision" class="revision"><header><b>r{{ selectedRevision.revision }}</b><button @click="restoreRevision"><FileClock :size="14" />{{ t('personaWorkspace.restore') }}</button></header><pre>{{ selectedRevision.unified_diff }}</pre></div>
          <div v-else class="empty"><FileClock :size="24" />{{ t('personaWorkspace.selectHistory') }}</div>
        </section>
      </main>
    </div>
  </div>
</template>

<style scoped>
.persona-workspace { height: 100%; display: flex; flex-direction: column; overflow: hidden; background: var(--panel); color: var(--text); }
button { display: inline-flex; align-items: center; gap: 6px; border: 1px solid var(--line); border-radius: 7px; background: var(--panel-solid); color: var(--text); padding: 7px 10px; cursor: pointer; } button:disabled { opacity: .45; cursor: default; }
.workspace-header { min-height: 62px; padding: 10px 18px; display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid var(--line); }.workspace-header > div { display: flex; align-items: center; gap: 10px; }.workspace-header span,.workspace-header b,.workspace-header small { display: block; }.workspace-header small { margin-top: 2px; color: var(--text-muted); font-size: 11px; }
.workspace-error { margin: 10px 16px 0; padding: 10px 12px; border: 1px solid var(--danger); border-radius: 8px; color: var(--danger); font-size: 12px; }
.workspace-grid { flex: 1; min-height: 0; display: grid; grid-template-columns: 218px minmax(0, 1fr); }
nav { padding: 12px; overflow: auto; border-right: 1px solid var(--line); } nav button { width: 100%; justify-content: space-between; margin-bottom: 5px; border-color: transparent; background: transparent; text-align: left; } nav button.active { border-color: color-mix(in srgb, var(--accent) 28%, var(--line)); background: color-mix(in srgb, var(--accent) 9%, transparent); color: var(--accent); } nav small { color: var(--text-muted); }
main { min-width: 0; min-height: 0; display: flex; flex-direction: column; }.document-bar { min-height: 48px; display: flex; align-items: center; justify-content: space-between; padding: 6px 12px; border-bottom: 1px solid var(--line); }.tabs,.actions { display: flex; gap: 5px; }.tabs button,.actions button { border-color: transparent; background: transparent; }.tabs button.active,.actions button.active { background: var(--panel-solid); border-color: var(--line); }.tabs i { min-width: 18px; border-radius: 9px; background: var(--danger); color: white; font-size: 10px; font-style: normal; }.actions .save { border-color: var(--accent); background: var(--accent); color: white; }
.current-pane { flex: 1; min-height: 0; }.markdown-preview { height: 100%; overflow: auto; box-sizing: border-box; padding: 22px 28px; line-height: 1.7; }.markdown-preview :deep(pre) { overflow: auto; padding: 12px; background: var(--panel-solid); border-radius: 8px; }.markdown-preview :deep(a) { color: var(--accent); }
.list-pane,.history-pane { flex: 1; min-height: 0; overflow: auto; padding: 18px; }.request-card { max-width: 820px; margin: 0 auto 12px; padding: 14px; border: 1px solid var(--line); border-radius: 10px; background: var(--panel-solid); }.request-card header { display: flex; justify-content: space-between; }.request-card time,.history-pane time { color: var(--text-muted); font-size: 10px; }.request-card p { color: var(--text-muted); font-size: 12px; }.request-card pre,.revision pre { overflow: auto; white-space: pre-wrap; padding: 12px; border-radius: 7px; background: var(--panel); }.request-card footer { display: flex; justify-content: flex-end; gap: 7px; }.request-card .accept { border-color: var(--accent); background: var(--accent); color: white; }
.history-pane { display: grid; grid-template-columns: 230px 1fr; gap: 14px; }.history-pane aside { overflow: auto; }.history-pane aside button { width: 100%; display: grid; grid-template-columns: auto 1fr; gap: 3px 8px; margin-bottom: 6px; text-align: left; }.history-pane aside button.active { border-color: var(--accent); }.history-pane aside span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }.history-pane aside time { grid-column: 1 / -1; }.revision { min-width: 0; overflow: hidden; display: flex; flex-direction: column; border: 1px solid var(--line); border-radius: 9px; }.revision header { display: flex; justify-content: space-between; align-items: center; padding: 10px 12px; border-bottom: 1px solid var(--line); }.revision pre { flex: 1; margin: 0; border-radius: 0; }.empty { height: 100%; display: grid; place-content: center; justify-items: center; gap: 8px; color: var(--text-muted); font-size: 12px; }
.spin { animation: spin 1s linear infinite; } @keyframes spin { to { transform: rotate(360deg); } }
@media (max-width: 760px) { .workspace-grid { grid-template-columns: 150px minmax(0,1fr); }.history-pane { grid-template-columns: 1fr; }.actions button { padding-inline: 7px; } }
</style>

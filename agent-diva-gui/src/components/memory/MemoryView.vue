<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Database, FileText, Loader2, Plus, RefreshCw, Save, Trash2 } from '@lucide/vue';
import {
  createMemoryRecord,
  deleteActmemCapsule,
  deleteMemoryRecord,
  getActmem,
  getActmemCapsule,
  getMemoryRecord,
  getMemoryRules,
  listActmemCapsules,
  listMemoryRecords,
  putActmem,
  putMemoryRules,
  updateMemoryRecord,
  type ActmemCapsule,
  type ActmemCapsuleSummary,
  type ActmemDocument,
  type MemoryRecord,
  type MemoryRulesDocument,
} from '../../api/desktop';
import PersonaMarkdownEditor from '../persona-memory/PersonaMarkdownEditor.vue';
import { appConfirm, appPrompt } from '../../utils/appDialog';
import { showAppToast } from '../../utils/appToast';

type WorkspaceTab = 'bml' | 'actmem' | 'memrules';
type ActmemSection = 'pulse' | 'recap' | 'work';

const { t } = useI18n();
const tab = ref<WorkspaceTab>('bml');
const loading = ref(false);
const saving = ref(false);
const error = ref('');

const records = ref<MemoryRecord[]>([]);
const selectedRecord = ref<MemoryRecord | null>(null);
const recordMode = ref<'read' | 'create' | 'edit'>('read');
const recordDraft = ref('');

const actmem = ref<ActmemDocument | null>(null);
const actmemEditSection = ref<ActmemSection | null>(null);
const actmemDraft = ref('');
const capsules = ref<ActmemCapsuleSummary[]>([]);
const selectedCapsule = ref<ActmemCapsule | null>(null);

const memrules = ref<MemoryRulesDocument | null>(null);
const memrulesEditing = ref(false);
const memrulesDraft = ref('');

const currentActmemValue = computed(() => {
  if (!actmem.value || !actmemEditSection.value) return '';
  return actmem.value[actmemEditSection.value];
});

function normalizeError(value: unknown): string {
  if (value && typeof value === 'object' && 'message' in value) return String(value.message);
  return value instanceof Error ? value.message : String(value);
}

function formatDate(value?: string | null): string {
  if (!value) return '—';
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString();
}

function summary(value: string): string {
  const compact = value.replace(/\s+/g, ' ').trim();
  return compact.length > 120 ? `${compact.slice(0, 120)}…` : compact;
}

async function loadBml(): Promise<void> {
  loading.value = true;
  error.value = '';
  try {
    records.value = await listMemoryRecords();
    if (selectedRecord.value) {
      selectedRecord.value = records.value.find((item) => item.id === selectedRecord.value?.id) ?? null;
    }
  } catch (value) {
    error.value = normalizeError(value);
  } finally {
    loading.value = false;
  }
}

async function loadActmem(): Promise<void> {
  loading.value = true;
  error.value = '';
  try {
    [actmem.value, capsules.value] = await Promise.all([getActmem(), listActmemCapsules()]);
  } catch (value) {
    error.value = normalizeError(value);
  } finally {
    loading.value = false;
  }
}

async function loadMemrules(): Promise<void> {
  loading.value = true;
  error.value = '';
  try {
    memrules.value = await getMemoryRules();
  } catch (value) {
    error.value = normalizeError(value);
  } finally {
    loading.value = false;
  }
}

async function refreshCurrent(): Promise<void> {
  if (tab.value === 'bml') await loadBml();
  if (tab.value === 'actmem') await loadActmem();
  if (tab.value === 'memrules') await loadMemrules();
}

async function selectTab(next: WorkspaceTab): Promise<void> {
  tab.value = next;
  error.value = '';
  await refreshCurrent();
}

async function selectRecord(id: string): Promise<void> {
  if (saving.value) return;
  error.value = '';
  try {
    selectedRecord.value = await getMemoryRecord(id);
    recordMode.value = 'read';
  } catch (value) {
    error.value = normalizeError(value);
  }
}

function beginCreateRecord(): void {
  selectedRecord.value = null;
  recordDraft.value = '';
  recordMode.value = 'create';
  error.value = '';
}

function beginEditRecord(): void {
  if (!selectedRecord.value) return;
  recordDraft.value = selectedRecord.value.content;
  recordMode.value = 'edit';
  error.value = '';
}

function cancelRecordEdit(): void {
  recordMode.value = 'read';
  recordDraft.value = '';
  error.value = '';
}

async function saveRecord(): Promise<void> {
  const content = recordDraft.value.trim();
  if (!content || saving.value) return;
  saving.value = true;
  error.value = '';
  try {
    const saved = recordMode.value === 'create'
      ? await createMemoryRecord(content)
      : await updateMemoryRecord(selectedRecord.value!.id, content, selectedRecord.value!.revision);
    selectedRecord.value = saved;
    const index = records.value.findIndex((item) => item.id === saved.id);
    if (index >= 0) records.value.splice(index, 1, saved);
    else records.value.unshift(saved);
    recordMode.value = 'read';
    showAppToast(t('memory.saved'), 'success');
  } catch (value) {
    error.value = normalizeError(value);
  } finally {
    saving.value = false;
  }
}

async function removeRecord(): Promise<void> {
  const record = selectedRecord.value;
  if (!record || saving.value) return;
  const reason = await appPrompt(t('memory.removePrompt.message'), {
    title: t('memory.removePrompt.title'),
    placeholder: t('memory.removePrompt.placeholder'),
    confirmLabel: t('memory.removePrompt.confirm'),
    cancelLabel: t('memory.cancel'),
  });
  if (!reason) return;
  saving.value = true;
  error.value = '';
  try {
    await deleteMemoryRecord(record.id, reason, record.revision);
    records.value = records.value.filter((item) => item.id !== record.id);
    selectedRecord.value = null;
    showAppToast(t('memory.deleted'), 'success');
  } catch (value) {
    error.value = normalizeError(value);
  } finally {
    saving.value = false;
  }
}

function beginActmemEdit(section: ActmemSection): void {
  if (!actmem.value) return;
  actmemEditSection.value = section;
  actmemDraft.value = actmem.value[section];
  error.value = '';
}

async function saveActmem(): Promise<void> {
  if (!actmem.value || !actmemEditSection.value || saving.value) return;
  saving.value = true;
  error.value = '';
  try {
    actmem.value = await putActmem({
      [actmemEditSection.value]: actmemDraft.value,
      base_revision: actmem.value.revision,
    });
    actmemEditSection.value = null;
    showAppToast(t('memory.saved'), 'success');
  } catch (value) {
    error.value = normalizeError(value);
  } finally {
    saving.value = false;
  }
}

async function selectCapsule(name: string): Promise<void> {
  error.value = '';
  try {
    selectedCapsule.value = await getActmemCapsule(name);
  } catch (value) {
    error.value = normalizeError(value);
  }
}

async function removeCapsule(): Promise<void> {
  const capsule = selectedCapsule.value;
  if (!capsule || saving.value) return;
  const confirmed = await appConfirm(t('memory.capsuleDeleteConfirm'), {
    title: t('memory.capsules'),
    confirmLabel: t('memory.delete'),
    cancelLabel: t('memory.cancel'),
  });
  if (!confirmed) return;
  saving.value = true;
  try {
    await deleteActmemCapsule(capsule.name);
    capsules.value = capsules.value.filter((item) => item.name !== capsule.name);
    selectedCapsule.value = null;
  } catch (value) {
    error.value = normalizeError(value);
  } finally {
    saving.value = false;
  }
}

function beginMemrulesEdit(): void {
  if (!memrules.value) return;
  memrulesDraft.value = memrules.value.content;
  memrulesEditing.value = true;
  error.value = '';
}

async function saveMemrules(): Promise<void> {
  if (!memrules.value || saving.value) return;
  saving.value = true;
  error.value = '';
  try {
    memrules.value = await putMemoryRules(memrulesDraft.value);
    memrulesEditing.value = false;
    showAppToast(t('memory.saved'), 'success');
  } catch (value) {
    error.value = normalizeError(value);
  } finally {
    saving.value = false;
  }
}

onMounted(() => void loadBml());
</script>

<template>
  <div class="memory-workspace">
    <header class="workspace-header">
      <div class="workspace-title"><Database :size="18" /><strong>{{ t('memory.title') }}</strong></div>
      <button class="button secondary" type="button" :disabled="loading || saving" @click="refreshCurrent">
        <Loader2 v-if="loading" :size="15" class="spin" /><RefreshCw v-else :size="15" />
        {{ t('memory.refresh') }}
      </button>
    </header>

    <nav class="workspace-tabs" :aria-label="t('memory.workspaceTabs')">
      <button v-for="item in (['bml', 'actmem', 'memrules'] as const)" :key="item" type="button" :class="{ active: tab === item }" @click="selectTab(item)">
        {{ t(`memory.tabs.${item}`) }}
      </button>
    </nav>

    <p v-if="error" class="error-banner">{{ error }}</p>

    <div v-if="tab === 'bml'" class="split-layout">
      <section class="list-panel">
        <div class="panel-toolbar">
          <strong>{{ t('memory.records') }}</strong>
          <button class="button primary" type="button" @click="beginCreateRecord"><Plus :size="14" />{{ t('memory.add') }}</button>
        </div>
        <div v-if="loading && records.length === 0" class="empty"><Loader2 :size="22" class="spin" /></div>
        <div v-else-if="records.length === 0" class="empty">{{ t('memory.emptyTitle') }}</div>
        <button v-for="record in records" v-else :key="record.id" class="record-row" :class="{ active: selectedRecord?.id === record.id }" type="button" @click="selectRecord(record.id)">
          <span>{{ summary(record.content) }}</span><time>{{ formatDate(record.updated_at) }}</time>
        </button>
      </section>

      <section class="detail-panel">
        <div v-if="recordMode === 'create' || recordMode === 'edit'" class="editor-pane">
          <div class="panel-toolbar"><strong>{{ t(recordMode === 'create' ? 'memory.add' : 'memory.edit') }}</strong></div>
          <textarea v-model="recordDraft" class="content-textarea" :placeholder="t('memory.contentPlaceholder')" />
          <div class="action-row">
            <button class="button secondary" type="button" :disabled="saving" @click="cancelRecordEdit">{{ t('memory.cancel') }}</button>
            <button class="button primary" type="button" :disabled="saving || !recordDraft.trim()" @click="saveRecord"><Save :size="14" />{{ t('memory.save') }}</button>
          </div>
        </div>
        <div v-else-if="selectedRecord" class="record-detail">
          <div class="panel-toolbar">
            <strong>{{ t('memory.recordDetail') }}</strong>
            <div class="action-row compact">
              <button class="button secondary" type="button" @click="beginEditRecord">{{ t('memory.edit') }}</button>
              <button class="button danger" type="button" :disabled="saving" @click="removeRecord"><Trash2 :size="14" />{{ t('memory.delete') }}</button>
            </div>
          </div>
          <pre class="read-content">{{ selectedRecord.content }}</pre>
          <dl class="meta-list">
            <div><dt>{{ t('memory.meta.revision') }}</dt><dd>{{ selectedRecord.revision }}</dd></div>
            <div><dt>{{ t('memory.meta.source') }}</dt><dd>{{ selectedRecord.provenance || '—' }}</dd></div>
            <div><dt>{{ t('memory.meta.effectiveAt') }}</dt><dd>{{ formatDate(selectedRecord.updated_at) }}</dd></div>
          </dl>
        </div>
        <div v-else class="empty"><FileText :size="30" />{{ t('memory.detailPlaceholder') }}</div>
      </section>
    </div>

    <div v-else-if="tab === 'actmem'" class="actmem-layout">
      <section class="actmem-head">
        <div class="panel-toolbar"><strong>ACTMEM</strong><span v-if="actmem" class="source-badge">r{{ actmem.revision }}</span></div>
        <template v-if="actmem">
          <article v-for="section in (['pulse', 'recap', 'work'] as const)" :key="section" class="section-card">
            <div class="section-card-title"><strong>{{ t(`memory.actmem.${section}`) }}</strong><button class="button secondary" type="button" @click="beginActmemEdit(section)">{{ t('memory.edit') }}</button></div>
            <pre>{{ actmem[section] || t('memory.emptySection') }}</pre>
          </article>
        </template>
      </section>
      <section class="capsule-panel">
        <div class="panel-toolbar"><strong>{{ t('memory.capsules') }}</strong></div>
        <div v-if="capsules.length === 0" class="empty small">{{ t('memory.noCapsules') }}</div>
        <button v-for="capsule in capsules" v-else :key="capsule.name" class="record-row" type="button" @click="selectCapsule(capsule.name)">
          <span>{{ capsule.session_key }}</span><time>{{ formatDate(capsule.created_at) }}</time>
        </button>
        <div v-if="selectedCapsule" class="capsule-detail">
          <div class="section-card-title"><strong>{{ selectedCapsule.name }}</strong><button class="button danger" type="button" @click="removeCapsule"><Trash2 :size="14" />{{ t('memory.delete') }}</button></div>
          <pre>{{ selectedCapsule.markdown }}</pre>
        </div>
      </section>
      <div v-if="actmemEditSection" class="editor-overlay">
        <div class="editor-dialog">
          <div class="panel-toolbar"><strong>{{ t(`memory.actmem.${actmemEditSection}`) }}</strong><span>r{{ actmem?.revision }}</span></div>
          <div class="markdown-editor"><PersonaMarkdownEditor v-model="actmemDraft" /></div>
          <div class="action-row">
            <button class="button secondary" type="button" @click="actmemEditSection = null">{{ t('memory.cancel') }}</button>
            <button class="button primary" type="button" :disabled="saving || actmemDraft === currentActmemValue" @click="saveActmem"><Save :size="14" />{{ t('memory.save') }}</button>
          </div>
        </div>
      </div>
    </div>

    <section v-else class="rules-panel">
      <div class="panel-toolbar">
        <div><strong>MEMRULES</strong><span v-if="memrules" class="source-badge">{{ t(`memory.ruleSource.${memrules.source}`) }}</span></div>
        <button v-if="!memrulesEditing" class="button secondary" type="button" @click="beginMemrulesEdit">{{ t('memory.edit') }}</button>
      </div>
      <div v-if="memrulesEditing" class="rules-editor">
        <div class="markdown-editor"><PersonaMarkdownEditor v-model="memrulesDraft" /></div>
        <div class="action-row">
          <button class="button secondary" type="button" @click="memrulesEditing = false">{{ t('memory.cancel') }}</button>
          <button class="button primary" type="button" :disabled="saving || memrulesDraft === memrules?.content" @click="saveMemrules"><Save :size="14" />{{ t('memory.save') }}</button>
        </div>
      </div>
      <pre v-else-if="memrules" class="read-content rules-content">{{ memrules.content }}</pre>
    </section>
  </div>
</template>

<style scoped>
.memory-workspace { height: 100%; display: flex; flex-direction: column; min-height: 0; color: var(--text); background: var(--panel); }
.workspace-header, .panel-toolbar, .section-card-title, .action-row { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
.workspace-header { min-height: 56px; padding: 10px 18px; border-bottom: 1px solid var(--line); }
.workspace-title { display: flex; align-items: center; gap: 8px; }
.workspace-tabs { display: flex; gap: 4px; padding: 8px 18px; border-bottom: 1px solid var(--line); overflow-x: auto; }
.workspace-tabs button { border: 0; background: transparent; color: var(--text-muted); padding: 7px 14px; border-radius: var(--radius-sm); cursor: pointer; font-weight: 600; }
.workspace-tabs button.active { color: var(--accent); background: var(--accent-bg-light); }
.button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; border: 1px solid var(--line); border-radius: var(--radius-sm); padding: 7px 11px; cursor: pointer; color: var(--text); background: var(--panel-solid); }
.button:disabled { opacity: .55; cursor: default; }.button.primary { color: white; background: var(--accent); border-color: var(--accent); }.button.danger { color: var(--danger-strong, #dc2626); }.action-row.compact { justify-content: flex-end; }
.error-banner { margin: 8px 18px 0; padding: 8px 10px; color: var(--danger-deep, #b91c1c); background: color-mix(in srgb, #ef4444 10%, transparent); border-radius: var(--radius-sm); }
.split-layout, .actmem-layout { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(260px, 34%) 1fr; }
.list-panel, .capsule-panel { border-right: 1px solid var(--line); overflow: auto; }.detail-panel, .actmem-head, .rules-panel { min-width: 0; overflow: auto; }
.panel-toolbar { min-height: 52px; padding: 10px 14px; border-bottom: 1px solid var(--line); }
.record-row { width: 100%; display: flex; flex-direction: column; align-items: flex-start; gap: 5px; padding: 12px 14px; text-align: left; border: 0; border-bottom: 1px solid var(--line); background: transparent; color: var(--text); cursor: pointer; }
.record-row:hover, .record-row.active { background: var(--accent-bg-light); }.record-row time { font-size: 11px; color: var(--text-muted); }
.empty { min-height: 180px; display: flex; align-items: center; justify-content: center; gap: 9px; color: var(--text-muted); }.empty.small { min-height: 90px; }
.editor-pane, .record-detail, .rules-editor { height: 100%; display: flex; flex-direction: column; min-height: 0; }.content-textarea { flex: 1; min-height: 220px; margin: 14px; resize: none; padding: 12px; border: 1px solid var(--line); border-radius: var(--radius-sm); color: var(--text); background: var(--panel-solid); }
.editor-pane > .action-row, .rules-editor > .action-row, .editor-dialog > .action-row { padding: 10px 14px; border-top: 1px solid var(--line); justify-content: flex-end; }
.read-content, .section-card pre, .capsule-detail pre { margin: 0; padding: 16px; white-space: pre-wrap; overflow-wrap: anywhere; font: 13px/1.6 var(--font-mono, ui-monospace); }
.meta-list { padding: 0 16px; }.meta-list div { display: grid; grid-template-columns: 120px 1fr; padding: 8px 0; border-top: 1px solid var(--line); }.meta-list dt { color: var(--text-muted); }
.actmem-layout { grid-template-columns: minmax(0, 1fr) 320px; }.actmem-head { padding-bottom: 20px; }.capsule-panel { border-right: 0; border-left: 1px solid var(--line); }.section-card { margin: 14px; border: 1px solid var(--line); border-radius: var(--radius-sm); overflow: hidden; }.section-card-title { padding: 9px 12px; border-bottom: 1px solid var(--line); }.section-card pre { max-height: 220px; overflow: auto; }.capsule-detail { margin: 12px; border: 1px solid var(--line); border-radius: var(--radius-sm); }.capsule-detail .section-card-title { align-items: flex-start; overflow-wrap: anywhere; }
.source-badge { margin-left: 8px; font-size: 11px; color: var(--text-muted); }.rules-panel { flex: 1; }.rules-content { max-width: 980px; }.markdown-editor { flex: 1; min-height: 280px; }
.editor-overlay { position: absolute; inset: 0; z-index: 20; display: grid; place-items: center; padding: 24px; background: color-mix(in srgb, #000 35%, transparent); }.memory-workspace { position: relative; }.editor-dialog { width: min(900px, 100%); height: min(680px, 100%); display: flex; flex-direction: column; background: var(--panel); border: 1px solid var(--line); border-radius: var(--radius); box-shadow: 0 18px 50px rgb(0 0 0 / .25); overflow: hidden; }
.spin { animation: spin 1s linear infinite; }@keyframes spin { to { transform: rotate(360deg); } }
@media (max-width: 760px) { .workspace-header { padding-inline: 12px; }.workspace-tabs { padding-inline: 12px; }.split-layout, .actmem-layout { grid-template-columns: 1fr; overflow: auto; }.list-panel, .capsule-panel { border: 0; border-bottom: 1px solid var(--line); max-height: 42vh; }.detail-panel { min-height: 44vh; }.editor-overlay { padding: 8px; }.editor-dialog { height: 100%; }.workspace-header .button { font-size: 0; }.workspace-header .button svg { margin: 0; } }
</style>

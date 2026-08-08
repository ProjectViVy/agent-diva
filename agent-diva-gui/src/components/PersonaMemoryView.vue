<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { BookUser, Loader2, RefreshCw, Inbox, ScrollText } from '@lucide/vue';
import SectionGroupList, { type PersonaMenuItem } from './persona-memory/SectionGroupList.vue';
import SectionEditor from './persona-memory/SectionEditor.vue';
import PersonaMemoryEmptyState from './persona-memory/PersonaMemoryEmptyState.vue';
import PersonaMemoryErrorState from './persona-memory/PersonaMemoryErrorState.vue';
import {
  getLaputaCognitiveFile,
  getLaputaSnapshot,
  getLaputaSection,
  isTauriRuntime,
} from '../api/desktop';
import type { LaputaCognitiveKind, LaputaSection, LaputaSectionName } from '../api/desktop';
import { showAppToast } from '../utils/appToast';
import { appConfirm } from '../utils/appDialog';

const { t } = useI18n();
const emit = defineEmits<{
  (event: 'proposal-created', proposalId: string): void;
}>();

interface LaputaSnapshot {
  sections: Record<string, {
    status: 'owned' | 'tbd';
    last_modified?: string | null;
  }>;
}

const selectedSection = ref<PersonaMenuItem>('identity');
const snapshot = ref<LaputaSnapshot | null>(null);
const loadingSnapshot = ref(false);
const loadingSection = ref(false);
const sectionError = ref('');
const sectionContent = ref<LaputaSection | null>(null);
const cognitiveContent = ref('');
const draftContent = ref('');
const originalContent = ref('');
const isDirty = ref(false);
const editorRevision = ref(0);

const isCognitiveFile = computed(() => isCognitive(selectedSection.value));

function isCognitive(name: PersonaMenuItem): name is LaputaCognitiveKind {
  return name === 'memrules' || name === 'world';
}

const displayName = computed(() => t('laputa.sections.' + selectedSection.value));

const selectedSectionStatus = computed(() =>
  isSectionName(selectedSection.value)
    ? (snapshot.value?.sections[selectedSection.value]?.status ?? 'tbd')
    : 'tbd',
);

const selectedSectionLastUpdated = computed(() =>
  isSectionName(selectedSection.value)
    ? (snapshot.value?.sections[selectedSection.value]?.last_modified ?? undefined)
    : undefined,
);

function isSectionName(name: PersonaMenuItem): name is LaputaSectionName {
  return !isCognitive(name);
}

const isUninitialized = computed(() => {
  return snapshot.value !== null && Object.keys(snapshot.value.sections).length === 0;
});

function hasMessage(err: unknown): err is { message: unknown } {
  return (
    err !== null &&
    err !== undefined &&
    typeof err === 'object' &&
    'message' in err
  );
}

function normalizeError(err: unknown): string {
  if (hasMessage(err)) {
    return String(err.message);
  }
  return err instanceof Error ? err.message : String(err);
}

async function loadSnapshot(): Promise<void> {
  loadingSnapshot.value = true;
  sectionError.value = '';
  try {
    if (isTauriRuntime()) {
      const raw = await getLaputaSnapshot();
      snapshot.value = {
        sections: Object.fromEntries(
          Object.entries(raw.sections).map(([name, section]) => [
            name,
            {
              status: section.status === 'owned' ? 'owned' : 'tbd',
              last_modified: section.last_modified ?? null,
            },
          ]),
        ),
      };
    } else {
      snapshot.value = { sections: {} };
    }
  } catch (err: unknown) {
    sectionError.value = normalizeError(err);
    showAppToast(t('laputa.loadError'), 'error');
  } finally {
    loadingSnapshot.value = false;
  }
}

async function loadCognitiveFile(kind: LaputaCognitiveKind): Promise<void> {
  loadingSection.value = true;
  sectionError.value = '';
  try {
    if (isTauriRuntime()) {
      const result = await getLaputaCognitiveFile(kind);
      cognitiveContent.value = result.content ?? '';
    } else {
      cognitiveContent.value = '';
    }
  } catch (err: unknown) {
    sectionError.value = normalizeError(err);
    showAppToast(t('laputa.loadError'), 'error');
  } finally {
    loadingSection.value = false;
  }
}

async function loadSection(name: LaputaSectionName): Promise<void> {
  loadingSection.value = true;
  sectionError.value = '';
  try {
    if (isTauriRuntime()) {
      sectionContent.value = await getLaputaSection(name);
    } else {
      sectionContent.value = null;
    }
    const raw = sectionContent.value?.content;
    const text = raw === null || raw === undefined ? '' : String(raw);
    draftContent.value = text;
    originalContent.value = text;
  } catch (err: unknown) {
    sectionError.value = normalizeError(err);
    showAppToast(t('laputa.loadError'), 'error');
  } finally {
    loadingSection.value = false;
  }
}

async function loadSelected(): Promise<void> {
  if (isCognitive(selectedSection.value)) {
    await loadCognitiveFile(selectedSection.value);
  } else {
    await loadSection(selectedSection.value);
  }
}

async function selectSection(nextId: PersonaMenuItem): Promise<void> {
  if (nextId === selectedSection.value) return;
  if (isDirty.value) {
    const confirmed = await appConfirm(
      t('laputa.confirmDiscard.message'),
      {
        title: t('laputa.confirmDiscard.title'),
        confirmLabel: t('laputa.confirmDiscard.discard'),
        cancelLabel: t('laputa.confirmDiscard.cancel'),
      },
    );
    if (!confirmed) return;
  }
  selectedSection.value = nextId;
  sectionError.value = '';
  draftContent.value = '';
  originalContent.value = '';
  cognitiveContent.value = '';
  isDirty.value = false;
  await loadSelected();
}

async function onSectionSelect(name: PersonaMenuItem): Promise<void> {
  await selectSection(name);
}

async function onRefresh(): Promise<void> {
  await loadSnapshot();
  if (isUninitialized.value) return;
  await loadSelected();
}

async function onProposalCreated(
  _name: LaputaSectionName,
  result: import('../api/desktop').WriteLaputaSectionResult,
): Promise<void> {
  await loadSelected();
  await loadSnapshot();
  editorRevision.value += 1;
  emit('proposal-created', result.proposal_id);
  showAppToast(t('laputa.proposalCreated'), 'success');
}

function onSaveFailed(_name: LaputaSectionName, message: string): void {
  showAppToast(t('laputa.saveFailed', { message }), 'error');
}

function onDirtyUpdate(next: boolean): void {
  isDirty.value = next;
}

onMounted(() => {
  loadSnapshot();
  loadSelected();
});
</script>

<template>
  <div class="persona-memory-view">
    <header class="persona-memory-header">
      <div class="persona-memory-title-block">
        <BookUser :size="18" />
        <span>{{ t('laputa.title') }}</span>
      </div>
      <button
        class="persona-memory-refresh"
        type="button"
        :disabled="loadingSnapshot || loadingSection"
        @click="onRefresh"
      >
        <Loader2 v-if="loadingSnapshot || loadingSection" :size="15" class="spin" />
        <RefreshCw v-else :size="15" />
        <span>{{ t('laputa.refresh') }}</span>
      </button>
    </header>

    <div class="persona-memory-body">
      <div class="persona-memory-list">
        <template v-if="loadingSnapshot">
          <div v-for="i in 6" :key="i" class="persona-memory-skeleton-item">
            <div class="skeleton-line short" />
            <div class="skeleton-line" />
          </div>
        </template>

        <SectionGroupList
          v-else-if="snapshot && !isUninitialized"
          :snapshot="snapshot"
          :selected-section="selectedSection"
          :aria-label="t('laputa.a11y.sectionList')"
          @select="onSectionSelect"
        />
      </div>

      <div class="persona-memory-detail">
        <template v-if="loadingSnapshot || loadingSection">
          <div class="persona-memory-detail-skeleton">
            <div class="skeleton-line title" />
            <div class="skeleton-line" />
            <div class="skeleton-line" />
            <div class="skeleton-line short" />
            <div class="skeleton-line" />
            <div class="skeleton-line medium" />
          </div>
        </template>

        <PersonaMemoryErrorState
          v-else-if="sectionError"
          :title="t('laputa.loadError')"
          :message="sectionError"
          :on-retry="() => loadSelected()"
        />

        <PersonaMemoryEmptyState
          v-else-if="isUninitialized"
          :icon="Inbox"
          :title="t('laputa.uninitializedTitle')"
          :description="t('laputa.uninitializedDesc')"
        />

        <!-- 认知治理文件（MEMRULES / WORLD）：只读展示 -->
        <div v-else-if="isCognitiveFile" class="cognitive-file-panel">
          <div class="cognitive-file-header">
            <ScrollText :size="16" />
            <span>{{ displayName }}</span>
            <span class="cognitive-file-badge">{{ t('laputa.cognitiveReadOnly') }}</span>
          </div>
          <pre class="cognitive-file-content">{{ cognitiveContent }}</pre>
        </div>

        <template v-else>
          <SectionEditor
            :key="`${selectedSection}-${editorRevision}`"
            v-model="draftContent"
            :section-name="selectedSection as LaputaSectionName"
            :display-name="displayName"
            :initial-content="originalContent"
            :status="selectedSectionStatus"
            :last-updated="selectedSectionLastUpdated"
            @proposal-created="onProposalCreated"
            @save-failed="onSaveFailed"
            @update:dirty="onDirtyUpdate"
          />
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.persona-memory-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--panel);
  border-radius: var(--radius);
  overflow: hidden;
}

.persona-memory-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px;
  border-bottom: 1px solid var(--line);
  flex-shrink: 0;
  min-height: 56px;
}

.persona-memory-title-block {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}

.persona-memory-refresh {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 6px 12px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
  color: var(--text);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.persona-memory-refresh:hover:not(:disabled) {
  background: var(--accent-bg-light);
  border-color: var(--accent-border);
  color: var(--accent);
}

.persona-memory-refresh:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.persona-memory-refresh:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--accent-glow), 0 0 0 4px var(--accent);
}

.persona-memory-body {
  display: flex;
  flex: 1;
  width: 100%;
  height: 100%;
  background: var(--panel, #ffffff);
}

.persona-memory-list {
  width: 280px;
  min-width: 280px;
  border-right: 1px solid var(--line);
  overflow-y: auto;
  flex-shrink: 0;
}

.persona-memory-detail {
  flex: 1;
  overflow-y: auto;
  min-width: 0;
  position: relative;
}

.cognitive-file-panel {
  padding: 20px 24px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  box-sizing: border-box;
}

.cognitive-file-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
  flex-shrink: 0;
}

.cognitive-file-badge {
  margin-left: auto;
  padding: 2px 10px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  background: var(--accent-bg-light);
  color: var(--accent);
}

.cognitive-file-content {
  flex: 1;
  margin: 0;
  padding: 14px 16px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-muted, #f8fafc);
  font-size: 13px;
  line-height: 1.7;
  color: var(--text);
  white-space: pre-wrap;
  word-break: break-word;
  overflow-y: auto;
}

.persona-memory-skeleton-item {
  padding: 12px;
  margin-bottom: 4px;
}

.persona-memory-detail-skeleton {
  padding: 24px;
}

.skeleton-line {
  height: 12px;
  border-radius: 4px;
  background: var(--accent-bg-light);
  margin-bottom: 10px;
  animation: skeleton-pulse 1.5s ease-in-out infinite;
}

.skeleton-line.short { width: 40%; }
.skeleton-line.medium { width: 65%; }
.skeleton-line.long { width: 85%; }
.skeleton-line.title { width: 55%; height: 18px; margin-bottom: 16px; }

@keyframes skeleton-pulse {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 0.8; }
}

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>

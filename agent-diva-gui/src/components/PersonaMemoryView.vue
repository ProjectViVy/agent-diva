<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { BookUser, Loader2, RefreshCw, Inbox, Network, Database, Sparkles } from '@lucide/vue';
import SectionGroupList from './persona-memory/SectionGroupList.vue';
import SectionEditor from './persona-memory/SectionEditor.vue';
import PersonaMemoryEmptyState from './persona-memory/PersonaMemoryEmptyState.vue';
import PersonaMemoryErrorState from './persona-memory/PersonaMemoryErrorState.vue';
import { getLaputaSnapshot, getLaputaSection, isTauriRuntime } from '../api/desktop';
import type { LaputaSection, LaputaSectionName } from '../api/desktop';
import { showAppToast } from '../utils/appToast';
import { appConfirm } from '../utils/appDialog';

const { t } = useI18n();
const emit = defineEmits<{
  (event: 'proposal-created', proposalId: string): void;
  (event: 'open-memory'): void;
}>();

interface LaputaSnapshot {
  sections: Record<string, {
    status: 'owned' | 'tbd';
    last_modified?: string | null;
  }>;
}

type GovernanceNode = 'garden' | 'laputa' | 'bml';
const activeNode = ref<GovernanceNode>('laputa');

const selectedSection = ref<LaputaSectionName>('identity');
const snapshot = ref<LaputaSnapshot | null>(null);
const loadingSnapshot = ref(false);
const loadingSection = ref(false);
const sectionError = ref('');
const sectionContent = ref<LaputaSection | null>(null);
const draftContent = ref('');
const originalContent = ref('');
const isDirty = ref(false);
const editorRevision = ref(0);

const displayName = computed(() => t('laputa.sections.' + selectedSection.value));

const selectedSectionStatus = computed(() =>
  snapshot.value?.sections[selectedSection.value]?.status ?? 'tbd',
);

const selectedSectionLastUpdated = computed(() =>
  snapshot.value?.sections[selectedSection.value]?.last_modified ?? undefined,
);

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

async function selectSection(nextId: LaputaSectionName): Promise<void> {
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
  isDirty.value = false;
  await loadSection(nextId);
}

async function onSectionSelect(name: LaputaSectionName): Promise<void> {
  await selectSection(name);
}

async function onRefresh(): Promise<void> {
  await loadSnapshot();
  if (isUninitialized.value) return;
  const validSections = snapshot.value?.sections ?? {};
  if (selectedSection.value in validSections) {
    await loadSection(selectedSection.value);
  } else {
    await selectSection('identity');
  }
}

async function onProposalCreated(
  _name: LaputaSectionName,
  result: import('../api/desktop').WriteLaputaSectionResult,
): Promise<void> {
  await loadSection(selectedSection.value);
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
  loadSection('identity');
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

    <div class="governance-layout">
      <!-- 树状图导航层（真实三层架构：Garden → Laputa / BML） -->
      <section class="governance-tree-section">
        <div class="governance-flow">
          <!-- 核心入口 -->
          <button 
            class="flow-node node-garden" 
            :class="{ active: activeNode === 'garden' }"
            @click="activeNode = 'garden'"
          >
            <div class="node-icon"><Network :size="20" /></div>
            <div class="node-content">
              <span class="node-title">{{ t('laputa.nodes.garden') }}</span>
              <span class="node-subtitle">Facade Workspace</span>
            </div>
          </button>

          <!-- 连接线与分支 -->
          <div class="flow-branches">
            <div class="branch-path path-laputa"></div>
            <div class="branch-path path-bml"></div>
          </div>

          <!-- 子节点群 -->
          <div class="flow-leaves">
            <button 
              class="flow-node node-laputa" 
              :class="{ active: activeNode === 'laputa' }"
              @click="activeNode = 'laputa'"
            >
              <div class="node-icon"><Sparkles :size="18" /></div>
              <div class="node-content">
                <span class="node-title">{{ t('laputa.nodes.laputa') }}</span>
                <span class="node-subtitle">Persona Governance</span>
              </div>
            </button>

            <button 
              class="flow-node node-bml" 
              :class="{ active: activeNode === 'bml' }"
              @click="emit('open-memory')"
            >
              <div class="node-icon"><Database :size="18" /></div>
              <div class="node-content">
                <span class="node-title">{{ t('laputa.nodes.bml') }}</span>
                <span class="node-subtitle">Memory Storage</span>
              </div>
            </button>
          </div>
        </div>
      </section>

      <!-- 详情编辑与占位层 -->
      <section class="governance-detail-section">
        <!-- Laputa (已实现) -->
        <div v-show="activeNode === 'laputa'" class="persona-memory-body">
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
              :on-retry="() => loadSection(selectedSection)"
            />

            <PersonaMemoryEmptyState
              v-else-if="isUninitialized"
              :icon="Inbox"
              :title="t('laputa.uninitializedTitle')"
              :description="t('laputa.uninitializedDesc')"
            />

            <template v-else>
              <SectionEditor
                :key="`${selectedSection}-${editorRevision}`"
                v-model="draftContent"
                :section-name="selectedSection"
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

        <!-- 其他节点占位 (Garden facade 说明) -->
        <div v-show="activeNode !== 'laputa'" class="governance-placeholder-body">
          <div class="placeholder-card">
            <div class="placeholder-icon-wrapper">
              <Network v-if="activeNode === 'garden'" :size="48" class="text-blue-500" />
              <Database v-else :size="48" class="text-emerald-500" />
            </div>
            <h2 class="placeholder-title">{{ t('laputa.nodes.' + activeNode) }}</h2>
            <p class="placeholder-desc">{{ t('laputa.placeholder.' + activeNode + 'Desc') }}</p>
            <button
              v-if="activeNode === 'bml'"
              class="placeholder-cta"
              type="button"
              @click="emit('open-memory')"
            >
              {{ t('laputa.openMemoryWorkspace') }}
            </button>
          </div>
        </div>
      </section>
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

.governance-layout {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  background: var(--panel-muted, #f8fafc);
}

/* 上半部分：记忆治理树状图 */
.governance-tree-section {
  flex-shrink: 0;
  padding: 24px;
  border-bottom: 1px solid var(--line);
  background: var(--panel-solid);
  display: flex;
  justify-content: center;
  overflow-x: auto;
}

.governance-flow {
  display: flex;
  align-items: center;
  gap: 32px;
  position: relative;
}

.flow-node {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 20px;
  border-radius: 16px;
  background: var(--panel, #ffffff);
  border: 1px solid var(--line, #e2e8f0);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.03);
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  text-align: left;
  z-index: 2;
  min-width: 220px;
}

.flow-node:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.06);
  border-color: var(--brand, #ec4899);
}

.flow-node.active {
  background: linear-gradient(135deg, rgba(236, 72, 153, 0.05) 0%, rgba(139, 92, 246, 0.05) 100%);
  border-color: var(--brand, #ec4899);
  box-shadow: 0 0 0 1px var(--brand, #ec4899), 0 8px 24px rgba(236, 72, 153, 0.15);
}

.node-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: 10px;
  color: #fff;
  flex-shrink: 0;
}

.node-garden .node-icon { background: linear-gradient(135deg, #3b82f6, #2563eb); }
.node-laputa .node-icon { background: linear-gradient(135deg, #ec4899, #d946ef); }
.node-bml .node-icon { background: linear-gradient(135deg, #10b981, #059669); }

.node-content {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.node-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text, #0f172a);
}

.node-subtitle {
  font-size: 11px;
  color: var(--text-muted, #64748b);
}

/* 连接线系统 */
.flow-branches {
  position: relative;
  width: 40px;
  height: 180px;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.flow-branches::before {
  content: '';
  position: absolute;
  left: 0;
  top: 50%;
  width: 20px;
  height: 2px;
  background: var(--line, #cbd5e1);
  transform: translateY(-50%);
}

.branch-path {
  position: absolute;
  left: 20px;
  width: 20px;
  border: 2px solid var(--line, #cbd5e1);
  border-left: 0;
  border-radius: 0 8px 8px 0;
}

.path-laputa { top: 30px; bottom: 50%; border-bottom: 0; border-radius: 0 8px 0 0; }
.path-bml { top: 50%; bottom: 30px; border-top: 0; border-radius: 0 0 8px 0; }

.flow-leaves {
  display: flex;
  flex-direction: column;
  gap: 16px;
  position: relative;
}

/* 下半部分：具体视图 */
.governance-detail-section {
  flex: 1;
  min-height: 0;
  display: flex;
  position: relative;
}

/* Laputa 分栏设计复用 */
.persona-memory-body {
  display: flex;
  flex: 1;
  width: 100%;
  height: 100%;
  background: var(--panel, #ffffff);
}

/* 占位层设计 */
.governance-placeholder-body {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px;
  background: var(--panel-muted, #f8fafc);
}

.placeholder-card {
  max-width: 420px;
  text-align: center;
  background: var(--panel-solid, #ffffff);
  border: 1px solid var(--line, #e2e8f0);
  border-radius: 24px;
  padding: 48px 32px;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.04);
  display: flex;
  flex-direction: column;
  align-items: center;
}

.placeholder-icon-wrapper {
  width: 96px;
  height: 96px;
  border-radius: 24px;
  background: rgba(241, 245, 249, 0.8);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 24px;
  box-shadow: inset 0 2px 4px rgba(255, 255, 255, 0.5);
}

.placeholder-title {
  font-size: 20px;
  font-weight: 700;
  color: var(--text, #0f172a);
  margin-bottom: 12px;
}

.placeholder-desc {
  font-size: 14px;
  line-height: 1.6;
  color: var(--text-muted, #64748b);
  margin-bottom: 24px;
}

.placeholder-cta {
  display: inline-block;
  padding: 8px 20px;
  border-radius: 999px;
  background: linear-gradient(135deg, #10b981, #059669);
  color: #fff;
  font-size: 13px;
  font-weight: 600;
  border: none;
  cursor: pointer;
  transition: opacity 0.15s ease;
}

.placeholder-cta:hover {
  opacity: 0.9;
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

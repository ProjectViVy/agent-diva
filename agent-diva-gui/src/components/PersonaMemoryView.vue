<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { BookUser, Loader2, RefreshCw, Inbox } from 'lucide-vue-next';
import SectionGroupList from './persona-memory/SectionGroupList.vue';
import SectionEditor from './persona-memory/SectionEditor.vue';
import PersonaMemoryEmptyState from './persona-memory/PersonaMemoryEmptyState.vue';
import PersonaMemoryErrorState from './persona-memory/PersonaMemoryErrorState.vue';
import { getLaputaSnapshot, getLaputaSection, isTauriRuntime } from '../api/desktop';
import type { LaputaSection, LaputaSectionName } from '../api/desktop';
import { showAppToast } from '../utils/appToast';

const { t } = useI18n();

interface LaputaSnapshot {
  sections: Record<string, {
    status: 'owned' | 'tbd';
    last_modified?: string | null;
  }>;
}

const selectedSection = ref<LaputaSectionName>('identity');
const snapshot = ref<LaputaSnapshot | null>(null);
const loadingSnapshot = ref(false);
const loadingSection = ref(false);
const sectionError = ref('');
const sectionContent = ref<LaputaSection | null>(null);
const draftContent = ref('');

const displayName = computed(() => t('laputa.sections.' + selectedSection.value));

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
    draftContent.value = raw === null || raw === undefined ? '' : String(raw);
  } catch (err: unknown) {
    sectionError.value = normalizeError(err);
    showAppToast(t('laputa.loadError'), 'error');
  } finally {
    loadingSection.value = false;
  }
}

async function onSectionSelect(name: LaputaSectionName): Promise<void> {
  selectedSection.value = name;
  await loadSection(name);
}

async function onRefresh(): Promise<void> {
  await loadSnapshot();
  if (isUninitialized.value) return;
  const validSections = snapshot.value?.sections ?? {};
  if (selectedSection.value in validSections) {
    await loadSection(selectedSection.value);
  } else {
    selectedSection.value = 'identity';
    await loadSection('identity');
  }
}

async function onSaved(_name: LaputaSectionName): Promise<void> {
  await loadSection(selectedSection.value);
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
            :section-name="selectedSection"
            :display-name="displayName"
            :initial-content="draftContent"
            @saved="onSaved"
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

.persona-memory-body {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
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

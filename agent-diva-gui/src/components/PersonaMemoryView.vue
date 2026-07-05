<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { BookUser, RefreshCw, Loader2, AlertCircle } from 'lucide-vue-next';
import SectionGroupList from './persona-memory/SectionGroupList.vue';
import SectionEditor from './persona-memory/SectionEditor.vue';
import { getLaputaSnapshot, getLaputaSection, isTauriRuntime } from '../api/desktop';
import type { LaputaSection, LaputaSectionName } from '../api/desktop';
import { showAppToast } from '../utils/appToast';
import { appConfirm } from '../utils/appDialog';

const { t } = useI18n();

interface LaputaSnapshot {
  sections: Record<string, {
    status: 'owned' | 'tbd';
    last_modified?: string | null;
  }>;
}

const selectedSection = ref<LaputaSectionName>('identity');
const snapshot = ref<LaputaSnapshot | null>(null);
const loading = ref(false);
const error = ref('');
const isDirty = ref(false);
const sectionContent = ref<LaputaSection | null>(null);

function normalizeError(err: unknown): string {
  if (err && typeof err === 'object' && 'message' in err) {
    return String((err as { message: unknown }).message);
  }
  return err instanceof Error ? err.message : String(err);
}

async function loadSnapshot(): Promise<void> {
  loading.value = true;
  error.value = '';
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
    error.value = normalizeError(err);
    showAppToast(t('laputa.loadError'), 'error');
  } finally {
    loading.value = false;
  }
}

async function loadSection(name: LaputaSectionName): Promise<void> {
  loading.value = true;
  error.value = '';
  try {
    if (isTauriRuntime()) {
      sectionContent.value = await getLaputaSection(name);
    } else {
      sectionContent.value = null;
    }
    isDirty.value = false;
  } catch (err: unknown) {
    error.value = normalizeError(err);
    showAppToast(t('laputa.loadError'), 'error');
  } finally {
    loading.value = false;
  }
}

async function onSectionSelect(name: LaputaSectionName): Promise<void> {
  if (isDirty.value) {
    const confirmed = await appConfirm(t('laputa.confirmDiscard.message'), {
      title: t('laputa.confirmDiscard.title'),
      confirmLabel: t('laputa.confirmDiscard.discard'),
      cancelLabel: t('laputa.confirmDiscard.cancel'),
    });
    if (!confirmed) return;
  }
  selectedSection.value = name;
  await loadSection(name);
}

async function onRefresh(): Promise<void> {
  await loadSnapshot();
  const validSections = snapshot.value?.sections ?? {};
  if (selectedSection.value in validSections) {
    await loadSection(selectedSection.value);
  } else {
    selectedSection.value = 'identity';
    await loadSection('identity');
  }
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
        :disabled="loading"
        @click="onRefresh"
      >
        <Loader2 v-if="loading" :size="15" class="spin" />
        <RefreshCw v-else :size="15" />
        <span>{{ t('laputa.refresh') }}</span>
      </button>
    </header>

    <div class="persona-memory-body">
      <div class="persona-memory-list">
        <SectionGroupList
          :snapshot="snapshot"
          :selected-section="selectedSection"
          @select="onSectionSelect"
        />
      </div>

      <div class="persona-memory-detail">
        <div v-if="error" class="persona-memory-error" role="status">
          <AlertCircle :size="17" />
          <div>
            <strong>{{ t('laputa.loadError') }}</strong>
            <p>{{ error }}</p>
          </div>
          <button class="persona-memory-retry" type="button" @click="onRefresh">
            {{ t('laputa.retry') }}
          </button>
        </div>

        <SectionEditor
          :section-name="selectedSection"
          :section="sectionContent"
          :loading="loading"
          :error="error"
          @update:dirty="isDirty = $event"
          @refresh="loadSection(selectedSection)"
        />
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

.persona-memory-error {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  border-bottom: 1px solid var(--danger-bg);
  background: var(--danger-bg);
  padding: 12px 20px;
  color: var(--danger);
  font-size: 12px;
}

.persona-memory-error p {
  margin: 2px 0 0;
  overflow-wrap: anywhere;
}

.persona-memory-retry {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px;
  border: 1px solid var(--danger);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--danger);
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
}

.persona-memory-retry:hover {
  background: var(--danger-bg);
}

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { Database, Loader2, RefreshCw, Search, Trash2, X } from '@lucide/vue';
import {
  bmlGetMemory,
  bmlListMemories,
  bmlRemoveMemory,
  type BmlMemoryKind,
  type BmlStoredMemory,
} from '../../api/desktop';
import { showAppToast } from '../../utils/appToast';
import { appPrompt } from '../../utils/appDialog';

const { t } = useI18n();
const emit = defineEmits<{
  (e: 'open-approval', proposalId: string): void;
}>();

const memories = ref<BmlStoredMemory[]>([]);
const selected = ref<BmlStoredMemory | null>(null);
const query = ref('');
const kindFilter = ref<'all' | BmlMemoryKind>('all');
const loadingList = ref(false);
const loadingDetail = ref(false);
const removing = ref(false);
const error = ref('');
let searchTimer: ReturnType<typeof setTimeout> | null = null;

const kindOptions = computed(() => {
  const options: { value: 'all' | BmlMemoryKind; label: string }[] = [
    { value: 'all', label: t('memory.filterAll') },
  ];
  for (const kind of [
    'identity',
    'relationship',
    'commitment',
    'preference',
    'long_term',
    'history',
    'daily',
    'weekly',
    'monthly',
    'journal',
    'learning',
    'working_memory',
  ] as const) {
    options.push({ value: kind, label: t(`memory.kinds.${kind}`) });
  }
  return options;
});

function hasMessage(err: unknown): err is { message: unknown } {
  return err !== null && err !== undefined && typeof err === 'object' && 'message' in err;
}

function normalizeError(err: unknown): string {
  if (hasMessage(err)) return String(err.message);
  return err instanceof Error ? err.message : String(err);
}

function formatDate(value?: string | null): string {
  if (!value) return '—';
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString();
}

function kindLabel(kind: BmlMemoryKind): string {
  return t(`memory.kinds.${kind}` as Parameters<typeof t>[0]);
}

async function loadList(): Promise<void> {
  loadingList.value = true;
  error.value = '';
  try {
    const result = await bmlListMemories({
      query: query.value.trim() || undefined,
      kind: kindFilter.value === 'all' ? undefined : kindFilter.value,
    });
    memories.value = result.memories ?? [];
    if (selected.value) {
      const stillPresent = memories.value.some((entry) => entry.record.id === selected.value?.record.id);
      if (!stillPresent) selected.value = null;
    }
  } catch (err) {
    error.value = normalizeError(err);
  } finally {
    loadingList.value = false;
  }
}

async function selectMemory(id: string): Promise<void> {
  if (selected.value?.record.id === id) return;
  selected.value = null;
  loadingDetail.value = true;
  error.value = '';
  try {
    const result = await bmlGetMemory(id);
    selected.value = result.memory ?? null;
  } catch (err) {
    error.value = normalizeError(err);
  } finally {
    loadingDetail.value = false;
  }
}

async function onRemove(): Promise<void> {
  const memory = selected.value;
  if (!memory || removing.value) return;
  const reason = await appPrompt(t('memory.removePrompt.message'), {
    title: t('memory.removePrompt.title'),
    confirmLabel: t('memory.removePrompt.confirm'),
    cancelLabel: t('memory.removePrompt.cancel'),
    placeholder: t('memory.removePrompt.placeholder'),
  });
  if (!reason) return;
  removing.value = true;
  try {
    const result = await bmlRemoveMemory(memory.record.id, reason);
    const proposalId = result.proposal_id;
    memories.value = memories.value.filter((entry) => entry.record.id !== memory.record.id);
    selected.value = null;
    showAppToast(t('memory.removeProposalCreated'), 'success');
    emit('open-approval', proposalId);
  } catch (err) {
    showAppToast(normalizeError(err), 'error');
  } finally {
    removing.value = false;
  }
}

function onSearchInput(): void {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    void loadList();
  }, 300);
}

function clearQuery(): void {
  query.value = '';
  void loadList();
}

watch(kindFilter, () => {
  void loadList();
});

onMounted(() => {
  void loadList();
});
</script>

<template>
  <div class="memory-view">
    <header class="memory-view-header">
      <div class="memory-view-title-block">
        <Database :size="18" />
        <span>{{ t('memory.title') }}</span>
        <span class="memory-view-subtitle">{{ t('memory.subtitle') }}</span>
      </div>
      <button
        class="memory-view-refresh"
        type="button"
        :disabled="loadingList"
        @click="loadList"
      >
        <Loader2 v-if="loadingList" :size="15" class="spin" />
        <RefreshCw v-else :size="15" />
        <span>{{ t('memory.refresh') }}</span>
      </button>
    </header>

    <div class="memory-layout">
      <section class="memory-list-panel">
        <div class="memory-list-toolbar">
          <div class="memory-search-box">
            <Search :size="14" class="memory-search-icon" />
            <input
              v-model="query"
              class="memory-search-input"
              type="search"
              :placeholder="t('memory.searchPlaceholder')"
              :aria-label="t('memory.searchPlaceholder')"
              @input="onSearchInput"
            />
            <button
              v-if="query"
              class="memory-search-clear"
              type="button"
              :aria-label="t('memory.clearSearch')"
              @click="clearQuery"
            >
              <X :size="13" />
            </button>
          </div>
          <select
            v-model="kindFilter"
            class="memory-kind-select"
            :aria-label="t('memory.filterKind')"
          >
            <option v-for="option in kindOptions" :key="option.value" :value="option.value">
              {{ option.label }}
            </option>
          </select>
        </div>

        <div v-if="error" class="memory-list-error">{{ error }}</div>

        <div v-else-if="loadingList && memories.length === 0" class="memory-list-skeleton">
          <div v-for="i in 6" :key="i" class="memory-skeleton-item">
            <div class="skeleton-line short" />
            <div class="skeleton-line" />
            <div class="skeleton-line medium" />
          </div>
        </div>

        <div v-else-if="memories.length === 0" class="memory-list-empty">
          <Database :size="32" class="memory-list-empty-icon" />
          <p>{{ t('memory.emptyTitle') }}</p>
          <p class="memory-list-empty-desc">{{ t('memory.emptyDesc') }}</p>
        </div>

        <ul v-else class="memory-list">
          <li
            v-for="entry in memories"
            :key="entry.record.id"
            class="memory-list-item"
            :class="{ active: selected?.record.id === entry.record.id }"
            @click="selectMemory(entry.record.id)"
          >
            <div class="memory-list-item-top">
              <span class="memory-kind-badge" :class="`kind-${entry.record.kind}`">
                {{ kindLabel(entry.record.kind) }}
              </span>
              <span class="memory-list-item-time">{{ formatDate(entry.record.effective_at) }}</span>
            </div>
            <p class="memory-list-item-content">{{ entry.record.content }}</p>
          </li>
        </ul>
      </section>

      <section class="memory-detail-panel">
        <template v-if="loadingDetail">
          <div class="memory-detail-skeleton">
            <div class="skeleton-line title" />
            <div class="skeleton-line" />
            <div class="skeleton-line medium" />
            <div class="skeleton-line short" />
          </div>
        </template>

        <div v-else-if="!selected" class="memory-detail-placeholder">
          <Database :size="40" class="memory-detail-placeholder-icon" />
          <p>{{ t('memory.detailPlaceholder') }}</p>
        </div>

        <div v-else class="memory-detail">
          <div class="memory-detail-header">
            <div class="memory-detail-title-row">
              <span class="memory-kind-badge" :class="`kind-${selected.record.kind}`">
                {{ kindLabel(selected.record.kind) }}
              </span>
              <span class="memory-detail-id">{{ selected.record.id }}</span>
            </div>
            <button
              class="memory-remove-button"
              type="button"
              :disabled="removing"
              @click="onRemove"
            >
              <Loader2 v-if="removing" :size="14" class="spin" />
              <Trash2 v-else :size="14" />
              <span>{{ t('memory.remove') }}</span>
            </button>
          </div>

          <div class="memory-detail-content">{{ selected.record.content }}</div>

          <dl class="memory-meta-grid">
            <div class="memory-meta-row">
              <dt>{{ t('memory.meta.trust') }}</dt>
              <dd>{{ t(`memory.trusts.${selected.record.trust}`) }}</dd>
            </div>
            <div class="memory-meta-row">
              <dt>{{ t('memory.meta.sensitivity') }}</dt>
              <dd>{{ t(`memory.sensitivities.${selected.record.sensitivity}`) }}</dd>
            </div>
            <div class="memory-meta-row">
              <dt>{{ t('memory.meta.confidence') }}</dt>
              <dd>{{ (selected.record.confidence_bps / 100).toFixed(2) }}%</dd>
            </div>
            <div class="memory-meta-row">
              <dt>{{ t('memory.meta.source') }}</dt>
              <dd>{{ t(`memory.sources.${selected.record.provenance.source}`) }}</dd>
            </div>
            <div class="memory-meta-row">
              <dt>{{ t('memory.meta.sourceId') }}</dt>
              <dd>{{ selected.record.provenance.source_id }}</dd>
            </div>
            <div class="memory-meta-row">
              <dt>{{ t('memory.meta.capturedAt') }}</dt>
              <dd>{{ formatDate(selected.record.provenance.captured_at) }}</dd>
            </div>
            <div class="memory-meta-row">
              <dt>{{ t('memory.meta.effectiveAt') }}</dt>
              <dd>{{ formatDate(selected.record.effective_at) }}</dd>
            </div>
            <div class="memory-meta-row">
              <dt>{{ t('memory.meta.expiresAt') }}</dt>
              <dd>{{ formatDate(selected.record.expires_at ?? null) }}</dd>
            </div>
            <div class="memory-meta-row">
              <dt>{{ t('memory.meta.session') }}</dt>
              <dd>{{ selected.record.scope.session_id || '—' }}</dd>
            </div>
            <div class="memory-meta-row">
              <dt>{{ t('memory.meta.revision') }}</dt>
              <dd>{{ selected.revision }}</dd>
            </div>
          </dl>

          <div v-if="selected.record.evidence_refs.length > 0" class="memory-evidence">
            <h3>{{ t('memory.evidenceTitle') }}</h3>
            <ul>
              <li v-for="ref in selected.record.evidence_refs" :key="ref.id">
                <span class="memory-evidence-source">{{ ref.source }}</span>
                <span class="memory-evidence-uri">{{ ref.uri }}</span>
                <span v-if="ref.excerpt" class="memory-evidence-excerpt">{{ ref.excerpt }}</span>
              </li>
            </ul>
          </div>

          <div v-if="selected.record.supersedes.length > 0" class="memory-supersedes">
            <h3>{{ t('memory.supersedesTitle') }}</h3>
            <ul>
              <li v-for="target in selected.record.supersedes" :key="target">{{ target }}</li>
            </ul>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.memory-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--panel);
  border-radius: var(--radius);
  overflow: hidden;
}

.memory-view-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px;
  border-bottom: 1px solid var(--line);
  flex-shrink: 0;
  min-height: 56px;
}

.memory-view-title-block {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}

.memory-view-subtitle {
  font-size: 12px;
  font-weight: 400;
  color: var(--text-muted);
}

.memory-view-refresh {
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

.memory-view-refresh:hover:not(:disabled) {
  background: var(--accent-bg-light);
  border-color: var(--accent-border);
  color: var(--accent);
}

.memory-view-refresh:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.memory-layout {
  display: flex;
  flex: 1;
  min-height: 0;
}

.memory-list-panel {
  width: 300px;
  min-width: 300px;
  border-right: 1px solid var(--line);
  display: flex;
  flex-direction: column;
  background: var(--panel-muted, #f8fafc);
}

.memory-list-toolbar {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  border-bottom: 1px solid var(--line);
  background: var(--panel-solid);
}

.memory-search-box {
  position: relative;
  display: flex;
  align-items: center;
}

.memory-search-icon {
  position: absolute;
  left: 10px;
  color: var(--text-muted);
}

.memory-search-input {
  width: 100%;
  padding: 7px 30px 7px 30px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel);
  color: var(--text);
  font-size: 13px;
  outline: none;
}

.memory-search-input:focus {
  border-color: var(--accent-border);
  box-shadow: 0 0 0 2px var(--accent-glow);
}

.memory-search-clear {
  position: absolute;
  right: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  background: none;
  border: none;
  cursor: pointer;
  padding: 2px;
}

.memory-kind-select {
  padding: 6px 8px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel);
  color: var(--text);
  font-size: 12px;
  outline: none;
}

.memory-list {
  flex: 1;
  overflow-y: auto;
  list-style: none;
  margin: 0;
  padding: 8px;
}

.memory-list-item {
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.15s ease;
  margin-bottom: 4px;
  background: var(--panel-solid);
}

.memory-list-item:hover {
  border-color: var(--accent-border);
}

.memory-list-item.active {
  border-color: var(--accent);
  background: var(--accent-bg-light);
}

.memory-list-item-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 6px;
}

.memory-kind-badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  background: var(--accent-bg-light);
  color: var(--accent);
  white-space: nowrap;
}

.kind-working_memory {
  background: #fef3c7;
  color: #92400e;
}

.kind-identity,
.kind-relationship,
.kind-commitment,
.kind-preference {
  background: #ede9fe;
  color: #6d28d9;
}

.kind-long_term,
.kind-learning {
  background: #dcfce7;
  color: #166534;
}

.memory-list-item-time {
  font-size: 11px;
  color: var(--text-muted);
  white-space: nowrap;
}

.memory-list-item-content {
  margin: 0;
  font-size: 13px;
  line-height: 1.5;
  color: var(--text);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.memory-list-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 32px;
  color: var(--text-muted);
  font-size: 13px;
  text-align: center;
}

.memory-list-empty-icon {
  opacity: 0.5;
}

.memory-list-empty-desc {
  font-size: 12px;
  opacity: 0.8;
  margin: 0;
}

.memory-list-error {
  padding: 12px;
  font-size: 12px;
  color: #b91c1c;
  background: #fef2f2;
}

.memory-list-skeleton {
  flex: 1;
  padding: 12px;
}

.memory-skeleton-item {
  padding: 12px;
  margin-bottom: 8px;
  background: var(--panel-solid);
  border-radius: var(--radius-sm);
}

.memory-detail-panel {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  background: var(--panel);
  position: relative;
}

.memory-detail-placeholder {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--text-muted);
  font-size: 13px;
}

.memory-detail-placeholder-icon {
  opacity: 0.5;
}

.memory-detail {
  padding: 20px 24px;
}

.memory-detail-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 16px;
}

.memory-detail-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.memory-detail-id {
  font-size: 12px;
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.memory-remove-button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: 1px solid #fecaca;
  border-radius: var(--radius-sm);
  background: #fef2f2;
  color: #b91c1c;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.memory-remove-button:hover:not(:disabled) {
  background: #fee2e2;
  border-color: #f87171;
}

.memory-remove-button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.memory-detail-content {
  padding: 14px 16px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-muted, #f8fafc);
  font-size: 14px;
  line-height: 1.7;
  color: var(--text);
  white-space: pre-wrap;
  word-break: break-word;
  margin-bottom: 20px;
}

.memory-meta-grid {
  margin: 0 0 20px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  overflow: hidden;
}

.memory-meta-row {
  display: flex;
  padding: 8px 12px;
  border-bottom: 1px solid var(--line);
  font-size: 12px;
}

.memory-meta-row:last-child {
  border-bottom: none;
}

.memory-meta-row dt {
  width: 130px;
  flex-shrink: 0;
  color: var(--text-muted);
  font-weight: 500;
}

.memory-meta-row dd {
  margin: 0;
  color: var(--text);
  word-break: break-all;
}

.memory-evidence,
.memory-supersedes {
  margin-bottom: 20px;
}

.memory-evidence h3,
.memory-supersedes h3 {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  margin: 0 0 8px;
}

.memory-evidence ul,
.memory-supersedes ul {
  list-style: none;
  margin: 0;
  padding: 0;
}

.memory-evidence li {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 12px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  margin-bottom: 6px;
  font-size: 12px;
}

.memory-evidence-source {
  font-weight: 600;
  color: var(--accent);
}

.memory-evidence-uri {
  color: var(--text-muted);
  word-break: break-all;
}

.memory-evidence-excerpt {
  color: var(--text);
  opacity: 0.85;
}

.memory-supersedes li {
  padding: 6px 12px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  margin-bottom: 6px;
  font-size: 12px;
  color: var(--text-muted);
  word-break: break-all;
}

.memory-detail-skeleton {
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

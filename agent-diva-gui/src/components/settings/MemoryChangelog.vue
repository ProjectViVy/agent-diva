<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { History, LoaderCircle, ChevronLeft, ChevronRight } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';

const { t } = useI18n();

interface ChangelogEntry {
  timestamp: string;
  source: string;
  summary: string;
  change_type: string;
}

interface ChangelogPage {
  entries: ChangelogEntry[];
  total: number;
  page: number;
  limit: number;
}

const loading = ref(true);
const loadError = ref<string | null>(null);
const entries = ref<ChangelogEntry[]>([]);
const currentPage = ref(1);
const totalEntries = ref(0);
const limit = 20;

const totalPages = computed(() => Math.max(1, Math.ceil(totalEntries.value / limit)));

const loadPage = async (page: number) => {
  loading.value = true;
  loadError.value = null;
  try {
    const data = await invoke<ChangelogPage | ChangelogEntry[]>('get_memory_changelog', {
      page,
      limit,
    });
    if (Array.isArray(data)) {
      entries.value = data;
      totalEntries.value = data.length;
      currentPage.value = 1;
    } else {
      entries.value = data.entries ?? [];
      totalEntries.value = data.total ?? 0;
      currentPage.value = data.page ?? page;
    }
  } catch (e) {
    loadError.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
};

const goToPage = (page: number) => {
  if (page < 1 || page > totalPages.value) return;
  void loadPage(page);
};

const formatDate = (ts: string): string => {
  try {
    const d = new Date(ts);
    if (isNaN(d.getTime())) return ts;
    return d.toLocaleString();
  } catch {
    return ts;
  }
};

onMounted(() => void loadPage(1));
</script>

<template>
  <div class="p-6 space-y-6 fade-in">
    <!-- Header -->
    <div class="flex items-center space-x-3">
      <div class="settings-dashboard-icon">
        <History :size="20" />
      </div>
      <div>
        <h3 class="settings-dashboard-title">{{ t('memoryChangelog.title') }}</h3>
        <p class="settings-dashboard-desc">{{ t('memoryChangelog.desc') }}</p>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center py-12">
      <LoaderCircle :size="24" class="animate-spin" :style="{ color: 'var(--accent)' }" />
      <span class="ml-3 settings-muted">{{ t('memoryChangelog.loading') }}</span>
    </div>

    <!-- Error -->
    <div v-else-if="loadError" class="settings-section" :style="{ borderColor: 'var(--danger)' }">
      <p class="text-sm" :style="{ color: 'var(--danger)' }">{{ t('memoryChangelog.loadError') }}: {{ loadError }}</p>
      <button type="button" class="settings-btn settings-btn-secondary mt-3" @click="loadPage(currentPage)">
        {{ t('memoryChangelog.retry') }}
      </button>
    </div>

    <!-- Empty -->
    <div v-else-if="entries.length === 0" class="settings-section">
      <div class="flex flex-col items-center justify-center py-8">
        <History :size="40" class="opacity-20 mb-3" />
        <p class="settings-muted text-sm">{{ t('memoryChangelog.empty') }}</p>
      </div>
    </div>

    <!-- Timeline -->
    <template v-else>
      <div class="memory-timeline">
        <div
          v-for="(entry, idx) in entries"
          :key="idx"
          class="memory-timeline-item"
        >
          <!-- Dot + Line -->
          <div class="memory-timeline-track">
            <div class="memory-timeline-dot" />
            <div v-if="idx < entries.length - 1" class="memory-timeline-line" />
          </div>

          <!-- Content -->
          <div class="memory-timeline-content">
            <div class="memory-timeline-meta">
              <span class="memory-timeline-date">{{ formatDate(entry.timestamp) }}</span>
              <span class="memory-timeline-source">{{ entry.source }}</span>
              <span
                class="memory-timeline-type"
                :style="{
                  background: 'var(--accent-bg-light)',
                  color: 'var(--accent)',
                }"
              >
                {{ entry.change_type }}
              </span>
            </div>
            <p class="memory-timeline-summary settings-label">{{ entry.summary }}</p>
          </div>
        </div>
      </div>

      <!-- Pagination -->
      <div v-if="totalPages > 1" class="memory-pagination">
        <button
          type="button"
          class="settings-btn settings-btn-secondary memory-page-btn"
          :disabled="currentPage <= 1"
          @click="goToPage(currentPage - 1)"
        >
          <ChevronLeft :size="16" />
        </button>
        <span class="settings-muted text-sm">
          {{ t('memoryChangelog.pageInfo', { current: currentPage, total: totalPages }) }}
        </span>
        <button
          type="button"
          class="settings-btn settings-btn-secondary memory-page-btn"
          :disabled="currentPage >= totalPages"
          @click="goToPage(currentPage + 1)"
        >
          <ChevronRight :size="16" />
        </button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.fade-in {
  animation: slideIn 0.3s ease-out;
}

@keyframes slideIn {
  from { opacity: 0; transform: translateX(20px); }
  to { opacity: 1; transform: translateX(0); }
}

.memory-timeline {
  display: flex;
  flex-direction: column;
  gap: 0;
}

.memory-timeline-item {
  display: flex;
  gap: 1rem;
  min-height: 4rem;
}

.memory-timeline-track {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 16px;
  flex-shrink: 0;
  padding-top: 6px;
}

.memory-timeline-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--accent);
  flex-shrink: 0;
  box-shadow: 0 0 0 3px var(--accent-glow);
}

.memory-timeline-line {
  width: 2px;
  flex: 1;
  background: var(--line);
  margin-top: 4px;
  margin-bottom: -4px;
}

.memory-timeline-content {
  flex: 1;
  min-width: 0;
  padding-bottom: 1.25rem;
}

.memory-timeline-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.25rem;
}

.memory-timeline-date {
  font-size: 0.75rem;
  font-family: monospace;
  color: var(--text-muted);
}

.memory-timeline-source {
  font-size: 0.7rem;
  padding: 0.125rem 0.5rem;
  border-radius: 9999px;
  background: var(--panel);
  border: 1px solid var(--line);
  color: var(--text-muted);
}

.memory-timeline-type {
  font-size: 0.65rem;
  padding: 0.125rem 0.5rem;
  border-radius: 9999px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.memory-timeline-summary {
  font-size: 0.875rem;
  line-height: 1.5;
}

.memory-pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  padding-top: 0.5rem;
}

.memory-page-btn {
  padding: 0.375rem 0.5rem;
}
</style>

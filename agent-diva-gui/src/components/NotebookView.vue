<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import MarkdownIt from 'markdown-it';
import hljs from 'highlight.js';
import 'highlight.js/styles/github-dark.css';
import {
  BookOpen,
  Calendar,
  FileText,
  RefreshCw,
  ShieldCheck,
  Brain,
  StickyNote,
  Loader2,
  AlertCircle,
  Inbox,
} from 'lucide-vue-next';
import { invoke } from '@tauri-apps/api/core';
import { appConfirm } from '../utils/appDialog';
import { showAppToast } from '../utils/appToast';

const { t } = useI18n();

// --- Types ---
type ReportPeriod = 'daily' | 'weekly' | 'monthly';

interface NotebookReport {
  id: string;
  date: string;      // ISO date string, e.g. "2026-06-01"
  title: string;
  summary: string;
  content: string;   // full markdown
}

// --- Markdown renderer ---
const md = new MarkdownIt({
  html: false,
  linkify: true,
  highlight(str: string, lang: string) {
    if (lang && hljs.getLanguage(lang)) {
      try {
        return hljs.highlight(str, { language: lang }).value;
      } catch {
        // fall through
      }
    }
    return '<pre><code>' + str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;') + '</code></pre>';
  },
});

// --- State ---
const activePeriod = ref<ReportPeriod>('weekly');
const reports = ref<NotebookReport[]>([]);
const selectedId = ref<string | null>(null);
const loading = ref(false);
const error = ref('');
const actionBusy = ref(false);

const pollIntervalMs = 60_000;
let pollHandle: ReturnType<typeof setInterval> | null = null;

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// --- Computed ---
const selectedReport = computed(() =>
  reports.value.find((r) => r.id === selectedId.value) ?? null,
);

const renderedContent = computed(() => {
  if (!selectedReport.value) return '';
  return md.render(selectedReport.value.content);
});

const periodTabs: { key: ReportPeriod; labelKey: string }[] = [
  { key: 'daily', labelKey: 'notebook.periodDaily' },
  { key: 'weekly', labelKey: 'notebook.periodWeekly' },
  { key: 'monthly', labelKey: 'notebook.periodMonthly' },
];

// --- Data fetching ---
async function fetchReports() {
  loading.value = true;
  error.value = '';
  try {
    if (isTauri()) {
      const { invoke } = await import('@tauri-apps/api/core');
      reports.value = await invoke<NotebookReport[]>('get_notebook_reports', {
        period: activePeriod.value,
      });
    } else {
      // Browser preview: mock data
      reports.value = [];
    }
    // Auto-select first if nothing selected
    if (reports.value.length > 0 && !selectedId.value) {
      selectedId.value = reports.value[0].id;
    }
    // Clear selection if it no longer exists
    if (selectedId.value && !reports.value.find((r) => r.id === selectedId.value)) {
      selectedId.value = reports.value.length > 0 ? reports.value[0].id : null;
    }
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function switchPeriod(period: ReportPeriod) {
  if (activePeriod.value === period) return;
  activePeriod.value = period;
  selectedId.value = null;
  fetchReports();
}

function selectReport(id: string) {
  selectedId.value = id;
}

function retry() {
  fetchReports();
}

// --- Bottom bar actions ---
async function solidifyAsSop() {
  if (!selectedReport.value) return;
  const confirmed = await appConfirm(
    t('notebook.sopConfirmMessage', { title: selectedReport.value.title }),
    { title: t('notebook.sopConfirmTitle') },
  );
  if (!confirmed) return;
  actionBusy.value = true;
  try {
    if (isTauri()) {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('solidify_report_as_sop', { reportId: selectedReport.value.id });
    }
    showAppToast(t('notebook.sopSuccess'), 'success');
  } catch (e: unknown) {
    showAppToast(e instanceof Error ? e.message : String(e), 'error');
  } finally {
    actionBusy.value = false;
  }
}

async function solidifyAsSkill() {
  if (!selectedReport.value) return;
  const confirmed = await appConfirm(
    t('notebook.skillConfirmMessage', { title: selectedReport.value.title }),
    { title: t('notebook.skillConfirmTitle') },
  );
  if (!confirmed) return;
  actionBusy.value = true;
  try {
    if (isTauri()) {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('solidify_report_as_skill', { reportId: selectedReport.value.id });
    }
    showAppToast(t('notebook.skillSuccess'), 'success');
  } catch (e: unknown) {
    showAppToast(e instanceof Error ? e.message : String(e), 'error');
  } finally {
    actionBusy.value = false;
  }
}

async function updateLongTermMemory() {
  if (!selectedReport.value) return;
  const confirmed = await appConfirm(
    t('notebook.memoryConfirmMessage', { title: selectedReport.value.title }),
    { title: t('notebook.memoryConfirmTitle') },
  );
  if (!confirmed) return;
  actionBusy.value = true;
  try {
    if (isTauri()) {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('update_memory_from_report', { reportId: selectedReport.value.id });
    }
    showAppToast(t('notebook.memorySuccess'), 'success');
  } catch (e: unknown) {
    showAppToast(e instanceof Error ? e.message : String(e), 'error');
  } finally {
    actionBusy.value = false;
  }
}

// --- Truncation helper ---
function truncate(text: string, max: number): string {
  if (text.length <= max) return text;
  return text.slice(0, max) + '…';
}

// --- Lifecycle ---
onMounted(() => {
  fetchReports();
  // Poll for updates every 60s
  pollHandle = window.setInterval(fetchReports, 60_000);
});

onUnmounted(() => {
  if (pollHandle !== null) {
    clearInterval(pollHandle);
    pollHandle = null;
  }
});
</script>

<template>
  <div class="notebook-view">
    <!-- Header with period tabs -->
    <div class="notebook-header">
      <div class="notebook-header-title">
        <BookOpen :size="18" />
        <span>{{ t('notebook.title') }}</span>
      </div>
      <div class="notebook-tabs">
        <button
          v-for="tab in periodTabs"
          :key="tab.key"
          class="notebook-tab"
          :class="{ active: activePeriod === tab.key }"
          @click="switchPeriod(tab.key)"
        >
          {{ t(tab.labelKey) }}
        </button>
      </div>
    </div>

    <!-- Main content: dual-panel -->
    <div class="notebook-body">
      <!-- Left panel: report list -->
      <div class="notebook-list">
        <!-- Loading skeleton -->
        <template v-if="loading">
          <div v-for="i in 4" :key="i" class="notebook-skeleton-item">
            <div class="skeleton-line short" />
            <div class="skeleton-line long" />
            <div class="skeleton-line medium" />
          </div>
        </template>

        <!-- Error state -->
        <div v-else-if="error" class="notebook-empty-state">
          <AlertCircle :size="32" class="notebook-empty-icon error" />
          <p class="notebook-empty-text">{{ t('notebook.loadError') }}</p>
          <button class="notebook-retry-btn" @click="retry">
            <RefreshCw :size="14" />
            {{ t('notebook.retry') }}
          </button>
        </div>

        <!-- Empty state -->
        <div v-else-if="reports.length === 0" class="notebook-empty-state">
          <Inbox :size="32" class="notebook-empty-icon" />
          <p class="notebook-empty-text">{{ t('notebook.empty', { period: t(`notebook.period${activePeriod === 'daily' ? 'Daily' : activePeriod === 'weekly' ? 'Weekly' : 'Monthly'}`) }) }}</p>
        </div>

        <!-- Report list -->
        <template v-else>
          <div
            v-for="report in reports"
            :key="report.id"
            class="notebook-list-item"
            :class="{ selected: selectedId === report.id }"
            @click="selectReport(report.id)"
          >
            <div class="notebook-list-date">
              <Calendar :size="12" />
              {{ report.date }}
            </div>
            <div class="notebook-list-title">{{ report.title }}</div>
            <div class="notebook-list-summary">{{ truncate(report.summary, 100) }}</div>
          </div>
        </template>
      </div>

      <!-- Right panel: report detail -->
      <div class="notebook-detail">
        <!-- Loading skeleton -->
        <template v-if="loading">
          <div class="notebook-detail-skeleton">
            <div class="skeleton-line title" />
            <div class="skeleton-line" />
            <div class="skeleton-line" />
            <div class="skeleton-line short" />
            <div class="skeleton-line" />
            <div class="skeleton-line medium" />
          </div>
        </template>

        <!-- No selection / empty -->
        <div v-else-if="!selectedReport" class="notebook-empty-state">
          <FileText :size="40" class="notebook-empty-icon" />
          <p class="notebook-empty-text">{{ t('notebook.selectReport') }}</p>
        </div>

        <!-- Report content -->
        <div v-else class="notebook-detail-content">
          <div class="notebook-detail-header">
            <h2 class="notebook-detail-title">{{ selectedReport.title }}</h2>
            <div class="notebook-detail-date">
              <Calendar :size="14" />
              {{ selectedReport.date }}
            </div>
          </div>
          <div class="notebook-markdown markdown-body" v-html="renderedContent" />
        </div>
      </div>
    </div>

    <!-- Bottom action bar -->
    <div v-if="selectedReport" class="notebook-actions">
      <button
        class="notebook-action-btn"
        :disabled="actionBusy"
        @click="solidifyAsSop"
      >
        <ShieldCheck :size="16" />
        <span>{{ t('notebook.solidifySop') }}</span>
      </button>
      <button
        class="notebook-action-btn"
        :disabled="actionBusy"
        @click="solidifyAsSkill"
      >
        <Brain :size="16" />
        <span>{{ t('notebook.solidifySkill') }}</span>
      </button>
      <button
        class="notebook-action-btn"
        :disabled="actionBusy"
        @click="updateLongTermMemory"
      >
        <StickyNote :size="16" />
        <span>{{ t('notebook.updateMemory') }}</span>
      </button>
      <div v-if="actionBusy" class="notebook-action-loading">
        <Loader2 :size="16" class="spin" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.notebook-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--panel);
  border-radius: var(--radius);
  overflow: hidden;
}

/* ===== Header ===== */
.notebook-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px;
  border-bottom: 1px solid var(--line);
  flex-shrink: 0;
}

.notebook-header-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}

.notebook-tabs {
  display: flex;
  gap: 4px;
  background: var(--accent-bg-light);
  border-radius: 8px;
  padding: 3px;
}

.notebook-tab {
  padding: 6px 16px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-muted);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.notebook-tab:hover {
  color: var(--text);
}

.notebook-tab.active {
  background: var(--panel-solid);
  color: var(--accent);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

/* ===== Body (dual-panel) ===== */
.notebook-body {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

/* ===== Left panel: list ===== */
.notebook-list {
  width: 280px;
  min-width: 280px;
  border-right: 1px solid var(--line);
  overflow-y: auto;
  padding: 8px;
  flex-shrink: 0;
}

.notebook-list-item {
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  border-left: 4px solid transparent;
  transition: all 0.15s;
  margin-bottom: 2px;
}

.notebook-list-item:hover {
  background: var(--accent-bg-light);
}

.notebook-list-item.selected {
  border-left-color: var(--accent);
  background: var(--accent-bg-light);
}

.notebook-list-date {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: var(--text-muted);
  margin-bottom: 4px;
}

.notebook-list-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--text);
  margin-bottom: 4px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.notebook-list-summary {
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* ===== Right panel: detail ===== */
.notebook-detail {
  flex: 1;
  overflow-y: auto;
  min-width: 0;
}

.notebook-detail-content {
  padding: 20px 24px;
}

.notebook-detail-header {
  margin-bottom: 20px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--line);
}

.notebook-detail-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--text);
  margin: 0 0 8px 0;
}

.notebook-detail-date {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--text-muted);
}

/* ===== Markdown body ===== */
.notebook-markdown {
  font-size: 0.875rem;
  line-height: 1.7;
  color: var(--text);
}

.notebook-markdown :deep(p) {
  margin-bottom: 0.75em;
}

.notebook-markdown :deep(p:last-child) {
  margin-bottom: 0;
}

.notebook-markdown :deep(pre) {
  background-color: #1e1e1e;
  border-radius: 0.375rem;
  padding: 0.75rem;
  margin: 0.75rem 0;
  overflow-x: auto;
}

.notebook-markdown :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 0.85em;
  background-color: rgba(0, 0, 0, 0.1);
  padding: 0.15em 0.35em;
  border-radius: 0.25rem;
}

.notebook-markdown :deep(pre code) {
  background-color: transparent;
  padding: 0;
  color: #e5e7eb;
}

.notebook-markdown :deep(ul),
.notebook-markdown :deep(ol) {
  padding-left: 1.5em;
  margin-bottom: 0.75em;
}

.notebook-markdown :deep(ul) {
  list-style-type: disc;
}

.notebook-markdown :deep(ol) {
  list-style-type: decimal;
}

.notebook-markdown :deep(blockquote) {
  border-left: 3px solid var(--line);
  padding-left: 0.75rem;
  color: var(--text-muted);
  margin: 0.75rem 0;
}

.notebook-markdown :deep(h1),
.notebook-markdown :deep(h2),
.notebook-markdown :deep(h3) {
  color: var(--text);
  margin-top: 1.25em;
  margin-bottom: 0.5em;
}

.notebook-markdown :deep(h1) { font-size: 1.5em; }
.notebook-markdown :deep(h2) { font-size: 1.25em; }
.notebook-markdown :deep(h3) { font-size: 1.1em; }

.notebook-markdown :deep(a) {
  color: var(--accent);
  text-decoration: underline;
}

.notebook-markdown :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 0.75rem 0;
}

.notebook-markdown :deep(th),
.notebook-markdown :deep(td) {
  border: 1px solid var(--line);
  padding: 6px 10px;
  text-align: left;
  font-size: 0.85em;
}

.notebook-markdown :deep(th) {
  background: var(--accent-bg-light);
  font-weight: 600;
}

/* ===== Empty / Loading states ===== */
.notebook-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  min-height: 200px;
  gap: 12px;
  padding: 24px;
  text-align: center;
}

.notebook-empty-icon {
  color: var(--text-muted);
  opacity: 0.5;
}

.notebook-empty-icon.error {
  color: var(--danger);
  opacity: 0.7;
}

.notebook-empty-text {
  font-size: 14px;
  color: var(--text-muted);
  margin: 0;
}

.notebook-retry-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 16px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
  color: var(--accent);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s;
}

.notebook-retry-btn:hover {
  background: var(--accent-bg-light);
  border-color: var(--accent-border);
}

/* Skeleton loading */
.notebook-skeleton-item {
  padding: 12px;
  margin-bottom: 4px;
}

.notebook-detail-skeleton {
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

/* ===== Bottom action bar ===== */
.notebook-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 20px;
  height: 56px;
  min-height: 56px;
  border-top: 1px solid var(--line);
  background: var(--panel-solid);
  flex-shrink: 0;
}

.notebook-action-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 7px 16px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
  color: var(--text);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.notebook-action-btn:hover:not(:disabled) {
  background: var(--accent-bg-light);
  border-color: var(--accent-border);
  color: var(--accent);
}

.notebook-action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.notebook-action-loading {
  display: flex;
  align-items: center;
  color: var(--accent);
}

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>

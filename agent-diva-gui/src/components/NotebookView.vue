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
  Loader2,
  AlertCircle,
  Inbox,
  Search,
  Link2,
  CheckSquare,
} from '@lucide/vue';
import { showAppToast } from '../utils/appToast';

const { t } = useI18n();

// --- Types ---
type ReportPeriod = 'daily' | 'weekly' | 'monthly';

interface NotebookReport {
  id: string;
  period: ReportPeriod;
  date: string;
  title: string;
  summary: string;
  content: string;
  generatedAt?: string | null;
  generatedBy?: string | null;
  schemaVersion?: string | null;
  generationMode?: string | null;
  coverageStatus?: string | null;
  sourcePath?: string;
  isTruncated?: boolean;
  originalLineCount?: number;
  displayedLineCount?: number;
}

function generationModeLabel(mode?: string | null): string | null {
  if (mode === 'llm_curated') return 'LLM 归纳';
  if (mode === 'deterministic_fallback') return '确定性降级';
  return null;
}

interface SessionSearchHit {
  session_id: string;
  timestamp: string;
  snippet: string;
  source_uri: string;
  hash: string;
  source: 'session';
  snippet_truncated: boolean;
  message_index: number;
}

interface SessionSearchDiagnostic {
  session_id?: string | null;
  source_uri: string;
  reason: string;
}

interface SessionSearchResponse {
  hits: SessionSearchHit[];
  diagnostics: SessionSearchDiagnostic[];
  scanned_files: number;
  skipped_files: number;
  total_response_bytes: number;
  file_limit_reached: boolean;
  result_limit_reached: boolean;
  byte_limit_reached: boolean;
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
const activePeriod = ref<ReportPeriod>('daily');
const reports = ref<NotebookReport[]>([]);
const selectedId = ref<string | null>(null);
const loading = ref(false);
const error = ref('');
const generationBusy = ref(false);
const sessionQuery = ref('');
const sessionSearchBusy = ref(false);
const sessionSearchError = ref('');
const sessionHits = ref<SessionSearchHit[]>([]);
const selectedSessionHitKeys = ref<string[]>([]);

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

const emptyActionLabel = computed(() => {
  if (activePeriod.value === 'daily') return t('notebook.generateDaily');
  if (activePeriod.value === 'weekly') return t('notebook.generateWeekly');
  return t('notebook.generateMonthly');
});

const periodLabel = computed(() =>
  t(
    `notebook.period${
      activePeriod.value === 'daily'
        ? 'Daily'
        : activePeriod.value === 'weekly'
          ? 'Weekly'
          : 'Monthly'
    }`,
  ),
);

const periodTabs: { key: ReportPeriod; labelKey: string }[] = [
  { key: 'daily', labelKey: 'notebook.periodDaily' },
  { key: 'weekly', labelKey: 'notebook.periodWeekly' },
  { key: 'monthly', labelKey: 'notebook.periodMonthly' },
];

const selectedSessionHits = computed(() =>
  sessionHits.value.filter((hit) => selectedSessionHitKeys.value.includes(sessionHitKey(hit))),
);

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

async function triggerReportGeneration() {
  generationBusy.value = true;
  try {
    if (isTauri()) {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('trigger_notebook_report_generation', {
        period: activePeriod.value,
      });
    }
    showAppToast(t('notebook.generateTriggered', { period: periodLabel.value }), 'success');
    await fetchReports();
  } catch (e: unknown) {
    showAppToast(e instanceof Error ? e.message : String(e), 'error');
  } finally {
    generationBusy.value = false;
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

function sessionHitKey(hit: SessionSearchHit) {
  return hit.hash;
}

function toggleSessionHit(hit: SessionSearchHit) {
  const key = sessionHitKey(hit);
  if (selectedSessionHitKeys.value.includes(key)) {
    selectedSessionHitKeys.value = selectedSessionHitKeys.value.filter((item) => item !== key);
  } else {
    selectedSessionHitKeys.value = [...selectedSessionHitKeys.value, key];
  }
}

function clearSessionEvidenceSelection() {
  selectedSessionHitKeys.value = [];
}

async function searchSessionEvidence() {
  sessionSearchBusy.value = true;
  sessionSearchError.value = '';
  try {
    if (isTauri()) {
      const { invoke } = await import('@tauri-apps/api/core');
      const response = await invoke<SessionSearchResponse>('search_notebook_session_evidence_command', {
        request: {
          query: sessionQuery.value,
          maxResults: 8,
          maxSnippetChars: 180,
          maxTotalBytes: 12_000,
        },
      });
      sessionHits.value = response.hits;
      const validKeys = new Set(response.hits.map(sessionHitKey));
      selectedSessionHitKeys.value = selectedSessionHitKeys.value.filter((key) => validKeys.has(key));
      if (response.diagnostics.length > 0) {
        showAppToast(t('notebook.sessionDiagnostics', { count: response.diagnostics.length }), 'error');
      }
    } else {
      sessionHits.value = [];
    }
  } catch (e: unknown) {
    sessionHits.value = [];
    sessionSearchError.value = e instanceof Error ? e.message : String(e);
  } finally {
    sessionSearchBusy.value = false;
  }
}

// --- Bottom bar actions ---
// --- Truncation helper ---
function truncate(text: string | undefined | null, max: number): string {
  if (!text) return '';
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
          <p class="notebook-empty-text">{{ t('notebook.empty', { period: periodLabel }) }}</p>
          <button
            class="notebook-retry-btn"
            :disabled="generationBusy"
            @click="triggerReportGeneration"
          >
            <Loader2 v-if="generationBusy" :size="14" class="spin" />
            <RefreshCw v-else :size="14" />
            {{ emptyActionLabel }}
          </button>
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
              <span
                v-if="generationModeLabel(selectedReport.generationMode)"
                class="notebook-generation-badge"
                :data-mode="selectedReport.generationMode || ''"
              >
                {{ generationModeLabel(selectedReport.generationMode) }}
              </span>
            </div>
          </div>
          <div v-if="selectedReport.isTruncated" class="notebook-truncated-banner">
            {{ t('notebook.truncatedNotice', {
              shown: selectedReport.displayedLineCount ?? 0,
              total: selectedReport.originalLineCount ?? 0,
            }) }}
          </div>
          <div class="notebook-markdown markdown-body" v-html="renderedContent" />

          <section class="notebook-session-evidence">
            <div class="notebook-session-evidence-header">
              <div class="notebook-session-evidence-title">
                <Link2 :size="15" />
                <span>{{ t('notebook.sessionEvidenceTitle') }}</span>
              </div>
              <div v-if="selectedSessionHits.length" class="notebook-session-selection-count">
                <CheckSquare :size="14" />
                <span>{{ t('notebook.sessionEvidenceSelected', { count: selectedSessionHits.length }) }}</span>
              </div>
            </div>

            <div class="notebook-session-evidence-search">
              <input
                v-model="sessionQuery"
                class="notebook-session-search-input"
                :placeholder="t('notebook.sessionEvidencePlaceholder')"
                @keydown.enter.prevent="searchSessionEvidence"
              />
              <button
                class="notebook-session-search-btn"
                :disabled="sessionSearchBusy || !sessionQuery.trim()"
                type="button"
                @click="searchSessionEvidence"
              >
                <Loader2 v-if="sessionSearchBusy" :size="14" class="spin" />
                <Search v-else :size="14" />
                <span>{{ t('notebook.sessionEvidenceSearch') }}</span>
              </button>
              <button
                v-if="selectedSessionHitKeys.length"
                class="notebook-session-clear-btn"
                type="button"
                @click="clearSessionEvidenceSelection"
              >
                {{ t('notebook.sessionEvidenceClear') }}
              </button>
            </div>

            <p v-if="sessionSearchError" class="notebook-session-error">{{ sessionSearchError }}</p>
            <p v-else-if="!sessionSearchBusy && sessionQuery.trim() && sessionHits.length === 0" class="notebook-session-empty">
              {{ t('notebook.sessionEvidenceEmpty') }}
            </p>

            <ul v-if="sessionHits.length" class="notebook-session-hit-list">
              <li
                v-for="hit in sessionHits"
                :key="sessionHitKey(hit)"
                class="notebook-session-hit"
                :class="{ selected: selectedSessionHitKeys.includes(sessionHitKey(hit)) }"
                @click="toggleSessionHit(hit)"
              >
                <div class="notebook-session-hit-meta">
                  <span>{{ hit.session_id }}</span>
                  <span>{{ hit.timestamp }}</span>
                </div>
                <div class="notebook-session-hit-snippet">{{ hit.snippet }}</div>
              </li>
            </ul>
          </section>
        </div>
      </div>
    </div>

    <!-- Bottom action bar -->
    <div v-if="selectedReport" class="notebook-actions">
      <button
        class="notebook-regenerate-btn"
        :disabled="generationBusy"
        @click="triggerReportGeneration"
      >
        <Loader2 v-if="generationBusy" :size="16" class="spin" />
        <RefreshCw v-else :size="16" />
        <span>{{ t('notebook.regenerate', { period: periodLabel }) }}</span>
      </button>
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

.notebook-generation-badge {
  margin-left: 4px;
  padding: 1px 8px;
  border-radius: 999px;
  font-size: 11px;
  line-height: 1.4;
  border: 1px solid var(--border, rgba(255, 255, 255, 0.12));
  color: var(--text-muted);
}

.notebook-generation-badge[data-mode='llm_curated'] {
  color: #7dd3a7;
  border-color: rgba(125, 211, 167, 0.35);
}

.notebook-generation-badge[data-mode='deterministic_fallback'] {
  color: #f0c674;
  border-color: rgba(240, 198, 116, 0.35);
}

/* ===== Markdown body ===== */
.notebook-markdown {
  font-size: 0.875rem;
  line-height: 1.7;
  color: var(--text);
}

.notebook-truncated-banner {
  margin-bottom: 16px;
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--accent-border);
  background: var(--accent-bg-light);
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1.5;
}

.notebook-session-evidence {
  margin-top: 24px;
  padding-top: 20px;
  border-top: 1px solid var(--line);
}

.notebook-session-evidence-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}

.notebook-session-evidence-title,
.notebook-session-selection-count {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--text);
  font-size: 13px;
  font-weight: 600;
}

.notebook-session-selection-count {
  color: var(--accent);
}

.notebook-session-evidence-search {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
  margin-bottom: 10px;
}

.notebook-session-search-input {
  flex: 1;
  min-width: 220px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
  color: var(--text);
  padding: 8px 10px;
  font-size: 13px;
}

.notebook-session-search-btn,
.notebook-session-clear-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
  color: var(--text);
  padding: 8px 12px;
  font-size: 13px;
  cursor: pointer;
}

.notebook-session-search-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.notebook-session-hit-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.notebook-session-hit {
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  padding: 10px 12px;
  cursor: pointer;
  transition: all 0.15s ease;
  background: var(--panel-solid);
}

.notebook-session-hit.selected {
  border-color: var(--accent-border);
  background: var(--accent-bg-light);
}

.notebook-session-hit-meta {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  font-size: 12px;
  color: var(--text-muted);
  margin-bottom: 6px;
}

.notebook-session-hit-snippet,
.notebook-session-error,
.notebook-session-empty {
  font-size: 13px;
  line-height: 1.5;
  color: var(--text-muted);
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

.notebook-action-btn,
.notebook-regenerate-btn {
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

.notebook-action-btn:hover:not(:disabled),
.notebook-regenerate-btn:hover:not(:disabled) {
  background: var(--accent-bg-light);
  border-color: var(--accent-border);
  color: var(--accent);
}

.notebook-action-btn:disabled,
.notebook-regenerate-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.notebook-action-loading {
  display: flex;
  align-items: center;
  color: var(--accent);
}

.notebook-preview-backdrop {
  position: fixed;
  inset: 0;
  z-index: 140;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: rgba(15, 23, 42, 0.42);
}

.notebook-preview {
  width: min(680px, 100%);
  max-height: min(760px, calc(100vh - 48px));
  overflow-y: auto;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--panel-solid);
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.22);
}

.notebook-preview-header,
.notebook-preview-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 16px 18px;
  border-bottom: 1px solid var(--line);
}

.notebook-preview-actions {
  justify-content: flex-end;
  border-top: 1px solid var(--line);
  border-bottom: 0;
}

.notebook-preview-kicker {
  display: block;
  margin-bottom: 3px;
  color: var(--text-muted);
  font-size: 12px;
}

.notebook-preview-header h3,
.notebook-preview-section h4 {
  margin: 0;
  color: var(--text);
}

.notebook-preview-header h3 {
  font-size: 16px;
  font-weight: 600;
}

.notebook-preview-close {
  width: 32px;
  height: 32px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 20px;
  line-height: 1;
}

.notebook-preview-close:hover:not(:disabled) {
  color: var(--text);
  background: var(--accent-bg-light);
}

.notebook-preview-attention {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin: 14px 18px 0;
  padding: 10px 12px;
  border: 1px solid var(--warning-border, var(--accent-border));
  border-radius: var(--radius-sm);
  background: var(--warning-bg, var(--accent-bg-light));
  color: var(--text);
  font-size: 13px;
}

.notebook-preview-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
  padding: 16px 18px 4px;
  margin: 0;
}

.notebook-preview-grid div {
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel);
}

.notebook-preview-grid dt,
.notebook-preview-section h4 {
  color: var(--text-muted);
  font-size: 12px;
  font-weight: 600;
}

.notebook-preview-grid dd {
  margin: 4px 0 0;
  color: var(--text);
  font-size: 13px;
  overflow-wrap: anywhere;
}

.notebook-preview-section {
  padding: 12px 18px;
}

.notebook-preview-section h4 {
  margin-bottom: 8px;
}

.notebook-preview-section p {
  margin: 0;
  color: var(--text);
  font-size: 13px;
  line-height: 1.5;
}

.notebook-preview-section ul {
  margin: 0;
  padding: 0;
  list-style: none;
}

.notebook-preview-section li {
  display: flex;
  gap: 8px;
  align-items: baseline;
  padding: 8px 0;
  border-top: 1px solid var(--line);
  color: var(--text-muted);
  font-size: 12px;
}

.notebook-preview-section li:first-child {
  border-top: 0;
}

.notebook-preview-section li span {
  overflow-wrap: anywhere;
}

.notebook-preview-primary,
.notebook-preview-secondary {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 34px;
  padding: 7px 14px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
}

.notebook-preview-primary {
  border: 1px solid var(--accent-border);
  background: var(--accent);
  color: var(--accent-contrast, #fff);
}

.notebook-preview-secondary {
  border: 1px solid var(--line);
  background: var(--panel-solid);
  color: var(--text);
}

.notebook-preview-primary:disabled,
.notebook-preview-secondary:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.spin {
  animation: spin 1s linear infinite;
}

@media (max-width: 720px) {
  .notebook-preview-grid {
    grid-template-columns: 1fr;
  }

  .notebook-actions {
    height: auto;
    min-height: 56px;
    flex-wrap: wrap;
    padding: 10px 12px;
  }
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>

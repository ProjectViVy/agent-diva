<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { RefreshCcw, ScrollText, ChevronDown, ChevronUp } from 'lucide-vue-next';

const { t } = useI18n();

const props = defineProps<{
  logLines: string[];
  loading?: boolean;
  error?: string;
  maxLines?: number;
}>();

const emit = defineEmits<{
  (e: 'refresh'): void;
  (e: 'update:maxLines', value: number): void;
}>();

// Log level filter state - all enabled by default
const logLevelFilter = ref<Set<string>>(new Set(['ERROR', 'WARN', 'INFO', 'DEBUG']));
const logScrollRef = ref<HTMLDivElement | null>(null);
const logPinnedToBottom = ref(true);
const expandedLines = ref<Set<number>>(new Set());

const LOG_LEVELS = ['ERROR', 'WARN', 'INFO', 'DEBUG'] as const;
const LINE_COUNT_OPTIONS = [100, 200, 500, 1000] as const;

const LOG_LEVEL_COLORS: Record<string, string> = {
  ERROR: 'text-rose-400 bg-rose-500/10',
  WARN: 'text-amber-400 bg-amber-500/10',
  INFO: 'text-sky-400 bg-sky-500/10',
  DEBUG: 'text-slate-400 bg-slate-500/10',
};

const LOG_LEVEL_DOT_COLORS: Record<string, string> = {
  ERROR: 'bg-rose-500',
  WARN: 'bg-amber-500',
  INFO: 'bg-sky-500',
  DEBUG: 'bg-slate-500',
};

function toggleLogLevel(level: string) {
  const next = new Set(logLevelFilter.value);
  if (next.has(level)) {
    next.delete(level);
  } else {
    next.add(level);
  }
  logLevelFilter.value = next;
}

function isLevelActive(level: string): boolean {
  return logLevelFilter.value.has(level);
}

// Security: strip ANSI escape codes
function stripAnsi(str: string): string {
  return str.replace(/\x1b\[[\d;?]*[A-Za-z]/g, '').replace(/\r/g, '');
}

// Security: HTML escape for XSS prevention
function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

// Security: redact sensitive information
function redactSensitive(line: string): string {
  return line
    .replace(/api[_-]?key[=:]\s*\S+/gi, 'api_key=***REDACTED***')
    .replace(/token[=:]\s*\S+/gi, 'token=***REDACTED***')
    .replace(/password[=:]\s*\S+/gi, 'password=***REDACTED***')
    .replace(/secret[=:]\s*\S+/gi, 'secret=***REDACTED***')
    .replace(/bearer\s+\S+/gi, 'bearer ***REDACTED***');
}

// Detect log level from line content
function detectLogLevel(line: string): string {
  if (/\b(ERROR|CRITICAL|FATAL)\b/i.test(line)) return 'ERROR';
  if (/\b(WARN|WARNING)\b/i.test(line)) return 'WARN';
  if (/\bINFO\b/i.test(line)) return 'INFO';
  if (/\b(DEBUG|TRACE)\b/i.test(line)) return 'DEBUG';
  return 'INFO';
}

// Sanitize and prepare log line for display
function sanitizeLine(line: string): string {
  const stripped = stripAnsi(line);
  const redacted = redactSensitive(stripped);
  return escapeHtml(redacted);
}

// Highlight log line with syntax coloring
function highlightLine(line: string): string {
  const sanitized = sanitizeLine(line);

  // Highlight timestamp: YYYY-MM-DD HH:mm:ss,SSS or similar
  let highlighted = sanitized.replace(
    /^(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}[,.]\d+)/,
    '<span class="logTimestamp">$1</span>'
  );

  // Highlight log level
  highlighted = highlighted.replace(
    /\b(ERROR|WARN|INFO|DEBUG)\b/,
    '<span class="logLevel logLevel--$1">$1</span>'
  );

  // Highlight module name (e.g., [gateway::server] or gateway::server - )
  highlighted = highlighted.replace(
    /\[([a-zA-Z_:]+)\]/g,
    '<span class="logModule">[$1]</span>'
  );

  return highlighted;
}

// Check if line is too long (for truncation)
function isLongLine(line: string): boolean {
  return line.length > 300;
}

// Toggle line expansion
function toggleLineExpansion(index: number, event: Event) {
  event.stopPropagation();
  const next = new Set(expandedLines.value);
  if (next.has(index)) {
    next.delete(index);
  } else {
    next.add(index);
  }
  expandedLines.value = next;
}

function isLineExpanded(index: number): boolean {
  return expandedLines.value.has(index);
}

// Filter log lines based on level filter
const filteredLogLines = computed(() => {
  return props.logLines
    .map((line, index) => ({ line, originalIndex: index }))
    .filter(({ line }) => {
      const level = detectLogLevel(line);
      return logLevelFilter.value.has(level);
    });
});

// Scroll handling
const LOG_PIN_PX = 56;

function onLogScroll() {
  const el = logScrollRef.value;
  if (!el) return;
  const fromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
  logPinnedToBottom.value = fromBottom <= LOG_PIN_PX;
}

watch(
  () => props.logLines,
  async () => {
    await nextTick();
    if (!logPinnedToBottom.value) return;
    const el = logScrollRef.value;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  },
  { deep: true }
);

function scrollToBottom() {
  const el = logScrollRef.value;
  if (el) {
    el.scrollTop = el.scrollHeight;
    logPinnedToBottom.value = true;
  }
}

function updateMaxLines(value: number) {
  emit('update:maxLines', value);
}

function refresh() {
  emit('refresh');
}

// Expose scrollToBottom for parent components
defineExpose({
  scrollToBottom,
});
</script>

<template>
  <div class="log-panel space-y-3">
    <!-- Header with filter bar -->
    <div class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
      <div class="flex items-center gap-3">
        <!-- Log level filter badges -->
        <div class="flex items-center gap-1.5">
          <span class="text-xs text-slate-500 mr-1">{{ t('console.logLevelFilter') }}:</span>
          <button
            v-for="level in LOG_LEVELS"
            :key="level"
            type="button"
            class="logFilterBadge transition-all"
            :class="[
              isLevelActive(level) ? LOG_LEVEL_COLORS[level] : 'bg-slate-700/50 text-slate-500',
              isLevelActive(level) && 'logFilterBadge--active'
            ]"
            @click="toggleLogLevel(level)"
          >
            <span
              class="w-2 h-2 rounded-full mr-1.5"
              :class="LOG_LEVEL_DOT_COLORS[level]"
            />
            {{ level }}
          </button>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <!-- Auto-scroll indicator -->
        <button
          type="button"
          class="log-action-btn"
          :class="logPinnedToBottom ? 'log-action-btn--active' : 'log-action-btn--inactive'"
          @click="scrollToBottom"
        >
          <ChevronDown :size="12" />
          {{ t('console.autoScroll') }}
        </button>

        <!-- Line count selector -->
        <select
          :value="maxLines || 200"
          class="log-select"
          @change="updateMaxLines(Number(($event.target as HTMLSelectElement).value))"
        >
          <option v-for="count in LINE_COUNT_OPTIONS" :key="count" :value="count">
            {{ count }} {{ t('console.lineCount') }}
          </option>
        </select>

        <!-- Refresh button -->
        <button
          type="button"
          class="log-refresh-btn"
          :disabled="loading"
          @click="refresh"
        >
          <RefreshCcw :size="12" :class="loading ? 'animate-spin' : ''" />
          {{ t('console.refreshLogs') }}
        </button>
      </div>
    </div>

    <!-- Log container -->
    <div
      ref="logScrollRef"
      class="logContainer"
      @scroll.passive="onLogScroll"
    >
      <!-- Empty state -->
      <div v-if="filteredLogLines.length === 0" class="logEmpty">
        <ScrollText :size="32" class="text-slate-600 mb-2" />
        <p>{{ t('console.noLogs') }}</p>
      </div>

      <!-- Log lines -->
      <div v-else class="logContent">
        <template v-for="({ line, originalIndex }, _displayIndex) in filteredLogLines" :key="`${originalIndex}-${_displayIndex}`">
          <div
            class="logLine group"
            :class="`logLine--${detectLogLevel(line)}`"
          >
            <div
              class="logLineContent"
              :class="{ 'logLineContent--truncated': !isLineExpanded(originalIndex) && isLongLine(line) }"
              v-html="highlightLine(line)"
            />
            <!-- Expand/collapse button for long lines -->
            <button
              v-if="isLongLine(line)"
              type="button"
              class="logLineExpand opacity-0 group-hover:opacity-100 transition-opacity"
              @click="toggleLineExpansion(originalIndex, $event)"
            >
              <ChevronDown v-if="!isLineExpanded(originalIndex)" :size="12" />
              <ChevronUp v-else :size="12" />
            </button>
          </div>
        </template>
      </div>
    </div>

    <!-- Error message -->
    <p v-if="error" class="text-xs text-rose-400 break-words">
      {{ error }}
    </p>
  </div>
</template>

<style scoped>
.log-panel {
  @apply flex flex-col;
}

.logFilterBadge {
  @apply inline-flex items-center rounded-full px-2.5 py-1 text-xs font-medium;
}

.logContainer {
  @apply rounded-xl border border-slate-800 bg-slate-950 min-h-[280px] max-h-[480px] overflow-auto;
}

.logEmpty {
  @apply flex flex-col items-center justify-center px-6 py-12 text-sm text-slate-500;
}

.logContent {
  @apply px-4 py-3;
}

.logLine {
  @apply relative flex items-start gap-2 py-0.5 text-xs leading-5 font-mono;
}

.logLineContent {
  @apply flex-1 whitespace-pre-wrap break-words;
}

.logLineContent--truncated {
  @apply line-clamp-2;
}

.logLineExpand {
  @apply absolute right-2 top-1 p-1 rounded bg-slate-800 text-slate-400 hover:text-slate-200;
}

/* Log action buttons */
.log-action-btn {
  @apply inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-xs transition;
}

.log-action-btn--active {
  background: rgba(16, 185, 129, 0.15);
  color: #10b981;
}

.log-action-btn--inactive {
  background: rgba(100, 116, 139, 0.3);
  color: #94a3b8;
}

.log-select {
  border-radius: 0.5rem;
  border: 1px solid #334155;
  background: #1e293b;
  padding: 0.375rem 0.625rem;
  font-size: 0.75rem;
  color: #cbd5e1;
}

.log-select:focus {
  outline: none;
  border-color: #60a5fa;
}

.log-refresh-btn {
  @apply inline-flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs transition;
  border: 1px solid #334155;
  background: #1e293b;
  color: #cbd5e1;
}

.log-refresh-btn:hover:not(:disabled) {
  background: #334155;
}

.log-refresh-btn:disabled {
  opacity: 0.6;
}

/* Highlight classes - using deep selectors for v-html content */
:deep(.logTimestamp) {
  @apply text-slate-500;
}

:deep(.logLevel) {
  @apply px-1.5 py-0.5 rounded font-semibold;
}

:deep(.logLevel--ERROR) {
  @apply text-rose-400 bg-rose-500/10;
}

:deep(.logLevel--WARN) {
  @apply text-amber-400 bg-amber-500/10;
}

:deep(.logLevel--INFO) {
  @apply text-sky-400 bg-sky-500/10;
}

:deep(.logLevel--DEBUG) {
  @apply text-slate-400 bg-slate-500/10;
}

:deep(.logModule) {
  @apply text-violet-400;
}

/* Log line level-specific styling */
.logLine--ERROR {
  @apply bg-rose-500/5 -mx-4 px-4;
}

.logLine--WARN {
  @apply bg-amber-500/5 -mx-4 px-4;
}
</style>
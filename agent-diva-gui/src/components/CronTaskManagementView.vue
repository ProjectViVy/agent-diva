<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  Pause,
  Play,
  PlayCircle,
  Square,
  Plus,
  Pencil,
  Trash2,
  RefreshCw,
  Search,
  CalendarClock,
  Clock,
  Repeat,
  AlertCircle,
} from '@lucide/vue';
import { appConfirm } from '../utils/appDialog';
import { listCronJobs } from '../api/settings';
import { vivyClient } from '../api/vivy/instance';
import type { CronJobWriteParams, VivyCronJob, VivyCronSchedule } from '../api/vivy/contracts';

const { t } = useI18n();

type ScheduleKind = 'at' | 'every' | 'cron';
type CronStatus = 'running' | 'scheduled' | 'paused' | 'completed' | 'failed';

type CronJobDto = VivyCronJob;
type CronSchedule = VivyCronSchedule;

const isTauri = () => typeof window !== 'undefined' && (window as any)._wails?.environment != null;
const loading = ref(false);
const error = ref('');
const showForm = ref(false);
const submitting = ref(false);
const editingJobId = ref<string | null>(null);
const jobs = ref<CronJobDto[]>([]);
const actionBusy = reactive<Record<string, boolean>>({});
let pollHandle: number | null = null;

// Tab filter state
type TabFilter = 'active' | 'scheduled' | 'all';
const activeTab = ref<TabFilter>('active');
const searchQuery = ref('');

const form = reactive({
  name: '',
  scheduleKind: 'every' as ScheduleKind,
  atInput: '',
  everySeconds: 300,
  cronExpr: '0 9 * * *',
  timezone: Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC',
  message: '',
  deliver: true,
  channel: 'api',
  to: 'gui',
  deleteAfterRun: false,
  enabled: true,
});

// Status helpers
const ACTIVE_STATUSES = new Set<CronStatus>(['running']);
const SCHEDULED_STATUSES = new Set<CronStatus>(['scheduled', 'paused']);

function isInStatus(job: CronJobDto, statusSet: Set<CronStatus>): boolean {
  return statusSet.has(job.computedStatus as CronStatus);
}

// Filtered jobs
const activeJobs = computed(() =>
  jobs.value.filter((j) => isInStatus(j, ACTIVE_STATUSES)),
);
const scheduledJobs = computed(() =>
  jobs.value.filter((j) => isInStatus(j, SCHEDULED_STATUSES)),
);
const allJobs = computed(() => jobs.value);

const filteredJobs = computed(() => {
  let base: CronJobDto[];
  switch (activeTab.value) {
    case 'active':
      base = activeJobs.value;
      break;
    case 'scheduled':
      base = scheduledJobs.value;
      break;
    case 'all':
    default:
      base = allJobs.value;
      break;
  }
  if (!searchQuery.value.trim()) return base;
  const q = searchQuery.value.toLowerCase();
  return base.filter((j) => j.name.toLowerCase().includes(q));
});

const pausedCount = computed(() => jobs.value.filter((job) => job.computedStatus === 'paused').length);
const failedCount = computed(() => jobs.value.filter((job) => job.computedStatus === 'failed').length);

function formatTime(value?: number | null): string {
  if (!value) return t('cron.notScheduled');
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? t('cron.notScheduled') : date.toLocaleString();
}

function formatDuration(startedAt?: number | null): string {
  if (!startedAt) return t('cron.notRunning');
  const delta = Math.max(0, Date.now() - startedAt);
  const seconds = Math.floor(delta / 1000);
  const minutes = Math.floor(seconds / 60);
  if (minutes > 0) return t('cron.runningForMinutes', { count: minutes });
  return t('cron.runningForSeconds', { count: seconds });
}

function describeSchedule(schedule: CronSchedule): string {
  if (schedule.kind === 'at') return `${t('cron.scheduleAt')} ${formatTime(schedule.atMs)}`;
  if (schedule.kind === 'every') return t('cron.scheduleEverySeconds', { count: Math.floor((schedule.everyMs || 0) / 1000) });
  return schedule.tz ? `${schedule.expr} (${schedule.tz})` : schedule.expr || '';
}

function getTriggerBadge(schedule: CronSchedule): { label: string; icon: any } {
  if (schedule.kind === 'at') return { label: t('cron.schedule.at'), icon: CalendarClock };
  if (schedule.kind === 'every') return { label: t('cron.schedule.every'), icon: Repeat };
  return { label: t('cron.schedule.cron'), icon: Clock };
}

function statusClass(status: string): string {
  switch (status) {
    case 'running':
      return 'bg-success-soft text-success';
    case 'paused':
      return 'bg-warning-soft text-warning';
    case 'failed':
      return 'bg-destructive-soft text-destructive';
    case 'completed':
      return 'bg-accent text-primary';
    default:
      return 'bg-secondary text-foreground';
  }
}

function statusDotClass(status: string): string {
  switch (status) {
    case 'running':
      return 'bg-success';
    case 'scheduled':
      return 'bg-info';
    case 'paused':
      return 'bg-warning';
    case 'failed':
      return 'bg-destructive';
    case 'completed':
      return 'bg-secondary';
    default:
      return 'bg-secondary';
  }
}

function buildPayload(): CronJobWriteParams {
  const schedule =
    form.scheduleKind === 'at'
      ? { kind: 'at', atMs: new Date(form.atInput).getTime() }
      : form.scheduleKind === 'every'
        ? { kind: 'every', everyMs: Math.max(1, form.everySeconds) * 1000 }
        : { kind: 'cron', expr: form.cronExpr.trim(), tz: form.timezone.trim() || undefined };

  return {
    name: form.name.trim(),
    schedule,
    payload: {
      kind: 'agent_turn',
      message: form.message,
      deliver: form.deliver,
      channel: form.channel.trim() || undefined,
      to: form.to.trim() || undefined,
    },
    delete_after_run: form.deleteAfterRun,
    enabled: form.enabled,
  };
}

function resetForm() {
  editingJobId.value = null;
  form.name = '';
  form.scheduleKind = 'every';
  form.atInput = '';
  form.everySeconds = 300;
  form.cronExpr = '0 9 * * *';
  form.timezone = Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
  form.message = '';
  form.deliver = true;
  form.channel = 'api';
  form.to = 'gui';
  form.deleteAfterRun = false;
  form.enabled = true;
}

function openCreate() {
  resetForm();
  showForm.value = true;
}

function openEdit(job: CronJobDto) {
  editingJobId.value = job.id;
  form.name = job.name;
  form.scheduleKind = job.schedule.kind as ScheduleKind;
  form.atInput = job.schedule.atMs ? new Date(job.schedule.atMs).toISOString().slice(0, 16) : '';
  form.everySeconds = Math.max(1, Math.floor((job.schedule.everyMs || 0) / 1000) || 300);
  form.cronExpr = job.schedule.expr || '0 9 * * *';
  form.timezone = job.schedule.tz || Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
  form.message = job.payload.message;
  form.deliver = job.payload.deliver;
  form.channel = job.payload.channel || 'api';
  form.to = job.payload.to || 'gui';
  form.deleteAfterRun = job.deleteAfterRun;
  form.enabled = job.enabled;
  showForm.value = true;
}

async function fetchJobs() {
  if (!isTauri()) return;
  loading.value = true;
  error.value = '';
  try {
    jobs.value = await listCronJobs();
  } catch (err) {
    error.value = String(err);
  } finally {
    loading.value = false;
  }
}

async function submitForm() {
  submitting.value = true;
  error.value = '';
  try {
    const payload = buildPayload();
    if (editingJobId.value) {
      await vivyClient.cronUpdate(editingJobId.value, payload);
    } else {
      await vivyClient.cronCreate(payload);
    }
    showForm.value = false;
    resetForm();
    await fetchJobs();
  } catch (err) {
    error.value = String(err);
  } finally {
    submitting.value = false;
  }
}

async function withBusy(jobId: string, task: () => Promise<void>) {
  actionBusy[jobId] = true;
  try {
    await task();
  } catch (err) {
    error.value = String(err);
  } finally {
    actionBusy[jobId] = false;
  }
}

async function toggleJob(job: CronJobDto) {
  await withBusy(job.id, async () => {
    // cron/update is full-replace: resend the whole job with enabled flipped.
    await vivyClient.cronUpdate(job.id, {
      name: job.name,
      enabled: !job.enabled,
      schedule: job.schedule,
      payload: job.payload,
      delete_after_run: job.deleteAfterRun,
    });
    await fetchJobs();
  });
}

async function runJob(job: CronJobDto) {
  await withBusy(job.id, async () => {
    await vivyClient.cronTrigger(job.id);
    await fetchJobs();
  });
}

async function stopJob(job: CronJobDto) {
  await withBusy(job.id, async () => {
    await vivyClient.cronStop(job.id);
    await fetchJobs();
  });
}

async function deleteJob(job: CronJobDto) {
  const message = job.isRunning
    ? t('cron.confirmDeleteRunning', { name: job.name })
    : t('cron.confirmDelete', { name: job.name });
  if (!(await appConfirm(message))) return;
  await withBusy(job.id, async () => {
    await vivyClient.cronDelete(job.id);
    await fetchJobs();
  });
}

onMounted(async () => {
  await fetchJobs();
  if (isTauri()) {
    pollHandle = window.setInterval(() => {
      void fetchJobs();
    }, 5000);
  }
});

onUnmounted(() => {
  if (pollHandle !== null) {
    window.clearInterval(pollHandle);
  }
});
</script>

<template>
  <div class="cron-view h-full min-h-0 overflow-y-auto p-6">
    <div class="mx-auto max-w-5xl space-y-6">
      <!-- Header with gradient -->
      <section class="rounded-2xl border border-border bg-card px-5 py-4 text-foreground shadow-sm">
        <div class="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
          <div class="min-w-0 flex-1">
            <h2 class="text-[30px] font-bold leading-none tracking-tight">{{ t('cron.title') }}</h2>
          </div>
          <div class="flex shrink-0 flex-wrap gap-2 md:justify-end">
            <button
              class="ui-button ui-button--outline"
              @click="fetchJobs"
            >
              <span class="inline-flex items-center gap-1.5"><RefreshCw :size="14" />{{ t('cron.refresh') }}</span>
            </button>
            <button
              class="ui-button ui-button--primary"
              @click="openCreate"
            >
              <span class="inline-flex items-center gap-1.5"><Plus :size="14" />{{ t('cron.newTask') }}</span>
            </button>
          </div>
        </div>
        <div class="mt-4 grid gap-2 md:grid-cols-4">
          <div class="rounded-lg border border-border bg-background px-4 py-3">
            <div class="text-xs font-medium leading-none text-muted-foreground">{{ t('cron.totalTasks') }}</div>
            <div class="mt-1 text-xl font-semibold leading-none">{{ jobs.length }}</div>
          </div>
          <div class="rounded-lg border border-border bg-background px-4 py-3">
            <div class="text-xs font-medium leading-none text-muted-foreground">{{ t('cron.runningTasks') }}</div>
            <div class="mt-1 text-xl font-semibold leading-none">{{ activeJobs.length }}</div>
          </div>
          <div class="rounded-lg border border-border bg-background px-4 py-3">
            <div class="text-xs font-medium leading-none text-muted-foreground">{{ t('cron.pausedTasks') }}</div>
            <div class="mt-1 text-xl font-semibold leading-none">{{ pausedCount }}</div>
          </div>
          <div class="rounded-lg border border-border bg-background px-4 py-3">
            <div class="text-xs font-medium leading-none text-muted-foreground">{{ t('cron.failedTasks') }}</div>
            <div class="mt-1 text-xl font-semibold leading-none">{{ failedCount }}</div>
          </div>
        </div>
      </section>

      <!-- Error banner -->
      <div v-if="error" class="rounded-2xl border border-destructive bg-destructive-soft px-4 py-3 text-sm text-destructive">
        {{ error }}
      </div>

      <!-- Inline form panel (replaces modal) -->
      <section v-if="showForm" class="rounded-xl border border-border bg-card p-5 shadow-sm">
        <div class="flex items-center justify-between">
          <h3 class="text-lg font-semibold text-foreground">
            {{ editingJobId ? t('cron.editTask') : t('cron.newTask') }}
          </h3>
          <button class="ui-button ui-button--ghost " @click="showForm = false">
            {{ t('cron.cancel') }}
          </button>
        </div>
        <div class="mt-4 grid gap-4 md:grid-cols-2">
          <label class="text-sm text-foreground">
            <div class="mb-2 font-medium">{{ t('cron.taskName') }}</div>
            <input v-model="form.name" class="ui-input w-full" />
          </label>
          <label class="text-sm text-foreground">
            <div class="mb-2 font-medium">{{ t('cron.scheduleType') }}</div>
            <select v-model="form.scheduleKind" class="ui-input w-full">
              <option value="at">{{ t('cron.schedule.at') }}</option>
              <option value="every">{{ t('cron.schedule.every') }}</option>
              <option value="cron">{{ t('cron.schedule.cron') }}</option>
            </select>
          </label>
          <label v-if="form.scheduleKind === 'at'" class="text-sm text-foreground">
            <div class="mb-2 font-medium">{{ t('cron.runAt') }}</div>
            <input v-model="form.atInput" type="datetime-local" class="ui-input w-full" />
          </label>
          <label v-if="form.scheduleKind === 'every'" class="text-sm text-foreground">
            <div class="mb-2 font-medium">{{ t('cron.everySeconds') }}</div>
            <input v-model.number="form.everySeconds" type="number" min="1" class="ui-input w-full" />
          </label>
          <label v-if="form.scheduleKind === 'cron'" class="text-sm text-foreground">
            <div class="mb-2 font-medium">{{ t('cron.cronExpr') }}</div>
            <input v-model="form.cronExpr" class="ui-input w-full" />
          </label>
          <label v-if="form.scheduleKind === 'cron'" class="text-sm text-foreground">
            <div class="mb-2 font-medium">{{ t('cron.timezone') }}</div>
            <input v-model="form.timezone" class="ui-input w-full" />
          </label>
        </div>
        <label class="mt-4 block text-sm text-foreground">
          <div class="mb-2 font-medium">{{ t('cron.message') }}</div>
          <textarea v-model="form.message" rows="4" class="ui-input w-full min-h-24" />
        </label>
        <div class="mt-4 grid gap-4 md:grid-cols-2">
          <label class="text-sm text-foreground">
            <div class="mb-2 font-medium">{{ t('cron.channel') }}</div>
            <input v-model="form.channel" class="ui-input w-full" />
          </label>
          <label class="text-sm text-foreground">
            <div class="mb-2 font-medium">{{ t('cron.recipient') }}</div>
            <input v-model="form.to" class="ui-input w-full" />
          </label>
        </div>
        <div class="mt-4 flex flex-wrap gap-4 text-sm text-foreground">
          <label class="inline-flex items-center gap-2">
            <input v-model="form.deliver" type="checkbox" />
            {{ t('cron.deliver') }}
          </label>
          <label class="inline-flex items-center gap-2">
            <input v-model="form.enabled" type="checkbox" />
            {{ t('cron.enabled') }}
          </label>
          <label class="inline-flex items-center gap-2">
            <input v-model="form.deleteAfterRun" type="checkbox" />
            {{ t('cron.deleteAfterRun') }}
          </label>
        </div>
        <div class="mt-5 flex justify-end gap-3">
          <button class="ui-button ui-button--ghost " @click="showForm = false">
            {{ t('cron.cancel') }}
          </button>
          <button class="ui-button ui-button--primary " :disabled="submitting" @click="submitForm">
            {{ submitting ? t('cron.saving') : t('cron.save') }}
          </button>
        </div>
      </section>

      <!-- Tab filter bar + search -->
      <section class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <div class="flex items-center gap-1 rounded-xl bg-card p-1 shadow-sm">
          <button
            class="ui-button ui-button--ghost transition"
            :class="activeTab === 'active' ? 'bg-success-soft text-success' : 'text-muted-foreground hover:bg-muted'"
            @click="activeTab = 'active'"
          >
            <span class="inline-flex items-center gap-1.5">
              <PlayCircle :size="14" />
              {{ t('cron.tabActive') || 'Active' }}
              <span class="ml-1 rounded-full bg-success-soft px-1.5 py-0.5 text-xs">{{ activeJobs.length }}</span>
            </span>
          </button>
          <button
            class="ui-button ui-button--ghost transition"
            :class="activeTab === 'scheduled' ? 'bg-info-soft text-info' : 'text-muted-foreground hover:bg-muted'"
            @click="activeTab = 'scheduled'"
          >
            <span class="inline-flex items-center gap-1.5">
              <Clock :size="14" />
              {{ t('cron.tabScheduled') || 'Scheduled' }}
              <span class="ml-1 rounded-full bg-info-soft px-1.5 py-0.5 text-xs">{{ scheduledJobs.length }}</span>
            </span>
          </button>
          <button
            class="ui-button ui-button--ghost transition"
            :class="activeTab === 'all' ? 'bg-secondary text-foreground' : 'text-muted-foreground hover:bg-muted'"
            @click="activeTab = 'all'"
          >
            <span class="inline-flex items-center gap-1.5">
              {{ t('cron.tabAll') || 'All' }}
              <span class="ml-1 rounded-full bg-secondary px-1.5 py-0.5 text-xs">{{ jobs.length }}</span>
            </span>
          </button>
        </div>
        <div class="relative">
          <Search :size="16" class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground" />
          <input
            v-model="searchQuery"
            type="text"
            class="ui-input ui-input--leading-icon md:w-64"
            :placeholder="t('cron.searchPlaceholder') || 'Search tasks...'"
          />
        </div>
      </section>

      <!-- Task list -->
      <section class="space-y-3">
        <!-- Loading state -->
        <div v-if="loading && filteredJobs.length === 0" class="flex flex-col items-center justify-center rounded-2xl bg-card px-6 py-12 text-muted-foreground">
          <RefreshCw :size="24" class="animate-spin text-muted-foreground" />
          <p class="mt-3 text-sm">{{ t('cron.loading') }}</p>
        </div>

        <!-- Empty state -->
        <div v-else-if="filteredJobs.length === 0 && !searchQuery && jobs.length === 0 && !error" class="flex flex-col items-center justify-center rounded-2xl border border-dashed border-border bg-card px-6 py-12 text-muted-foreground">
          <CalendarClock :size="40" class="text-muted-foreground" />
          <p class="mt-3 font-medium">{{ t('cron.emptyState') }}</p>
        </div>

        <div v-else-if="filteredJobs.length === 0 && !searchQuery && jobs.length > 0" class="flex flex-col items-center justify-center rounded-2xl border border-dashed border-border bg-card px-6 py-12 text-muted-foreground">
          <CalendarClock :size="40" class="text-muted-foreground" />
          <p class="mt-3 font-medium">{{ activeTab === 'active' ? t('cron.noRunningTasks') : t('cron.noScheduledTasks') }}</p>
        </div>

        <!-- No search results -->
        <div v-else-if="filteredJobs.length === 0 && searchQuery && !error" class="flex flex-col items-center justify-center rounded-2xl border border-dashed border-border bg-card px-6 py-12 text-muted-foreground">
          <Search :size="40" class="text-muted-foreground" />
          <p class="mt-3 font-medium">{{ t('cron.noSearchResults') || 'No tasks matching search' }}</p>
        </div>

        <!-- Job cards -->
        <article
          v-for="job in filteredJobs"
          :key="job.id"
          class="group rounded-2xl border border-border bg-card p-5 shadow-sm transition hover:shadow-md"
        >
          <!-- Header row -->
          <div class="flex items-start justify-between gap-4">
            <div class="flex items-start gap-3 min-w-0 flex-1">
              <!-- Status dot -->
              <div class="mt-1.5 h-2.5 w-2.5 shrink-0 rounded-full" :class="statusDotClass(job.computedStatus)" />
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-2">
                  <h4 class="text-base font-semibold text-foreground truncate">{{ job.name }}</h4>
                  <span class="rounded-full px-2.5 py-0.5 text-xs font-medium" :class="statusClass(job.computedStatus)">
                    {{ t(`cron.status.${job.computedStatus}`) }}
                  </span>
                  <span class="inline-flex items-center gap-1 rounded-md bg-secondary px-2 py-0.5 text-xs text-muted-foreground">
                    <component :is="getTriggerBadge(job.schedule).icon" :size="12" />
                    {{ getTriggerBadge(job.schedule).label }}
                  </span>
                </div>
                <p class="mt-1 text-sm text-muted-foreground">{{ describeSchedule(job.schedule) }}</p>
                <!-- Content preview -->
                <p v-if="job.payload.message" class="mt-1 line-clamp-1 text-xs text-muted-foreground">
                  {{ job.payload.message }}
                </p>
              </div>
            </div>
            <!-- Action buttons -->
            <div class="flex shrink-0 items-center gap-1">
              <button
                class="ui-button ui-button--ghost transition"
                :title="job.enabled ? t('cron.pause') : t('cron.enable')"
                :disabled="actionBusy[job.id]"
                @click="toggleJob(job)"
              >
                <Pause v-if="job.enabled" :size="16" />
                <Play v-else :size="16" />
              </button>
              <button
                class="ui-button ui-button--ghost transition  "
                :title="t('cron.runNow')"
                :disabled="actionBusy[job.id]"
                @click="runJob(job)"
              >
                <PlayCircle :size="16" />
              </button>
              <button
                v-if="job.isRunning"
                class="ui-button ui-button--destructive transition  "
                :title="t('cron.stop')"
                :disabled="actionBusy[job.id]"
                @click="stopJob(job)"
              >
                <Square :size="16" />
              </button>
              <button
                class="ui-button ui-button--ghost transition  "
                :title="t('cron.edit')"
                :disabled="actionBusy[job.id]"
                @click="openEdit(job)"
              >
                <Pencil :size="16" />
              </button>
              <button
                class="ui-button ui-button--destructive transition  "
                :title="t('cron.delete')"
                :disabled="actionBusy[job.id]"
                @click="deleteJob(job)"
              >
                <Trash2 :size="16" />
              </button>
            </div>
          </div>

          <!-- Details grid -->
          <div class="mt-3 grid gap-3 border-t border-border pt-3 text-xs text-muted-foreground md:grid-cols-4">
            <div>
              <span class="font-medium text-foreground">{{ t('cron.nextRun') }}</span>
              <p class="mt-0.5 text-muted-foreground">{{ formatTime(job.state.nextRunAtMs) }}</p>
            </div>
            <div>
              <span class="font-medium text-foreground">{{ t('cron.lastRun') }}</span>
              <p class="mt-0.5 text-muted-foreground">{{ formatTime(job.state.lastRunAtMs) }}</p>
            </div>
            <div>
              <span class="font-medium text-foreground">{{ t('cron.channel') }}</span>
              <p class="mt-0.5 text-muted-foreground">{{ job.payload.channel || '-' }}</p>
            </div>
            <div>
              <span class="font-medium text-foreground">{{ t('cron.recipient') }}</span>
              <p class="mt-0.5 text-muted-foreground">{{ job.payload.to || '-' }}</p>
            </div>
          </div>

          <!-- Running indicator -->
          <div v-if="job.isRunning" class="mt-3 rounded-xl bg-success-soft px-3 py-2 text-xs text-success">
            <span class="font-medium">{{ t('cron.runningFor') }}</span> {{ formatDuration(job.activeRun?.startedAtMs) }}
          </div>

          <!-- Error display -->
          <div v-if="job.state.lastError" class="mt-3 flex items-start gap-2 rounded-xl bg-destructive-soft px-3 py-2 text-xs text-destructive">
            <AlertCircle :size="14" class="mt-0.5 shrink-0" />
            <span>{{ job.state.lastError }}</span>
          </div>
        </article>
      </section>

      <!-- Status legend footer -->
      <section class="flex flex-wrap items-center justify-center gap-4 rounded-2xl bg-card px-4 py-3 text-xs text-muted-foreground shadow-sm">
        <span class="inline-flex items-center gap-1.5">
          <span class="h-2 w-2 rounded-full bg-success" />
          {{ t('cron.status.running') || 'Running' }}
        </span>
        <span class="inline-flex items-center gap-1.5">
          <span class="h-2 w-2 rounded-full bg-info" />
          {{ t('cron.status.scheduled') || 'Scheduled' }}
        </span>
        <span class="inline-flex items-center gap-1.5">
          <span class="h-2 w-2 rounded-full bg-warning" />
          {{ t('cron.status.paused') || 'Paused' }}
        </span>
        <span class="inline-flex items-center gap-1.5">
          <span class="h-2 w-2 rounded-full bg-destructive" />
          {{ t('cron.status.failed') || 'Failed' }}
        </span>
        <span class="inline-flex items-center gap-1.5">
          <span class="h-2 w-2 rounded-full bg-secondary" />
          {{ t('cron.status.completed') || 'Completed' }}
        </span>
      </section>
    </div>
  </div>
</template>

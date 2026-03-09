<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import { Pause, Play, PlayCircle, Square, Plus, Pencil, Trash2, RefreshCw } from 'lucide-vue-next';

const { t } = useI18n();

type ScheduleKind = 'at' | 'every' | 'cron';
type CronStatus = 'running' | 'scheduled' | 'paused' | 'completed' | 'failed';

interface CronSchedule {
  kind: ScheduleKind;
  atMs?: number;
  everyMs?: number;
  expr?: string;
  tz?: string | null;
}

interface CronPayload {
  kind: string;
  message: string;
  deliver: boolean;
  channel?: string | null;
  to?: string | null;
}

interface CronRunSnapshot {
  run_id: string;
  job_id: string;
  startedAtMs: number;
  lastHeartbeatAtMs: number;
  trigger: 'scheduled' | 'manual';
  cancelable: boolean;
}

interface CronJobDto {
  id: string;
  name: string;
  enabled: boolean;
  schedule: CronSchedule;
  payload: CronPayload;
  state: {
    nextRunAtMs?: number | null;
    lastRunAtMs?: number | null;
    lastStatus?: string | null;
    lastError?: string | null;
  };
  createdAtMs: number;
  updatedAtMs: number;
  deleteAfterRun: boolean;
  isRunning: boolean;
  activeRun?: CronRunSnapshot | null;
  computedStatus: CronStatus;
}

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
const loading = ref(false);
const error = ref('');
const showForm = ref(false);
const submitting = ref(false);
const editingJobId = ref<string | null>(null);
const jobs = ref<CronJobDto[]>([]);
const actionBusy = reactive<Record<string, boolean>>({});
let pollHandle: number | null = null;

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

const runningJobs = computed(() => jobs.value.filter((job) => job.isRunning));
const otherJobs = computed(() => jobs.value.filter((job) => !job.isRunning));
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

function statusClass(status: CronStatus): string {
  switch (status) {
    case 'running':
      return 'bg-emerald-100 text-emerald-700';
    case 'paused':
      return 'bg-amber-100 text-amber-700';
    case 'failed':
      return 'bg-rose-100 text-rose-700';
    case 'completed':
      return 'bg-sky-100 text-sky-700';
    default:
      return 'bg-slate-100 text-slate-700';
  }
}

function buildPayload() {
  const schedule =
    form.scheduleKind === 'at'
      ? { kind: 'at', atMs: new Date(form.atInput).getTime() }
      : form.scheduleKind === 'every'
        ? { kind: 'every', everyMs: Math.max(1, form.everySeconds) * 1000 }
        : { kind: 'cron', expr: form.cronExpr.trim(), tz: form.timezone.trim() || null };

  return {
    name: form.name.trim(),
    schedule,
    payload: {
      kind: 'agent_turn',
      message: form.message,
      deliver: form.deliver,
      channel: form.channel.trim() || null,
      to: form.to.trim() || null,
    },
    deleteAfterRun: form.deleteAfterRun,
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
  form.scheduleKind = job.schedule.kind;
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
    const result = await invoke<CronJobDto[]>('get_cron_jobs');
    jobs.value = Array.isArray(result) ? result : [];
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
      await invoke('update_cron_job', { jobId: editingJobId.value, payload });
    } else {
      await invoke('create_cron_job', { payload });
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
    await invoke('set_cron_job_enabled', { jobId: job.id, enabled: !job.enabled });
    await fetchJobs();
  });
}

async function runJob(job: CronJobDto) {
  await withBusy(job.id, async () => {
    await invoke('run_cron_job', { jobId: job.id, force: true });
    await fetchJobs();
  });
}

async function stopJob(job: CronJobDto) {
  await withBusy(job.id, async () => {
    await invoke('stop_cron_job_run', { jobId: job.id });
    await fetchJobs();
  });
}

async function deleteJob(job: CronJobDto) {
  const confirmed = window.confirm(
    job.isRunning ? t('cron.confirmDeleteRunning', { name: job.name }) : t('cron.confirmDelete', { name: job.name }),
  );
  if (!confirmed) return;
  await withBusy(job.id, async () => {
    await invoke('delete_cron_job', { jobId: job.id });
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
  <div class="h-full min-h-0 overflow-y-auto bg-slate-50/80 p-6">
    <div class="mx-auto max-w-7xl space-y-6">
      <section class="rounded-[28px] bg-gradient-to-r from-emerald-500 via-teal-500 to-cyan-500 px-5 py-4 text-white shadow-lg shadow-cyan-900/10">
        <div class="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
          <div class="min-w-0 flex-1 space-y-1.5">
            <p class="text-xs font-medium uppercase tracking-[0.12em] text-white/72">{{ t('cron.subtitle') }}</p>
            <h2 class="text-[30px] font-bold leading-none tracking-tight">{{ t('cron.title') }}</h2>
          </div>
          <div class="flex shrink-0 flex-wrap gap-2 md:justify-end">
            <button
              class="rounded-xl border border-white/28 bg-white/12 px-3.5 py-2 text-xs font-medium shadow-[inset_0_0_0_1px_rgba(255,255,255,0.08)] backdrop-blur transition hover:bg-white/20"
              @click="fetchJobs"
            >
              <span class="inline-flex items-center gap-1.5"><RefreshCw :size="14" />{{ t('cron.refresh') }}</span>
            </button>
            <button
              class="rounded-xl bg-white px-3.5 py-2 text-xs font-semibold text-emerald-700 transition hover:bg-emerald-50"
              @click="openCreate"
            >
              <span class="inline-flex items-center gap-1.5"><Plus :size="14" />{{ t('cron.newTask') }}</span>
            </button>
          </div>
        </div>
        <div class="mt-4 grid gap-2 md:grid-cols-4">
          <div class="rounded-2xl border border-white/12 bg-white/10 px-4 py-3">
            <div class="text-[11px] font-medium uppercase tracking-[0.12em] text-white/68">{{ t('cron.totalTasks') }}</div>
            <div class="mt-1 text-2xl font-semibold leading-none">{{ jobs.length }}</div>
          </div>
          <div class="rounded-2xl border border-white/12 bg-white/10 px-4 py-3">
            <div class="text-[11px] font-medium uppercase tracking-[0.12em] text-white/68">{{ t('cron.runningTasks') }}</div>
            <div class="mt-1 text-2xl font-semibold leading-none">{{ runningJobs.length }}</div>
          </div>
          <div class="rounded-2xl border border-white/12 bg-white/10 px-4 py-3">
            <div class="text-[11px] font-medium uppercase tracking-[0.12em] text-white/68">{{ t('cron.pausedTasks') }}</div>
            <div class="mt-1 text-2xl font-semibold leading-none">{{ pausedCount }}</div>
          </div>
          <div class="rounded-2xl border border-white/12 bg-white/10 px-4 py-3">
            <div class="text-[11px] font-medium uppercase tracking-[0.12em] text-white/68">{{ t('cron.failedTasks') }}</div>
            <div class="mt-1 text-2xl font-semibold leading-none">{{ failedCount }}</div>
          </div>
        </div>
      </section>

      <div v-if="error" class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
        {{ error }}
      </div>

      <section class="space-y-4">
        <div class="flex items-center justify-between">
          <h3 class="text-lg font-semibold text-slate-800">{{ t('cron.runningSection') }}</h3>
          <span class="text-sm text-slate-500">{{ runningJobs.length }}</span>
        </div>
        <div v-if="runningJobs.length > 0" class="grid gap-4 xl:grid-cols-2">
          <article
            v-for="job in runningJobs"
            :key="job.id"
            class="rounded-3xl border border-emerald-100 bg-white p-5 shadow-sm"
          >
            <div class="flex items-start justify-between gap-4">
              <div>
                <div class="flex items-center gap-3">
                  <h4 class="text-xl font-semibold text-slate-900">{{ job.name }}</h4>
                  <span class="rounded-full px-3 py-1 text-xs font-medium" :class="statusClass(job.computedStatus)">
                    {{ t(`cron.status.${job.computedStatus}`) }}
                  </span>
                </div>
                <p class="mt-2 text-sm text-slate-600">{{ describeSchedule(job.schedule) }}</p>
                <p class="mt-1 text-sm text-emerald-600">{{ formatDuration(job.activeRun?.startedAtMs) }}</p>
              </div>
              <div class="text-right text-xs text-slate-500">
                <div>ID: {{ job.id }}</div>
                <div>{{ t('cron.nextRun') }}: {{ formatTime(job.state.nextRunAtMs) }}</div>
              </div>
            </div>
            <div class="mt-4 rounded-2xl bg-slate-50 p-4 text-sm text-slate-700">
              <div class="font-medium text-slate-900">{{ t('cron.detailTitle') }}</div>
              <p class="mt-2 whitespace-pre-wrap break-words">{{ job.payload.message || t('cron.emptyMessage') }}</p>
              <div class="mt-3 grid gap-2 text-xs text-slate-500 md:grid-cols-3">
                <div>{{ t('cron.channel') }}: {{ job.payload.channel || '-' }}</div>
                <div>{{ t('cron.recipient') }}: {{ job.payload.to || '-' }}</div>
                <div>{{ t('cron.lastRun') }}: {{ formatTime(job.state.lastRunAtMs) }}</div>
              </div>
            </div>
            <div class="mt-4 flex flex-wrap gap-2">
              <button class="rounded-xl bg-amber-100 px-3 py-2 text-sm font-medium text-amber-700 hover:bg-amber-200" :disabled="actionBusy[job.id]" @click="toggleJob(job)">
                <span class="inline-flex items-center gap-2"><Pause v-if="job.enabled" :size="16" /><Play v-else :size="16" />{{ job.enabled ? t('cron.pause') : t('cron.enable') }}</span>
              </button>
              <button class="rounded-xl bg-sky-100 px-3 py-2 text-sm font-medium text-sky-700 hover:bg-sky-200" :disabled="actionBusy[job.id]" @click="runJob(job)">
                <span class="inline-flex items-center gap-2"><PlayCircle :size="16" />{{ t('cron.runNow') }}</span>
              </button>
              <button class="rounded-xl bg-rose-100 px-3 py-2 text-sm font-medium text-rose-700 hover:bg-rose-200" :disabled="actionBusy[job.id]" @click="stopJob(job)">
                <span class="inline-flex items-center gap-2"><Square :size="16" />{{ t('cron.stop') }}</span>
              </button>
              <button class="rounded-xl bg-slate-100 px-3 py-2 text-sm font-medium text-slate-700 hover:bg-slate-200" :disabled="actionBusy[job.id]" @click="openEdit(job)">
                <span class="inline-flex items-center gap-2"><Pencil :size="16" />{{ t('cron.edit') }}</span>
              </button>
              <button class="rounded-xl bg-rose-50 px-3 py-2 text-sm font-medium text-rose-700 hover:bg-rose-100" :disabled="actionBusy[job.id]" @click="deleteJob(job)">
                <span class="inline-flex items-center gap-2"><Trash2 :size="16" />{{ t('cron.delete') }}</span>
              </button>
            </div>
          </article>
        </div>
        <div v-else class="rounded-2xl border border-dashed border-slate-300 bg-white px-6 py-8 text-center text-sm text-slate-500">
          {{ t('cron.noRunningTasks') }}
        </div>
      </section>

      <section class="space-y-4">
        <div class="flex items-center justify-between">
          <h3 class="text-lg font-semibold text-slate-800">{{ t('cron.allSection') }}</h3>
          <span class="text-sm text-slate-500">{{ otherJobs.length }}</span>
        </div>
        <div v-if="loading" class="rounded-2xl bg-white px-6 py-8 text-center text-sm text-slate-500">
          {{ t('cron.loading') }}
        </div>
        <div v-else-if="otherJobs.length === 0" class="rounded-2xl border border-dashed border-slate-300 bg-white px-6 py-8 text-center text-sm text-slate-500">
          {{ t('cron.emptyState') }}
        </div>
        <div v-else class="grid gap-4 xl:grid-cols-2">
          <article
            v-for="job in otherJobs"
            :key="job.id"
            class="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm"
          >
            <div class="flex items-start justify-between gap-4">
              <div>
                <div class="flex items-center gap-3">
                  <h4 class="text-xl font-semibold text-slate-900">{{ job.name }}</h4>
                  <span class="rounded-full px-3 py-1 text-xs font-medium" :class="statusClass(job.computedStatus)">
                    {{ t(`cron.status.${job.computedStatus}`) }}
                  </span>
                </div>
                <p class="mt-2 text-sm text-slate-600">{{ describeSchedule(job.schedule) }}</p>
                <p class="mt-1 text-xs text-slate-500">{{ t('cron.nextRun') }}: {{ formatTime(job.state.nextRunAtMs) }}</p>
              </div>
              <div class="text-right text-xs text-slate-500">
                <div>ID: {{ job.id }}</div>
                <div>{{ t('cron.lastRun') }}: {{ formatTime(job.state.lastRunAtMs) }}</div>
              </div>
            </div>
            <div class="mt-4 rounded-2xl bg-slate-50 p-4 text-sm text-slate-700">
              <div class="font-medium text-slate-900">{{ t('cron.detailTitle') }}</div>
              <p class="mt-2 whitespace-pre-wrap break-words">{{ job.payload.message || t('cron.emptyMessage') }}</p>
              <div class="mt-3 grid gap-2 text-xs text-slate-500 md:grid-cols-3">
                <div>{{ t('cron.channel') }}: {{ job.payload.channel || '-' }}</div>
                <div>{{ t('cron.recipient') }}: {{ job.payload.to || '-' }}</div>
                <div>{{ t('cron.lastStatus') }}: {{ job.state.lastStatus || '-' }}</div>
              </div>
              <div v-if="job.state.lastError" class="mt-3 rounded-xl bg-rose-50 px-3 py-2 text-xs text-rose-700">
                {{ job.state.lastError }}
              </div>
            </div>
            <div class="mt-4 flex flex-wrap gap-2">
              <button class="rounded-xl bg-amber-100 px-3 py-2 text-sm font-medium text-amber-700 hover:bg-amber-200" :disabled="actionBusy[job.id]" @click="toggleJob(job)">
                <span class="inline-flex items-center gap-2"><Pause v-if="job.enabled" :size="16" /><Play v-else :size="16" />{{ job.enabled ? t('cron.pause') : t('cron.enable') }}</span>
              </button>
              <button class="rounded-xl bg-sky-100 px-3 py-2 text-sm font-medium text-sky-700 hover:bg-sky-200" :disabled="actionBusy[job.id]" @click="runJob(job)">
                <span class="inline-flex items-center gap-2"><PlayCircle :size="16" />{{ t('cron.runNow') }}</span>
              </button>
              <button class="rounded-xl bg-slate-100 px-3 py-2 text-sm font-medium text-slate-700 hover:bg-slate-200" :disabled="actionBusy[job.id]" @click="openEdit(job)">
                <span class="inline-flex items-center gap-2"><Pencil :size="16" />{{ t('cron.edit') }}</span>
              </button>
              <button class="rounded-xl bg-rose-50 px-3 py-2 text-sm font-medium text-rose-700 hover:bg-rose-100" :disabled="actionBusy[job.id]" @click="deleteJob(job)">
                <span class="inline-flex items-center gap-2"><Trash2 :size="16" />{{ t('cron.delete') }}</span>
              </button>
            </div>
          </article>
        </div>
      </section>
    </div>

    <div v-if="showForm" class="fixed inset-0 z-[90] flex items-center justify-center bg-slate-950/50 p-4">
      <div class="max-h-[90vh] w-full max-w-3xl overflow-y-auto rounded-3xl bg-white p-6 shadow-2xl">
        <div class="flex items-center justify-between">
          <h3 class="text-2xl font-semibold text-slate-900">
            {{ editingJobId ? t('cron.editTask') : t('cron.newTask') }}
          </h3>
          <button class="rounded-xl bg-slate-100 px-3 py-2 text-sm text-slate-600 hover:bg-slate-200" @click="showForm = false">
            {{ t('cron.cancel') }}
          </button>
        </div>
        <div class="mt-6 grid gap-4 md:grid-cols-2">
          <label class="text-sm text-slate-700">
            <div class="mb-2 font-medium">{{ t('cron.taskName') }}</div>
            <input v-model="form.name" class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none focus:border-emerald-400" />
          </label>
          <label class="text-sm text-slate-700">
            <div class="mb-2 font-medium">{{ t('cron.scheduleType') }}</div>
            <select v-model="form.scheduleKind" class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none focus:border-emerald-400">
              <option value="at">{{ t('cron.schedule.at') }}</option>
              <option value="every">{{ t('cron.schedule.every') }}</option>
              <option value="cron">{{ t('cron.schedule.cron') }}</option>
            </select>
          </label>
          <label v-if="form.scheduleKind === 'at'" class="text-sm text-slate-700">
            <div class="mb-2 font-medium">{{ t('cron.runAt') }}</div>
            <input v-model="form.atInput" type="datetime-local" class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none focus:border-emerald-400" />
          </label>
          <label v-if="form.scheduleKind === 'every'" class="text-sm text-slate-700">
            <div class="mb-2 font-medium">{{ t('cron.everySeconds') }}</div>
            <input v-model.number="form.everySeconds" type="number" min="1" class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none focus:border-emerald-400" />
          </label>
          <label v-if="form.scheduleKind === 'cron'" class="text-sm text-slate-700">
            <div class="mb-2 font-medium">{{ t('cron.cronExpr') }}</div>
            <input v-model="form.cronExpr" class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none focus:border-emerald-400" />
          </label>
          <label v-if="form.scheduleKind === 'cron'" class="text-sm text-slate-700">
            <div class="mb-2 font-medium">{{ t('cron.timezone') }}</div>
            <input v-model="form.timezone" class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none focus:border-emerald-400" />
          </label>
        </div>
        <label class="mt-4 block text-sm text-slate-700">
          <div class="mb-2 font-medium">{{ t('cron.message') }}</div>
          <textarea v-model="form.message" rows="5" class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none focus:border-emerald-400" />
        </label>
        <div class="mt-4 grid gap-4 md:grid-cols-2">
          <label class="text-sm text-slate-700">
            <div class="mb-2 font-medium">{{ t('cron.channel') }}</div>
            <input v-model="form.channel" class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none focus:border-emerald-400" />
          </label>
          <label class="text-sm text-slate-700">
            <div class="mb-2 font-medium">{{ t('cron.recipient') }}</div>
            <input v-model="form.to" class="w-full rounded-2xl border border-slate-200 px-4 py-3 outline-none focus:border-emerald-400" />
          </label>
        </div>
        <div class="mt-4 flex flex-wrap gap-4 text-sm text-slate-700">
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
        <div class="mt-6 flex justify-end gap-3">
          <button class="rounded-xl bg-slate-100 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-200" @click="showForm = false">
            {{ t('cron.cancel') }}
          </button>
          <button class="rounded-xl bg-emerald-600 px-4 py-2 text-sm font-semibold text-white hover:bg-emerald-700" :disabled="submitting" @click="submitForm">
            {{ submitting ? t('cron.saving') : t('cron.save') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

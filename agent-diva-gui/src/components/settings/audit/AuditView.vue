<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { getAuditEvents, getAuditRawLog, type AuditRecord } from '../../../api/audit';
import { isTauriRuntime } from '../../../api/desktop';

type AuditTab = 'structured' | 'raw';

const currentDate = ref(new Date().toISOString().slice(0, 10));
const currentTab = ref<AuditTab>('structured');
const loading = ref(false);
const error = ref('');
const events = ref<AuditRecord[]>([]);
const rawLog = ref('');

const isTauri = isTauriRuntime();

const formattedDate = computed(() => currentDate.value || new Date().toISOString().slice(0, 10));

const eventSummary = (record: AuditRecord) => {
  const event = record.event;
  switch (event.event_type) {
    case 'tool_invoked':
      return `Tool ${event.tool} invoked in ${event.duration_ms}ms`;
    case 'tool_denied':
      return `Tool ${event.tool} denied: ${event.reason}`;
    case 'decision_point':
      return `Decision ${event.phase}: ${event.llm_decision}`;
    case 'injection_detected':
      return `Injection detected (${event.severity}): ${event.pattern}`;
    case 'pii_redacted':
      return `PII redacted: ${event.kind} x ${event.count}`;
    case 'token_used':
      return `Token usage ${event.model}: ${event.total} total`;
    case 'presence_changed':
      return `Presence changed: ${event.from} -> ${event.to}`;
    case 'heartbeat_triggered':
      return `Heartbeat ${event.state}`;
  }
};

const loadAudit = async () => {
  if (!isTauri) {
    return;
  }
  loading.value = true;
  error.value = '';
  try {
    const [nextEvents, nextRawLog] = await Promise.all([
      getAuditEvents(formattedDate.value),
      getAuditRawLog(formattedDate.value),
    ]);
    events.value = nextEvents;
    rawLog.value = nextRawLog;
  } catch (err) {
    error.value = String(err);
    events.value = [];
    rawLog.value = '';
  } finally {
    loading.value = false;
  }
};

watch(currentDate, () => {
  void loadAudit();
});

onMounted(() => {
  void loadAudit();
});
</script>

<template>
  <div class="p-6 space-y-5">
    <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
      <div class="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div>
          <h3 class="text-lg font-semibold text-slate-900">Behavioral Audit</h3>
          <p class="text-sm text-slate-500">
            Review structured gateway events from `gateway.log.YYYY-MM-DD`.
          </p>
        </div>
        <div class="flex flex-wrap items-center gap-3">
          <input
            v-model="currentDate"
            type="date"
            class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-sm text-slate-700 outline-none transition focus:border-sky-300 focus:bg-white"
          />
          <button
            class="rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white transition hover:bg-slate-700 disabled:cursor-not-allowed disabled:opacity-60"
            :disabled="loading || !isTauri"
            @click="loadAudit"
          >
            {{ loading ? 'Loading...' : 'Refresh' }}
          </button>
        </div>
      </div>
    </div>

    <div
      v-if="!isTauri"
      class="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-800"
    >
      Browser preview cannot read local audit logs. Open the Tauri app to use this page.
    </div>

    <div
      v-else-if="error"
      class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700"
    >
      {{ error }}
    </div>

    <div class="rounded-2xl border border-slate-200 bg-white shadow-sm">
      <div class="flex border-b border-slate-100 px-3 pt-3">
        <button
          class="rounded-t-xl px-4 py-2 text-sm font-medium transition"
          :class="
            currentTab === 'structured'
              ? 'bg-slate-900 text-white'
              : 'text-slate-500 hover:bg-slate-100 hover:text-slate-900'
          "
          @click="currentTab = 'structured'"
        >
          Structured Events
        </button>
        <button
          class="rounded-t-xl px-4 py-2 text-sm font-medium transition"
          :class="
            currentTab === 'raw'
              ? 'bg-slate-900 text-white'
              : 'text-slate-500 hover:bg-slate-100 hover:text-slate-900'
          "
          @click="currentTab = 'raw'"
        >
          Raw Log
        </button>
      </div>

      <div v-if="currentTab === 'structured'" class="max-h-[560px] overflow-y-auto p-3">
        <div v-if="events.length === 0" class="rounded-xl bg-slate-50 px-4 py-10 text-center text-sm text-slate-500">
          No audit events for {{ formattedDate }}.
        </div>
        <div v-else class="space-y-3">
          <article
            v-for="(record, index) in events"
            :key="`${record.timestamp}-${index}`"
            class="rounded-2xl border border-slate-200 bg-slate-50/60 p-4"
          >
            <div class="flex flex-wrap items-center gap-2 text-xs uppercase tracking-wide text-slate-500">
              <span class="rounded-full bg-white px-2 py-1 font-semibold text-slate-700">
                {{ record.event.event_type }}
              </span>
              <span>{{ record.level }}</span>
              <span>{{ record.timestamp }}</span>
            </div>
            <p class="mt-3 text-sm font-medium text-slate-900">
              {{ eventSummary(record) }}
            </p>
            <pre class="mt-3 overflow-x-auto rounded-xl bg-slate-950 p-3 text-xs text-slate-100">{{ JSON.stringify(record.event, null, 2) }}</pre>
          </article>
        </div>
      </div>

      <div v-else class="p-3">
        <pre class="max-h-[560px] overflow-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{{ rawLog || `No raw log for ${formattedDate}.` }}</pre>
      </div>
    </div>
  </div>
</template>

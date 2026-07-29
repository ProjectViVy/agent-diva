<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Clock3, Folder, ShieldCheck, ShieldX, Terminal } from 'lucide-vue-next';
import type { ApprovalDecision, CommandApprovalRequest } from '../api/desktop';

const { t } = useI18n();
const props = defineProps<{
  request: CommandApprovalRequest;
  active: boolean;
  submitting?: boolean;
  error?: string | null;
}>();
const emit = defineEmits<{
  respond: [payload: { approval_id: string; decision: ApprovalDecision }];
  locate: [sessionKey: string];
}>();

const now = ref(Date.now());
let timer: ReturnType<typeof setInterval> | undefined;
const expiresAt = computed(
  () => new Date(props.request.created_at).getTime() + props.request.timeout_seconds * 1000,
);
const remaining = computed(() => Math.max(0, Math.ceil((expiresAt.value - now.value) / 1000)));
const expired = computed(() => remaining.value === 0);

function respond(decision: ApprovalDecision) {
  if (!props.active || props.submitting || expired.value) return;
  emit('respond', { approval_id: props.request.approval_id, decision });
}

onMounted(() => {
  timer = setInterval(() => {
    now.value = Date.now();
  }, 1000);
});
onBeforeUnmount(() => {
  if (timer) clearInterval(timer);
});
</script>

<template>
  <section class="command-approval" :class="{ inactive: !active, expired }">
    <header>
      <span class="title"><Terminal :size="16" />{{ t('approval.commandTitle') }}</span>
      <span class="countdown"><Clock3 :size="13" />{{ t('approval.remaining', { seconds: remaining }) }}</span>
    </header>
    <pre class="command">{{ request.command }}</pre>
    <div class="meta"><Folder :size="13" /><span>{{ request.cwd }}</span></div>
    <div class="reason">{{ request.reason }}</div>
    <div class="source">{{ t('approval.sourceSession', { session: request.scope.session_key }) }}</div>
    <div v-if="request.suggested_prefix" class="global-suggestion">
      <span>{{ t('approval.globalSuggestion') }}</span>
      <code>{{ request.suggested_prefix.pattern.join(' ') }}</code>
    </div>
    <div v-if="error" class="error" role="alert">{{ error }}</div>
    <div v-if="!active" class="inactive-action">
      <button @click="emit('locate', request.scope.session_key)">{{ t('approval.openSession') }}</button>
    </div>
    <div v-else class="actions">
      <button :disabled="submitting || expired" class="reject" @click="respond('reject')">
        <ShieldX :size="14" />{{ t('approval.reject') }}
      </button>
      <button :disabled="submitting || expired" @click="respond('approve_once')">
        <ShieldCheck :size="14" />{{ t('approval.approveOnce') }}
      </button>
      <button :disabled="submitting || expired" class="primary" @click="respond('approve_session')">
        <ShieldCheck :size="14" />{{ t('approval.approveSession') }}
      </button>
      <button
        v-if="request.suggested_prefix"
        :disabled="submitting || expired"
        class="global"
        @click="respond('approve_global')"
      >
        <ShieldCheck :size="14" />{{ t('approval.approveGlobal') }}
      </button>
    </div>
  </section>
</template>

<style scoped>
.command-approval { margin: 10px 0; padding: 14px; border: 1px solid var(--warning); border-radius: var(--radius-sm); background: var(--panel-solid); }
.command-approval.inactive { border-color: var(--line); }
.command-approval.expired { opacity: .65; }
header, .title, .countdown, .meta, .actions { display: flex; align-items: center; }
header { justify-content: space-between; gap: 12px; }
.title { gap: 6px; font-weight: 650; color: var(--text); }
.countdown, .meta, .source { color: var(--text-muted); font-size: .75rem; }
.countdown, .meta { gap: 5px; }
.command { margin: 10px 0; padding: 10px; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; border-radius: 6px; background: var(--bg); color: var(--text); font-size: .8rem; }
.reason { margin: 8px 0; color: var(--text); font-size: .8rem; }
.source { margin-top: 6px; }
.actions { justify-content: flex-end; gap: 8px; margin-top: 12px; flex-wrap: wrap; }
button { display: inline-flex; align-items: center; gap: 5px; padding: 6px 10px; border: 1px solid var(--line); border-radius: 6px; background: var(--panel-solid); color: var(--text); cursor: pointer; }
button.primary { background: var(--accent); border-color: var(--accent); color: white; }
button.global { background: var(--success); border-color: var(--success); color: white; }
button.reject:hover { color: var(--danger); border-color: var(--danger); }
button:disabled { cursor: not-allowed; opacity: .55; }
.inactive-action { margin-top: 10px; text-align: right; }
.error { margin-top: 8px; color: var(--danger); font-size: .78rem; }
.global-suggestion { display: flex; gap: 8px; align-items: center; margin-top: 8px; color: var(--text-muted); font-size: .75rem; flex-wrap: wrap; }
.global-suggestion code { color: var(--text); background: var(--bg); border-radius: 4px; padding: 2px 6px; }
</style>

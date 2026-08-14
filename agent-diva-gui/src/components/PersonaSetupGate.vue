<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Loader2, RefreshCw, Sparkles } from '@lucide/vue';
import { getPersonaStatus, initializePersona, isTauriRuntime, repairPersona } from '../api/desktop';
import type { PersonaInitializationPayload, PersonaStatusView } from '../api/desktop';
import { errorMessage } from '../utils/errorMessage';

const emit = defineEmits<{ (event: 'ready'): void }>();
const { t } = useI18n();
const status = ref<PersonaStatusView | null>(null);
const loading = ref(false);
const saving = ref(false);
const error = ref('');
const fields = reactive<PersonaInitializationPayload>({
  identity: '', relationship: '', redline: '', user: '', world: '',
});
type SetupKind = keyof PersonaInitializationPayload;
const requiredKinds: SetupKind[] = ['identity', 'relationship', 'redline', 'user', 'world'];
const invalidKinds = computed<SetupKind[]>(() => requiredKinds.filter((kind) => !status.value?.files[kind]?.valid));
const visible = computed(() => Boolean(error.value) || Boolean(status.value && status.value.status !== 'ready'));

async function load() {
  if (!isTauriRuntime()) return;
  loading.value = true;
  error.value = '';
  try {
    status.value = await getPersonaStatus();
    if (status.value.status === 'ready') emit('ready');
  } catch (cause) {
    error.value = errorMessage(cause, t('personaWorkspace.statusFailed'));
  } finally {
    loading.value = false;
  }
}

async function submit() {
  if (!status.value) return;
  saving.value = true;
  error.value = '';
  try {
    status.value = status.value.status === 'uninitialized'
      ? await initializePersona({ ...fields })
      : await repairPersona(Object.fromEntries(invalidKinds.value.map((kind) => [kind, fields[kind]])));
    if (status.value.status === 'ready') emit('ready');
  } catch (cause) {
    error.value = errorMessage(cause, t('personaWorkspace.setupFailed'));
  } finally {
    saving.value = false;
  }
}

onMounted(() => { window.setTimeout(load, 500); });
</script>

<template>
  <div v-if="visible" class="persona-setup-backdrop">
    <section class="persona-setup" role="dialog" aria-modal="true" aria-labelledby="persona-setup-title">
      <header>
        <div class="setup-icon"><Sparkles :size="20" /></div>
        <div><h1 id="persona-setup-title">{{ t('personaWorkspace.setupTitle') }}</h1><p>{{ t('personaWorkspace.setupSubtitle') }}</p></div>
      </header>
      <div v-if="status?.status === 'incomplete'" class="repair-note">{{ t('personaWorkspace.repairNote') }}</div>
      <div v-if="status" class="setup-fields">
        <label v-for="kind in (status?.status === 'incomplete' ? invalidKinds : requiredKinds)" :key="kind">
          <span>{{ t(`personaWorkspace.files.${kind}`) }}</span>
          <textarea v-model="fields[kind]" rows="3" :placeholder="kind === 'world' ? t('personaWorkspace.worldPlaceholder') : t('personaWorkspace.markdownPlaceholder')" />
        </label>
      </div>
      <p v-if="error" class="setup-error" role="alert">{{ error }}</p>
      <footer>
        <button class="secondary" :disabled="loading || saving" @click="load"><RefreshCw :size="15" />{{ t('personaWorkspace.retryStatus') }}</button>
        <button v-if="status" class="primary" :disabled="saving || invalidKinds.some((kind) => !fields[kind].trim())" @click="submit">
          <Loader2 v-if="saving" :size="15" class="spin" />{{ t('personaWorkspace.completeSetup') }}
        </button>
      </footer>
    </section>
  </div>
</template>

<style scoped>
.persona-setup-backdrop { position: fixed; inset: 0; z-index: 900; display: grid; place-items: center; padding: 28px; background: color-mix(in srgb, var(--bg) 78%, transparent); backdrop-filter: blur(18px); }
.persona-setup { width: min(760px, 100%); max-height: calc(100vh - 56px); overflow: auto; padding: 24px; border: 1px solid var(--line); border-radius: 18px; background: var(--panel-solid); box-shadow: 0 28px 80px rgba(0,0,0,.32); }
header { display: flex; gap: 13px; align-items: flex-start; }
.setup-icon { display: grid; place-items: center; width: 38px; height: 38px; border-radius: 12px; background: color-mix(in srgb, var(--accent) 14%, transparent); color: var(--accent); }
h1 { margin: 0; color: var(--text); font-size: 20px; } header p { margin: 5px 0 0; color: var(--text-muted); font-size: 13px; }
.repair-note { margin-top: 16px; padding: 10px 12px; border-radius: 8px; background: color-mix(in srgb, var(--warning) 10%, transparent); color: var(--text); font-size: 12px; }
.setup-fields { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; margin-top: 20px; }
label { display: grid; gap: 7px; } label:last-child:nth-child(odd) { grid-column: 1 / -1; } label span { color: var(--text); font-size: 12px; font-weight: 600; }
textarea { resize: vertical; min-height: 78px; padding: 10px 11px; border: 1px solid var(--line); border-radius: 8px; background: var(--panel); color: var(--text); font: inherit; line-height: 1.5; }
textarea:focus { outline: 2px solid color-mix(in srgb, var(--accent) 35%, transparent); border-color: var(--accent); }
.setup-error { color: var(--danger); font-size: 12px; }
footer { display: flex; justify-content: flex-end; gap: 9px; margin-top: 20px; }
button { display: inline-flex; align-items: center; gap: 6px; border: 1px solid var(--line); border-radius: 8px; padding: 9px 14px; cursor: pointer; }
button:disabled { opacity: .5; cursor: default; }.secondary { background: transparent; color: var(--text); }.primary { border-color: var(--accent); background: var(--accent); color: white; }
.spin { animation: spin 1s linear infinite; } @keyframes spin { to { transform: rotate(360deg); } }
@media (max-width: 680px) { .setup-fields { grid-template-columns: 1fr; } label:last-child:nth-child(odd) { grid-column: auto; } }
</style>

<script setup lang="ts">
import { onUnmounted, ref, watch } from 'vue';
import { Activity, Copy, Monitor } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import { showAppToast } from '../../../utils/appToast';

const { t } = useI18n();
const props = defineProps<{ lines: string[]; loading: boolean }>();
const emit = defineEmits<{ (event: 'refresh'): void }>();
const autoRefresh = ref(false);
let timer: ReturnType<typeof setInterval> | undefined;

watch(autoRefresh, enabled => {
  if (timer) clearInterval(timer);
  timer = enabled ? setInterval(() => emit('refresh'), 5000) : undefined;
});
onUnmounted(() => { if (timer) clearInterval(timer); });

async function copyLine(line: string) {
  try { await navigator.clipboard.writeText(line); showAppToast(t('auditPage.gui.copySuccess'), 'success'); }
  catch { showAppToast(t('auditPage.gui.copyFailed'), 'error'); }
}
</script>

<template>
  <div class="gui-log-tab" role="tabpanel" :aria-label="t('auditPage.gui.panelLabel')" :aria-busy="loading">
    <div class="gui-log-source"><Monitor :size="15" aria-hidden="true" /> {{ t('auditPage.gui.source') }}</div>
    <div class="log-toolbar">
      <button class="log-toolbar-btn" :class="{ active: autoRefresh }" :aria-pressed="autoRefresh" @click="autoRefresh = !autoRefresh">
        <Activity :size="14" aria-hidden="true" /> {{ autoRefresh ? t('auditPage.gui.autoRefreshOn') : t('auditPage.gui.autoRefreshOff') }}
      </button>
    </div>
    <div v-if="loading && !lines.length" class="empty-state">{{ t('auditPage.gui.loading') }}</div>
    <div v-else-if="!lines.length" class="empty-state" role="status"><p>{{ t('auditPage.gui.emptyTitle') }}</p><small>{{ t('auditPage.gui.emptyHint') }}</small></div>
    <div v-else class="log-lines-container">
      <div v-for="(line, index) in props.lines" :key="index" class="log-line">
        <code>{{ line }}</code><button :aria-label="t('auditPage.gui.copyLineLabel', { line: index + 1 })" @click="copyLine(line)"><Copy :size="12" /></button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.gui-log-source { display:flex; align-items:center; gap:6px; margin-bottom:12px; color:var(--info, #0ea5e9); font-size:12px; font-weight:600; }
.log-toolbar { margin-bottom:12px; }.log-toolbar-btn { display:inline-flex; gap:6px; align-items:center; padding:6px 12px; border:1px solid var(--line); border-radius:var(--radius-sm); background:var(--panel); color:var(--text-muted); cursor:pointer; }.log-toolbar-btn.active { color:var(--info, #0ea5e9); border-color:var(--info, #0ea5e9); }.empty-state { min-height:200px; display:grid; place-content:center; text-align:center; color:var(--text-muted); }.empty-state p { margin:0 0 6px; }.log-lines-container { max-height:500px; overflow:auto; border:1px solid var(--line); border-radius:var(--radius-sm); }.log-line { display:grid; grid-template-columns:1fr 28px; gap:8px; padding:6px 8px; border-bottom:1px solid var(--line); font:11px/1.6 'SF Mono','Fira Code',monospace; }.log-line:last-child { border-bottom:none; }.log-line code { white-space:pre-wrap; overflow-wrap:anywhere; }.log-line button { border:0; background:transparent; color:var(--text-muted); cursor:pointer; }
</style>

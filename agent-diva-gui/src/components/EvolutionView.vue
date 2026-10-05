<script setup lang="ts">
/**
 * DN-4D — cognition/evolution companion view (DN-C2 scope only).
 *
 * Surfaces the authoritative cognition block verbatim: policy,
 * eligibility/window, watermark, active run, results, pause/cancel and
 * recovery states. Policy toggling is a CAS write — it can never clear a
 * recovery block, which is shown separately and honestly.
 */
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { vivyCognitive } from '../state/vivy-cognitive';

const { t } = useI18n();
const cog = vivyCognitive();

const enabled = ref(false);
const minIntervalMs = ref(0);
const policyNotice = ref<string | null>(null);
const resultsItems = computed(() => cog.results.value?.items ?? []);
const cogStatus = computed(() => cog.status.value?.cognition);

async function savePolicy() {
  policyNotice.value = null;
  const out = await cog.setPolicy({
    enabled: enabled.value,
    minIntervalMs: minIntervalMs.value,
    baseRevision: cog.policy.value?.policy_revision ?? 0,
  });
  if (out && out.status !== 'ok') policyNotice.value = `${out.error.code}: ${out.error.message}`;
}

async function trigger() {
  await cog.triggerCognition();
}

async function cancelActive() {
  const runId = cogStatus.value?.active_run_id;
  if (runId) await cog.cancelCognition(runId);
}

async function syncEnabled() {
  await cog.getPolicy();
  if (cog.policy.value) {
    enabled.value = cog.policy.value.enabled;
    minIntervalMs.value = cog.policy.value.min_interval_ms;
  }
}

onMounted(async () => {
  await cog.refresh();
  if (!cog.capabilityBlocked.value) await Promise.all([syncEnabled(), cog.listResults()]);
});
</script>

<template>
  <div class="evolution-view">
    <div v-if="cog.capabilityBlocked.value" class="ev-notice">
      {{ t('evolutionView.unavailable', { code: cog.blockReason.value?.code }) }}
    </div>
    <div v-if="cog.frozenRecoveryRequired.value" class="ev-error">
      {{ t('evolutionView.recoveryRequired') }}
    </div>

    <section>
      <h4>{{ t('evolutionView.statusTitle') }}</h4>
      <div v-if="cogStatus" class="ev-grid">
        <span>{{ t('evolutionView.phase') }}: {{ cogStatus.phase }}</span>
        <span>{{ t('evolutionView.enabled') }}: {{ cogStatus.enabled }}</span>
        <span>{{ t('evolutionView.activeRun') }}: {{ cogStatus.active_run_id || '—' }}</span>
        <span>{{ t('evolutionView.source') }}: {{ cogStatus.source_id || '—' }}</span>
        <span>{{ t('evolutionView.watermark') }}: {{ cogStatus.watermark }}</span>
        <span>{{ t('evolutionView.pendingThrough') }}: {{ cogStatus.pending_through }}</span>
        <span v-if="cogStatus.block_reason" class="ev-error">
          {{ t('evolutionView.blockReason') }}: {{ cogStatus.block_reason }}
        </span>
      </div>
      <div v-else class="ev-empty">{{ t('evolutionView.noStatus') }}</div>
      <div v-if="cogStatus?.eligibility" class="ev-pre-wrap">
        <pre class="ev-pre">{{ JSON.stringify(cogStatus.eligibility, null, 2) }}</pre>
      </div>
      <div class="ev-row">
        <button class="ev-btn primary" @click="trigger">{{ t('evolutionView.trigger') }}</button>
        <button
          class="ev-btn"
          :disabled="!cogStatus?.active_run_id"
          @click="cancelActive"
        >
          {{ t('evolutionView.cancelActive') }}
        </button>
        <button class="ev-btn" @click="cog.refresh()">{{ t('evolutionView.refresh') }}</button>
      </div>
    </section>

    <section>
      <h4>{{ t('evolutionView.policyTitle') }}</h4>
      <div v-if="cog.policyError.value" class="ev-error">
        {{ cog.policyError.value.code }}: {{ cog.policyError.value.message }}
      </div>
      <label class="ev-check">
        <input v-model="enabled" type="checkbox" />
        {{ t('evolutionView.enableLabel') }}
      </label>
      <label class="ev-row">
        {{ t('evolutionView.minInterval') }}
        <input v-model.number="minIntervalMs" type="number" class="ev-num" min="0" />
      </label>
      <div class="ev-scope">
        {{ t('evolutionView.policyRevision') }}: {{ cog.policy.value?.policy_revision ?? '—' }}
      </div>
      <button class="ev-btn primary" @click="savePolicy">{{ t('evolutionView.savePolicy') }}</button>
      <div v-if="policyNotice" class="ev-error">{{ policyNotice }}</div>
    </section>

    <section>
      <h4>{{ t('evolutionView.resultsTitle') }}</h4>
      <div v-if="!resultsItems.length" class="ev-empty">{{ t('evolutionView.noResults') }}</div>
      <div v-for="(item, i) in resultsItems" :key="i" class="ev-result">
        <pre class="ev-pre">{{ JSON.stringify(item, null, 2) }}</pre>
      </div>
    </section>
  </div>
</template>

<style scoped>
.evolution-view {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 8px 0;
}
section {
  display: flex;
  flex-direction: column;
  gap: 8px;
  border-top: 1px solid var(--border, rgba(255, 255, 255, 0.08));
  padding-top: 10px;
}
h4 {
  margin: 0;
  font-size: 13px;
}
.ev-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 6px 16px;
  font-size: 12px;
}
.ev-row {
  display: flex;
  gap: 8px;
  align-items: center;
  font-size: 12px;
}
.ev-check {
  display: flex;
  gap: 6px;
  align-items: center;
  font-size: 12px;
}
.ev-num {
  width: 110px;
  background: transparent;
  border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  border-radius: 8px;
  padding: 6px;
  color: var(--text);
}
.ev-btn {
  padding: 6px 12px;
  border-radius: 8px;
  border: 1px solid var(--border, rgba(255, 255, 255, 0.15));
  background: transparent;
  color: var(--text);
  cursor: pointer;
  font-size: 12px;
}
.ev-btn.primary {
  background: var(--accent);
  border-color: transparent;
  color: #fff;
}
.ev-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.ev-notice {
  padding: 10px;
  border-radius: 8px;
  background: rgba(255, 170, 60, 0.12);
  font-size: 12px;
}
.ev-error {
  color: var(--danger, #f36);
  font-size: 12px;
}
.ev-empty,
.ev-scope {
  color: var(--text-muted);
  font-size: 12px;
}
.ev-pre {
  font-size: 11px;
  white-space: pre-wrap;
  margin: 0;
  max-height: 160px;
  overflow: auto;
}
.ev-result {
  border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  border-radius: 8px;
  padding: 8px;
}
</style>

<script setup lang="ts">
/**
 * DN-4D — scoped memory companion view (DN-C2 scope only).
 *
 * Search returns cards; expansion requires the observed revision; mutation
 * exposes receipt/canonical/index states verbatim. Hidden scopes are never
 * silently refetched — scope/destination come from the bound status.
 */
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { vivyCognitive } from '../state/vivy-cognitive';

const { t } = useI18n();
const cog = vivyCognitive();

const query = ref('');
const limit = ref(20);
const budgetChars = ref(4000);
const searching = ref(false);
const expandId = ref('');
const expandRevision = ref(0);
const mutationJson = ref('{\n  "operation": "put",\n  "record_id": "",\n  "body": ""\n}');
const mutationError = ref<string | null>(null);
const receiptId = ref('');

const scope = computed(() => cog.status.value?.scope);
const destination = computed(() => cog.status.value?.destination_id);
const cards = computed(() => cog.memoryCards.value?.items ?? []);

async function search() {
  searching.value = true;
  await cog.searchMemory({
    query: query.value,
    limit: limit.value,
    budgetChars: budgetChars.value,
  });
  searching.value = false;
}

async function expand(card: Record<string, unknown>) {
  expandId.value = String(card.card_id ?? card.id ?? '');
  const rev = Number(card.revision ?? card.expected_revision ?? 0);
  expandRevision.value = Number.isSafeInteger(rev) ? rev : 0;
  await cog.expandMemory(expandId.value, expandRevision.value, budgetChars.value);
}

async function mutate() {
  mutationError.value = null;
  try {
    const mutation = JSON.parse(mutationJson.value);
    await cog.mutateMemory(mutation);
  } catch (e) {
    mutationError.value = e instanceof Error ? e.message : String(e);
  }
}

async function lookupReceipt() {
  if (receiptId.value.trim()) await cog.receipt(receiptId.value.trim());
}
</script>

<template>
  <div class="memory-view">
    <div v-if="cog.capabilityBlocked.value" class="mv-notice">
      {{ t('memoryView.unavailable', { code: cog.blockReason.value?.code }) }}
    </div>
    <div class="mv-scope">
      <span>{{ t('memoryView.scope') }}: {{ scope ? JSON.stringify(scope) : '—' }}</span>
      <span>{{ t('memoryView.destination') }}: {{ destination ?? '—' }}</span>
    </div>
    <div v-if="cog.memoryError.value" class="mv-error">
      {{ cog.memoryError.value.code }}: {{ cog.memoryError.value.message }}
    </div>

    <section>
      <h4>{{ t('memoryView.searchTitle') }}</h4>
      <div class="mv-row">
        <input v-model="query" class="mv-input" :placeholder="t('memoryView.queryPlaceholder')" />
        <input v-model.number="limit" type="number" class="mv-num" :title="t('memoryView.limit')" />
        <button class="mv-btn primary" :disabled="searching" @click="search">
          {{ t('memoryView.search') }}
        </button>
      </div>
      <div v-if="cards.length" class="mv-cards">
        <div v-for="(card, i) in cards" :key="i" class="mv-card">
          <div class="mv-card-head">
            <span>{{ String(card.card_id ?? card.id ?? i) }}</span>
            <span v-if="card.revision !== undefined">r{{ card.revision }}</span>
          </div>
          <pre class="mv-pre">{{ card.title ?? card.summary ?? JSON.stringify(card) }}</pre>
          <button class="mv-btn" @click="expand(card)">{{ t('memoryView.expand') }}</button>
        </div>
      </div>
      <div v-else class="mv-empty">{{ t('memoryView.noCards') }}</div>
      <div v-if="cog.evidence.value" class="mv-evidence">
        <div class="mv-scope">
          {{ t('memoryView.evidenceFor', { id: expandId, rev: expandRevision }) }}
          <span v-if="cog.evidence.value.truncated">{{ t('memoryView.truncated') }}</span>
        </div>
        <div v-for="(item, i) in cog.evidence.value.items ?? []" :key="i" class="mv-card">
          <pre class="mv-pre">{{ JSON.stringify(item, null, 2) }}</pre>
        </div>
      </div>
    </section>

    <section>
      <h4>{{ t('memoryView.mutateTitle') }}</h4>
      <textarea v-model="mutationJson" class="mv-textarea" rows="4" spellcheck="false" />
      <div v-if="mutationError" class="mv-error">{{ mutationError }}</div>
      <button class="mv-btn primary" @click="mutate">{{ t('memoryView.mutate') }}</button>
      <div v-if="cog.lastMutation.value" class="mv-receipt">
        <div class="mv-scope">
          <span>op: {{ cog.lastMutation.value.operation_id || '—' }}</span>
          <span>{{ t('memoryView.status') }}: {{ cog.lastMutation.value.status || '—' }}</span>
          <span>{{ t('memoryView.canonical') }}: {{ cog.lastMutation.value.canonical_status || '—' }}</span>
          <span>{{ t('memoryView.index') }}: {{ cog.lastMutation.value.index_status || '—' }}</span>
        </div>
        <div v-if="cog.lastMutation.value.error_code" class="mv-error">
          {{ cog.lastMutation.value.error_code }}
        </div>
      </div>
    </section>

    <section>
      <h4>{{ t('memoryView.receiptTitle') }}</h4>
      <div class="mv-row">
        <input v-model="receiptId" class="mv-input" :placeholder="t('memoryView.receiptPlaceholder')" />
        <button class="mv-btn" @click="lookupReceipt">{{ t('memoryView.receiptLookup') }}</button>
      </div>
      <div v-for="(r, id) in cog.receipts.value" :key="id" class="mv-receipt">
        <div class="mv-scope">
          <span>{{ id }}</span>
          <span>{{ r.status }}</span>
          <span>{{ t('memoryView.canonical') }}: {{ r.canonical_status }}</span>
          <span>{{ t('memoryView.index') }}: {{ r.index_status }}</span>
          <span v-if="r.error_code" class="mv-error">{{ r.error_code }}</span>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.memory-view {
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
.mv-notice {
  padding: 10px;
  border-radius: 8px;
  background: rgba(255, 170, 60, 0.12);
  font-size: 12px;
}
.mv-scope {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  font-size: 12px;
  color: var(--text-muted);
}
.mv-row {
  display: flex;
  gap: 8px;
}
.mv-input {
  flex: 1;
  background: transparent;
  border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  border-radius: 8px;
  padding: 6px 10px;
  color: var(--text);
  font-size: 12px;
}
.mv-num {
  width: 70px;
  background: transparent;
  border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  border-radius: 8px;
  padding: 6px;
  color: var(--text);
  font-size: 12px;
}
.mv-btn {
  padding: 6px 12px;
  border-radius: 8px;
  border: 1px solid var(--border, rgba(255, 255, 255, 0.15));
  background: transparent;
  color: var(--text);
  cursor: pointer;
  font-size: 12px;
}
.mv-btn.primary {
  background: var(--accent);
  border-color: transparent;
  color: #fff;
}
.mv-card {
  border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  border-radius: 8px;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.mv-card-head {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: var(--text-muted);
}
.mv-pre {
  font-size: 11px;
  white-space: pre-wrap;
  margin: 0;
  max-height: 140px;
  overflow: auto;
}
.mv-textarea {
  width: 100%;
  background: transparent;
  border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  border-radius: 8px;
  padding: 8px;
  color: var(--text);
  font-size: 12px;
  font-family: var(--font-mono, ui-monospace);
}
.mv-error {
  color: var(--danger, #f36);
  font-size: 12px;
}
.mv-empty {
  color: var(--text-muted);
  font-size: 12px;
}
.mv-cards,
.mv-receipt {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.mv-receipt {
  border: 1px dashed var(--border, rgba(255, 255, 255, 0.1));
  border-radius: 8px;
  padding: 8px;
}
</style>

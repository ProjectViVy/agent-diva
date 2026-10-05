<script setup lang="ts">
/**
 * DN-4D — persona + frozen + scoped ACTMEM companion view.
 *
 * Reads and writes only through the vivy-cognitive controller (real
 * `diva.cognitive.*` actions). Current persona revisions and the session's
 * FrozenCore revisions are shown as separate fields — a human edit applies
 * to a NEW conversation's frozen core, not retroactively.
 */
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { vivyCognitive } from '../../state/vivy-cognitive';
import { PERSONA_KINDS, type PersonaKind, type PersonaReview } from '../../api/cognitive';
import PersonaMarkdownEditor from './PersonaMarkdownEditor.vue';
import PersonaSetupGate from './PersonaSetupGate.vue';

const { t } = useI18n();
const cog = vivyCognitive();

const activeKind = ref<PersonaKind>('identity');
const reason = ref('');
const saving = ref(false);
const saveNotice = ref<string | null>(null);
const actmemMaxChars = ref(4000);

const doc = computed(() => cog.personaDocs.value[activeKind.value]);
const err = computed(() => cog.personaErrors.value[activeKind.value]);
const unknownSave = computed(() => cog.unknownSaves.value[activeKind.value]);
const stateBadge = computed(() => cog.status.value?.persona.state ?? '—');

async function selectKind(kind: PersonaKind) {
  activeKind.value = kind;
  saveNotice.value = null;
  if (!cog.personaDocs.value[kind]) await cog.readPersona(kind);
}

async function save() {
  saving.value = true;
  saveNotice.value = null;
  const out = await cog.savePersona(activeKind.value, reason.value.trim() || undefined);
  if (out && out.status !== 'ok') saveNotice.value = `${out.error.code}: ${out.error.message}`;
  saving.value = false;
}

async function decide(review: PersonaReview, decision: 'accept' | 'reject') {
  await cog.decideReview(review.id, decision);
}

const actmemEntries = computed(() => (cog.actmem.value?.entries ?? []) as Array<Record<string, unknown>>);

onMounted(async () => {
  await cog.refresh();
  if (!cog.needsSetup.value) {
    await selectKind(activeKind.value);
    await Promise.all([cog.listReviews(), cog.readFrozen(), cog.readActmem(['pulse', 'recap', 'work'], actmemMaxChars.value), cog.ownerRead()]);
  }
});
</script>

<template>
  <div class="persona-memory-view">
    <PersonaSetupGate v-if="cog.needsSetup.value" />
    <template v-else>
      <div v-if="cog.capabilityBlocked.value" class="pm-notice">
        {{ t('personaMemory.unavailable', { code: cog.blockReason.value?.code }) }}
      </div>
      <div class="pm-head">
        <span class="pm-state">{{ t('personaMemory.personaState') }}: {{ stateBadge }}</span>
        <button class="ui-button ui-button--ghost pm-btn" @click="cog.refresh()">{{ t('personaMemory.refresh') }}</button>
      </div>

      <div class="pm-kinds">
        <button
          v-for="k in PERSONA_KINDS"
          :key="k"
          class="pm-kind"
          :class="{ active: k === activeKind }"
          @click="selectKind(k)"
        >
          {{ t(`personaSetup.kind.${k}`) }}
          <span v-if="cog.currentRevisionOf(k) !== undefined" class="pm-rev"
            >r{{ cog.currentRevisionOf(k) }}</span
          >
        </button>
      </div>

      <div class="pm-doc-meta" v-if="doc">
        <span>{{ doc.file_name }}</span>
        <span>rev {{ doc.revision }}</span>
        <span v-if="doc.pending_count">{{ t('personaMemory.pending', { n: doc.pending_count }) }}</span>
        <span v-if="!doc.valid" class="pm-warn">{{ doc.reason ?? t('personaMemory.invalid') }}</span>
      </div>
      <PersonaMarkdownEditor
        :model-value="cog.drafts[activeKind] ?? ''"
        @update:model-value="cog.setDraft(activeKind, $event)"
      />
      <div class="pm-save-row">
        <input
          v-model="reason"
          class="ui-input pm-reason"
          :placeholder="t('personaSetup.reasonPlaceholder')"
        />
        <button class="ui-button ui-button--ghost pm-btn primary" :disabled="saving" @click="save">
          {{ t('personaMemory.save') }}
        </button>
      </div>
      <div v-if="err" class="pm-error">{{ err.code }}: {{ err.message }}</div>
      <div v-if="unknownSave" class="pm-error">
        {{ t('personaMemory.unknownSave', { code: unknownSave.code }) }}
        <button class="ui-button ui-button--ghost pm-btn" @click="cog.reconcilePersona(activeKind)">
          {{ t('personaMemory.reconcile') }}
        </button>
      </div>
      <div v-if="saveNotice" class="pm-error">{{ saveNotice }}</div>

      <section class="pm-section">
        <h4>{{ t('personaMemory.reviews') }}</h4>
        <div v-if="!cog.reviews.value.length" class="pm-empty">{{ t('personaMemory.noReviews') }}</div>
        <div v-for="r in cog.reviews.value" :key="r.id" class="pm-review">
          <div class="pm-review-head">
            <span>{{ r.kind }} · {{ r.actor }} · base r{{ r.base_revision }}</span>
            <span class="pm-badge" :data-state="r.state">{{ r.state }}</span>
          </div>
          <pre class="pm-diff">{{ r.proposed_markdown }}</pre>
          <div v-if="r.state === 'pending'" class="pm-review-actions">
            <button class="ui-button ui-button--ghost pm-btn primary" @click="decide(r, 'accept')">
              {{ t('personaMemory.accept') }}
            </button>
            <button class="ui-button ui-button--ghost pm-btn" @click="decide(r, 'reject')">{{ t('personaMemory.reject') }}</button>
          </div>
        </div>
      </section>

      <section class="pm-section">
        <h4>{{ t('personaMemory.frozen') }}</h4>
        <div v-if="cog.frozenRecoveryRequired.value" class="pm-error">
          {{ t('personaMemory.frozenRecovery') }}
        </div>
        <div v-else-if="cog.frozen.value" class="pm-frozen">
          <span>{{ t('personaMemory.frozenState') }}: {{ cog.frozen.value.state }}</span>
          <span v-if="cog.frozen.value.revisions" class="pm-rev-list">
            <template v-for="(rev, k) in cog.frozen.value.revisions" :key="k">{{ k }}:r{{ rev }} </template>
          </span>
        </div>
        <div v-else class="pm-empty">{{ t('personaMemory.frozenEmpty') }}</div>
      </section>

      <section class="pm-section">
        <h4>{{ t('personaMemory.actmem') }}</h4>
        <div v-if="cog.actmemError.value" class="pm-error">
          {{ cog.actmemError.value.code }}: {{ cog.actmemError.value.message }}
        </div>
        <div v-if="actmemEntries.length" class="pm-entries">
          <div v-for="(entry, i) in actmemEntries" :key="i" class="pm-entry">
            <pre>{{ JSON.stringify(entry, null, 2) }}</pre>
          </div>
        </div>
        <div v-else class="pm-empty">{{ t('personaMemory.actmemEmpty') }}</div>
        <div v-if="cog.ownerDoc.value" class="pm-owner">
          <div class="pm-doc-meta">
            <span>{{ t('personaMemory.ownerDoc') }}</span>
            <span>rev {{ cog.ownerDoc.value.revision }}</span>
          </div>
          <pre class="pm-diff">{{ cog.ownerDoc.value.markdown || t('personaMemory.actmemEmpty') }}</pre>
        </div>
      </section>
    </template>
  </div>
</template>

<style scoped>
.persona-memory-view {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 8px 0;
}
.pm-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.pm-state {
  color: var(--muted-foreground);
  font-size: 12px;
}
.pm-kinds {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.pm-kind {
  padding: 4px 10px;
  border-radius: 999px;
  border: 1px solid var(--border, var(--border));
  background: transparent;
  color: var(--foreground);
  cursor: pointer;
  font-size: 12px;
}
.pm-kind.active {
  background: var(--primary);
  color: var(--primary-foreground);
}
.pm-rev {
  opacity: 0.7;
  margin-left: 4px;
}
.pm-doc-meta {
  display: flex;
  gap: 12px;
  font-size: 12px;
  color: var(--muted-foreground);
}
.pm-warn {
  color: var(--warning);
}
.pm-save-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.pm-reason {
  flex: 1;
  background: transparent;
  border: 1px solid var(--border, var(--border));
  border-radius: 8px;
  padding: 6px 10px;
  color: var(--foreground);
  font-size: 12px;
}
.pm-error {
  color: var(--destructive);
  font-size: 12px;
}
.pm-notice {
  padding: 10px;
  border-radius: 8px;
  background: var(--destructive-soft);
  font-size: 12px;
}
.pm-section {
  border-top: 1px solid var(--border, var(--border));
  padding-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.pm-section h4 {
  margin: 0;
  font-size: 13px;
}
.pm-empty {
  color: var(--muted-foreground);
  font-size: 12px;
}
.pm-review {
  border: 1px solid var(--border, var(--border));
  border-radius: 8px;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.pm-review-head {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
}
.pm-badge[data-state='pending'] {
  color: var(--warning);
}
.pm-badge[data-state='accepted'] {
  color: var(--success);
}
.pm-badge[data-state='rejected'],
.pm-badge[data-state='stale'] {
  color: var(--muted-foreground);
}
.pm-diff {
  font-size: 11px;
  white-space: pre-wrap;
  max-height: 120px;
  overflow: auto;
  margin: 0;
  color: var(--muted-foreground);
}
.pm-review-actions {
  display: flex;
  gap: 8px;
}
.pm-frozen {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
}
.pm-rev-list {
  color: var(--muted-foreground);
}
.pm-entry pre {
  font-size: 11px;
  white-space: pre-wrap;
  margin: 0;
}
.pm-owner {
  border-top: 1px dashed var(--border, var(--border));
  padding-top: 8px;
}
</style>

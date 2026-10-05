<script setup lang="ts">
/**
 * DN-4D — Persona setup gate.
 *
 * Rendered above the chat input (and inside the persona settings view) while
 * the bound session's persona is uninitialized/incomplete. Initializes via
 * the real `diva.cognitive.persona.initialize` action through the current
 * (or a newly created setup) session — the send path stays gated until the
 * authoritative status reports ready.
 */
import { computed, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { vivyCognitive } from '../../state/vivy-cognitive';
import { vivyChat } from '../../state/chat-instance';
import type { PersonaKind } from '../../api/cognitive';
import { REQUIRED_PERSONA_KINDS } from '../../api/cognitive';
import PersonaMarkdownEditor from './PersonaMarkdownEditor.vue';

const { t } = useI18n();
const cog = vivyCognitive();

const fields = reactive<Record<string, string>>({
  identity: '',
  relationship: '',
  redline: '',
  user: '',
  world: '',
});
const reason = ref('');
const submitting = ref(false);
const submitError = ref<string | null>(null);

const ready = computed(() => REQUIRED_PERSONA_KINDS.every((k: PersonaKind) => fields[k].trim()));

async function initialize() {
  submitError.value = null;
  submitting.value = true;
  try {
    if (!cog.sessionId.value) {
      // Setup session: real session via the existing controller; binding
      // happens when App.vue syncs the new currentSessionId.
      await vivyChat.newSession();
      if (vivyChat.currentSessionId) await cog.bind(vivyChat.currentSessionId);
    }
    const out = await cog.initialize(
      {
        identity: fields.identity,
        relationship: fields.relationship,
        redline: fields.redline,
        user: fields.user,
        world: fields.world,
      },
      reason.value.trim() || undefined,
    );
    if (out && out.status !== 'ok') submitError.value = `${out.error.code}: ${out.error.message}`;
  } catch (e) {
    submitError.value = e instanceof Error ? e.message : String(e);
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <div class="persona-setup-gate">
    <div class="gate-title">{{ t('personaSetup.title') }}</div>
    <div class="gate-desc">{{ t('personaSetup.desc') }}</div>
    <div v-for="kind in REQUIRED_PERSONA_KINDS" :key="kind" class="gate-field">
      <div class="gate-label">{{ t(`personaSetup.kind.${kind}`) }}</div>
      <PersonaMarkdownEditor v-model="fields[kind]" />
    </div>
    <input
      v-model="reason"
      class="gate-reason"
      :placeholder="t('personaSetup.reasonPlaceholder')"
    />
    <div v-if="submitError" class="gate-error">{{ submitError }}</div>
    <button class="gate-submit" :disabled="!ready || submitting" @click="initialize">
      {{ submitting ? t('personaSetup.initializing') : t('personaSetup.initialize') }}
    </button>
  </div>
</template>

<style scoped>
.persona-setup-gate {
  border: 1px solid var(--border, rgba(255, 255, 255, 0.12));
  border-radius: 12px;
  padding: 16px;
  margin: 8px 0;
  background: var(--panel-solid, rgba(20, 20, 24, 0.9));
  display: flex;
  flex-direction: column;
  gap: 10px;
  max-height: 60vh;
  overflow-y: auto;
}
.gate-title {
  font-weight: 600;
  font-size: 14px;
}
.gate-desc {
  color: var(--text-muted);
  font-size: 12px;
}
.gate-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-height: 90px;
}
.gate-label {
  font-size: 12px;
  color: var(--text-muted);
  text-transform: capitalize;
}
.gate-field :deep(.cm-editor) {
  border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  border-radius: 8px;
  min-height: 72px;
}
.gate-reason {
  background: transparent;
  border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  border-radius: 8px;
  padding: 8px 10px;
  color: var(--text);
  font-size: 12px;
}
.gate-error {
  color: var(--danger, #f36);
  font-size: 12px;
}
.gate-submit {
  align-self: flex-start;
  padding: 8px 16px;
  border-radius: 8px;
  border: none;
  background: var(--accent);
  color: #fff;
  cursor: pointer;
  font-size: 13px;
}
.gate-submit:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>

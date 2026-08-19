<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Eye, EyeOff } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import {
  CHANNEL_CREDENTIAL_FIELDS,
  coerceChannelFieldValue,
  fieldsByGroup,
  joinIdList,
  splitIdList,
  type WizardFormField,
} from './channel-wizard-fields';

const props = defineProps<{
  platform: string;
  config: Record<string, unknown>;
}>();

const { t } = useI18n();
const revealedSecrets = ref<Set<string>>(new Set());

const basicFields = computed(() => fieldsByGroup(props.platform, 'basic'));
const advancedFields = computed(() => fieldsByGroup(props.platform, 'advanced'));
const hasSchema = computed(() => (CHANNEL_CREDENTIAL_FIELDS[props.platform] || []).length > 0);

const extraKeys = computed(() => {
  const known = new Set((CHANNEL_CREDENTIAL_FIELDS[props.platform] || []).map((field) => field.key));
  known.add('enabled');
  return Object.keys(props.config).filter((key) => !known.has(key));
});

watch(
  () => props.platform,
  () => {
    revealedSecrets.value = new Set();
  },
);

function toggleRevealed(key: string) {
  if (revealedSecrets.value.has(key)) {
    revealedSecrets.value.delete(key);
  } else {
    revealedSecrets.value.add(key);
  }
}

function setField(field: WizardFormField, value: unknown) {
  props.config[field.key] = coerceChannelFieldValue(field, value);
}

function setStringList(field: WizardFormField, text: string) {
  props.config[field.key] = splitIdList(text);
}

function setBoolean(field: WizardFormField, next: boolean) {
  props.config[field.key] = next;
}

function fieldValue(field: WizardFormField): unknown {
  return props.config[field.key] ?? coerceChannelFieldValue(field, undefined);
}

function booleanValue(field: WizardFormField): boolean {
  return Boolean(fieldValue(field));
}
</script>

<template>
  <div class="channel-editor-form">
    <div v-if="!hasSchema && extraKeys.length === 0" class="channel-editor-empty">
      {{ t('channels.noEditableFields') }}
    </div>

    <div v-if="basicFields.length" class="credential-form">
      <div v-for="field in basicFields" :key="field.key" class="credential-field">
        <label v-if="field.type !== 'boolean'" class="credential-label">
          {{ field.label }}
          <span v-if="field.required" class="required-mark">*</span>
        </label>

        <select
          v-if="field.type === 'select'"
          class="credential-input"
          :value="String(fieldValue(field) ?? '')"
          @change="setField(field, ($event.target as HTMLSelectElement).value)"
        >
          <option v-for="opt in field.options" :key="opt.value" :value="opt.value">
            {{ opt.label }}
          </option>
        </select>

        <textarea
          v-else-if="field.type === 'textarea' || field.type === 'string-list'"
          class="credential-input"
          :value="field.type === 'string-list' ? joinIdList(fieldValue(field)) : String(fieldValue(field) ?? '')"
          :placeholder="field.placeholder"
          rows="3"
          @input="
            field.type === 'string-list'
              ? setStringList(field, ($event.target as HTMLTextAreaElement).value)
              : setField(field, ($event.target as HTMLTextAreaElement).value)
          "
        />

        <div v-else-if="field.type === 'boolean'" class="channels-config-card switch-row">
          <span class="text-sm font-medium" style="color: var(--text);">{{ field.label }}</span>
          <button
            type="button"
            role="switch"
            :aria-checked="booleanValue(field)"
            class="channels-toggle"
            :class="{ enabled: booleanValue(field) }"
            @click="setBoolean(field, !booleanValue(field))"
          >
            <span class="channels-toggle-thumb" />
          </button>
        </div>

        <div v-else-if="field.type === 'password'" class="credential-input-wrapper">
          <input
            class="credential-input"
            :type="revealedSecrets.has(field.key) ? 'text' : 'password'"
            :value="String(fieldValue(field) ?? '')"
            :placeholder="field.placeholder"
            autocomplete="off"
            @input="setField(field, ($event.target as HTMLInputElement).value)"
          />
          <button
            v-if="field.secret"
            type="button"
            class="input-toggle"
            :title="revealedSecrets.has(field.key) ? t('channels.hideSecret') : t('channels.showSecret')"
            @click="toggleRevealed(field.key)"
          >
            <EyeOff v-if="revealedSecrets.has(field.key)" :size="16" />
            <Eye v-else :size="16" />
          </button>
        </div>

        <input
          v-else
          class="credential-input"
          :type="field.type === 'number' ? 'number' : 'text'"
          :value="fieldValue(field) ?? ''"
          :placeholder="field.placeholder"
          @input="
            setField(
              field,
              field.type === 'number'
                ? ($event.target as HTMLInputElement).value === ''
                  ? null
                  : Number(($event.target as HTMLInputElement).value)
                : ($event.target as HTMLInputElement).value,
            )
          "
        />

        <p v-if="field.hint && field.type !== 'boolean'" class="credential-hint">{{ field.hint }}</p>
      </div>
    </div>

    <details v-if="advancedFields.length" class="channels-details channel-editor-advanced">
      <summary>{{ t('channels.advancedSettings') }}</summary>
      <div class="credential-form mt-3">
        <div v-for="field in advancedFields" :key="field.key" class="credential-field">
          <label v-if="field.type !== 'boolean'" class="credential-label">
            {{ field.label }}
            <span v-if="field.required" class="required-mark">*</span>
          </label>

          <select
            v-if="field.type === 'select'"
            class="credential-input"
            :value="String(fieldValue(field) ?? '')"
            @change="setField(field, ($event.target as HTMLSelectElement).value)"
          >
            <option v-for="opt in field.options" :key="opt.value" :value="opt.value">
              {{ opt.label }}
            </option>
          </select>

          <textarea
            v-else-if="field.type === 'textarea' || field.type === 'string-list'"
            class="credential-input"
            :value="field.type === 'string-list' ? joinIdList(fieldValue(field)) : String(fieldValue(field) ?? '')"
            :placeholder="field.placeholder"
            rows="3"
            @input="
              field.type === 'string-list'
                ? setStringList(field, ($event.target as HTMLTextAreaElement).value)
                : setField(field, ($event.target as HTMLTextAreaElement).value)
            "
          />

          <div v-else-if="field.type === 'boolean'" class="channels-config-card switch-row">
            <span class="text-sm font-medium" style="color: var(--text);">{{ field.label }}</span>
            <button
              type="button"
              role="switch"
              :aria-checked="booleanValue(field)"
              class="channels-toggle"
              :class="{ enabled: booleanValue(field) }"
              @click="setBoolean(field, !booleanValue(field))"
            >
              <span class="channels-toggle-thumb" />
            </button>
          </div>

          <div v-else-if="field.type === 'password'" class="credential-input-wrapper">
            <input
              class="credential-input"
              :type="revealedSecrets.has(field.key) ? 'text' : 'password'"
              :value="String(fieldValue(field) ?? '')"
              :placeholder="field.placeholder"
              autocomplete="off"
              @input="setField(field, ($event.target as HTMLInputElement).value)"
            />
            <button
              v-if="field.secret"
              type="button"
              class="input-toggle"
              :title="revealedSecrets.has(field.key) ? t('channels.hideSecret') : t('channels.showSecret')"
              @click="toggleRevealed(field.key)"
            >
              <EyeOff v-if="revealedSecrets.has(field.key)" :size="16" />
              <Eye v-else :size="16" />
            </button>
          </div>

          <input
            v-else
            class="credential-input"
            :type="field.type === 'number' ? 'number' : 'text'"
            :value="fieldValue(field) ?? ''"
            :placeholder="field.placeholder"
            @input="
              setField(
                field,
                field.type === 'number'
                  ? ($event.target as HTMLInputElement).value === ''
                    ? null
                    : Number(($event.target as HTMLInputElement).value)
                  : ($event.target as HTMLInputElement).value,
              )
            "
          />

          <p v-if="field.hint && field.type !== 'boolean'" class="credential-hint">{{ field.hint }}</p>
        </div>
      </div>
    </details>

    <div v-if="!hasSchema && extraKeys.length" class="credential-form">
      <div v-for="key in extraKeys" :key="key" class="credential-field">
        <label class="credential-label">{{ key }}</label>
        <textarea
          class="credential-input"
          :value="typeof config[key] === 'string' ? config[key] : JSON.stringify(config[key] ?? '', null, 2)"
          rows="3"
          @input="
            (() => {
              const raw = ($event.target as HTMLTextAreaElement).value;
              try {
                config[key] = JSON.parse(raw);
              } catch {
                config[key] = raw;
              }
            })()
          "
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.channel-editor-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  min-width: 0;
}

.channel-editor-empty {
  font-size: 0.875rem;
  color: var(--text-muted);
}

.credential-form {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.credential-field {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 0;
}

.credential-label {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text);
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.required-mark {
  color: var(--danger);
}

.credential-input,
.credential-input-wrapper {
  width: 100%;
}

.credential-input {
  width: 100%;
  padding: 0.75rem 1rem;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel);
  color: var(--text);
  font-size: 0.875rem;
  transition: all 0.15s ease;
}

.credential-input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-glow);
}

.credential-input-wrapper {
  position: relative;
}

.credential-input-wrapper .credential-input {
  padding-right: 3rem;
}

.input-toggle {
  position: absolute;
  right: 0.75rem;
  top: 50%;
  transform: translateY(-50%);
  width: 24px;
  height: 24px;
  border-radius: 4px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}

.input-toggle:hover {
  background: var(--accent-bg-light);
  color: var(--accent);
}

.credential-hint {
  font-size: 0.75rem;
  color: var(--text-muted);
  margin-top: 0.25rem;
}

.channel-editor-advanced {
  margin-top: 0.25rem;
}
</style>

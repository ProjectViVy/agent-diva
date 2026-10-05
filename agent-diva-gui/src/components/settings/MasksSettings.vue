<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Plus, RefreshCw, Trash2 } from '@lucide/vue';
import { useI18n } from 'vue-i18n';

import { isTauriRuntime } from '../../api/desktop';
import {
  createMask,
  deleteMask,
  getMask,
  getMaskSelection,
  listAllMasks,
  MaskActionError,
  setMaskSelection,
  updateMask,
  type MaskDefinition,
  type MaskSelection,
} from '../../api/masks';

const { t } = useI18n();

const props = defineProps<{
  currentSessionKey?: string;
}>();

const previewMode = computed(() => !isTauriRuntime());
const masks = ref<MaskDefinition[]>([]);
const selection = ref<MaskSelection | null>(null);
const loading = ref(false);
const error = ref('');
const unavailable = ref(false);

// Editor draft; null means collapsed.
const editing = ref<{
  id: string | null; // null => creating
  revision: number;
  name: string;
  description: string;
  body: string;
} | null>(null);
const saving = ref(false);

const sessionId = computed(() => props.currentSessionKey?.trim() || '');

async function refresh() {
  if (previewMode.value) {
    masks.value = [];
    selection.value = null;
    return;
  }
  loading.value = true;
  error.value = '';
  try {
    masks.value = await listAllMasks();
    unavailable.value = false;
    if (sessionId.value) {
      selection.value = await getMaskSelection(sessionId.value);
    } else {
      selection.value = null;
    }
  } catch (err) {
    if (err instanceof MaskActionError && err.code === null) {
      unavailable.value = true; // method not configured in this Generation
    }
    error.value = String(err instanceof Error ? err.message : err);
  } finally {
    loading.value = false;
  }
}

function conflictMessage(err: unknown): string {
  if (err instanceof MaskActionError) {
    if (err.code === 'revision_conflict') return t('masks.conflict');
    if (err.code === 'mask_in_use') return t('masks.inUse', { count: err.referenceCount ?? 0 });
    if (err.code === 'not_found') return t('masks.notFound');
  }
  return String(err instanceof Error ? err.message : err);
}

async function select(mask: MaskDefinition) {
  if (!sessionId.value || saving.value) return;
  saving.value = true;
  error.value = '';
  try {
    const current = selection.value;
    selection.value = await setMaskSelection(
      sessionId.value,
      mask.id,
      current?.revision ?? 0,
    );
  } catch (err) {
    error.value = conflictMessage(err);
    if (err instanceof MaskActionError && err.code === 'revision_conflict') {
      selection.value = await getMaskSelection(sessionId.value);
    }
  } finally {
    saving.value = false;
  }
}

function startCreate() {
  editing.value = { id: null, revision: 0, name: '', description: '', body: '' };
}

async function startEdit(mask: MaskDefinition) {
  error.value = '';
  try {
    const fresh = await getMask(mask.id); // pin current revision for the edit
    editing.value = {
      id: fresh.id,
      revision: fresh.revision,
      name: fresh.name,
      description: fresh.description,
      body: fresh.body,
    };
  } catch (err) {
    error.value = conflictMessage(err);
    await refresh();
  }
}

async function saveEdit() {
  const draft = editing.value;
  if (!draft || saving.value) return;
  saving.value = true;
  error.value = '';
  try {
    if (draft.id === null) {
      await createMask({ name: draft.name, description: draft.description, body: draft.body });
    } else {
      await updateMask({
        id: draft.id,
        expectedRevision: draft.revision,
        name: draft.name,
        description: draft.description,
        body: draft.body,
      });
    }
    editing.value = null;
    await refresh();
  } catch (err) {
    const msg = conflictMessage(err);
    if (err instanceof MaskActionError && err.code === 'revision_conflict') {
      editing.value = null;
      await refresh();
    }
    error.value = msg; // refresh() clears error — reassert after resync
  } finally {
    saving.value = false;
  }
}

async function remove(mask: MaskDefinition) {
  if (mask.built_in || saving.value) return;
  saving.value = true;
  error.value = '';
  try {
    await deleteMask(mask.id, mask.revision);
    await refresh();
  } catch (err) {
    const msg = conflictMessage(err);
    if (err instanceof MaskActionError && err.code === 'revision_conflict') {
      await refresh();
    }
    error.value = msg;
  } finally {
    saving.value = false;
  }
}

onMounted(refresh);
</script>

<template>
  <div class="settings-section">
    <div class="settings-section-header">
      <h3>{{ t('masks.title') }}</h3>
      <button class="settings-icon-btn" :disabled="loading" @click="refresh">
        <RefreshCw :size="16" />
      </button>
    </div>
    <p class="settings-hint">{{ t('masks.hint') }}</p>

    <div v-if="previewMode" class="settings-empty">{{ t('masks.preview') }}</div>
    <div v-else-if="unavailable" class="settings-empty">{{ t('masks.unavailable') }}</div>
    <template v-else>
      <p v-if="!sessionId" class="settings-hint">{{ t('masks.noSession') }}</p>
      <p v-else-if="selection && !selection.available" class="settings-hint">
        {{ t('masks.selectionInactive', { reason: selection.inactive_reason || '-' }) }}
      </p>
      <p v-if="error" class="settings-error">{{ error }}</p>

      <ul class="masks-list">
        <li
          v-for="mask in masks"
          :key="mask.id"
          class="masks-item"
          :class="{ active: selection?.mask_id === mask.id }"
        >
          <button
            class="masks-item-main"
            :disabled="!sessionId || saving"
            @click="select(mask)"
          >
            <span class="masks-item-name">
              {{ mask.name }}
              <span v-if="mask.built_in" class="masks-badge">{{ t('masks.builtIn') }}</span>
              <span v-if="selection?.mask_id === mask.id" class="masks-badge active">
                {{ t('masks.active') }}
              </span>
            </span>
            <span class="masks-item-desc">{{ mask.description }}</span>
          </button>
          <div class="masks-item-actions">
            <button
              class="settings-icon-btn"
              :disabled="saving"
              :title="t('masks.edit')"
              @click="startEdit(mask)"
            >✎</button>
            <button
              v-if="!mask.built_in"
              class="settings-icon-btn"
              :disabled="saving"
              :title="t('masks.delete')"
              @click="remove(mask)"
            >
              <Trash2 :size="14" />
            </button>
          </div>
        </li>
      </ul>

      <button class="settings-action-btn" :disabled="saving" @click="startCreate">
        <Plus :size="14" /> {{ t('masks.create') }}
      </button>

      <form v-if="editing" class="masks-editor" @submit.prevent="saveEdit">
        <input
          v-model="editing.name"
          class="settings-input"
          :placeholder="t('masks.namePlaceholder')"
          maxlength="128"
          required
        />
        <input
          v-model="editing.description"
          class="settings-input"
          :placeholder="t('masks.descPlaceholder')"
          maxlength="1024"
        />
        <textarea
          v-model="editing.body"
          class="settings-input masks-body"
          :placeholder="t('masks.bodyPlaceholder')"
          maxlength="16384"
          required
        />
        <div class="masks-editor-actions">
          <button type="submit" class="settings-action-btn" :disabled="saving">
            {{ t('masks.save') }}
          </button>
          <button type="button" class="settings-action-btn secondary" @click="editing = null">
            {{ t('masks.cancel') }}
          </button>
        </div>
      </form>
    </template>
  </div>
</template>

<style scoped>
.settings-section { padding: 16px 24px; max-width: 720px; }
.settings-section-header { display: flex; align-items: center; gap: 8px; }
.settings-hint { color: var(--text-muted); font-size: 0.8rem; }
.settings-error { color: var(--danger, #d34); font-size: 0.8rem; }
.settings-empty { color: var(--text-muted); padding: 16px 0; }
.masks-list { list-style: none; padding: 0; margin: 12px 0; display: flex; flex-direction: column; gap: 6px; }
.masks-item { display: flex; align-items: stretch; border: 1px solid var(--line); border-radius: 8px; }
.masks-item.active { border-color: var(--accent, #7c6cf0); }
.masks-item-main { flex: 1; text-align: left; padding: 8px 12px; background: none; border: none; cursor: pointer; color: var(--text); }
.masks-item-main:disabled { cursor: default; opacity: 0.7; }
.masks-item-name { display: flex; gap: 8px; align-items: center; font-weight: 600; font-size: 0.85rem; }
.masks-item-desc { display: block; color: var(--text-muted); font-size: 0.75rem; margin-top: 2px; }
.masks-badge { font-size: 0.65rem; padding: 1px 6px; border-radius: 999px; border: 1px solid var(--line); color: var(--text-muted); }
.masks-badge.active { color: var(--accent, #7c6cf0); border-color: var(--accent, #7c6cf0); }
.masks-item-actions { display: flex; align-items: center; gap: 4px; padding-right: 8px; }
.settings-icon-btn { background: none; border: none; color: var(--text-muted); cursor: pointer; padding: 4px; }
.settings-icon-btn:hover { color: var(--text); }
.settings-action-btn { display: inline-flex; align-items: center; gap: 6px; padding: 6px 12px; border-radius: 6px; border: 1px solid var(--line); background: var(--nav-hover); color: var(--text); cursor: pointer; font-size: 0.8rem; }
.settings-action-btn.secondary { background: transparent; }
.settings-input { width: 100%; padding: 6px 10px; border: 1px solid var(--line); border-radius: 6px; background: var(--bg); color: var(--text); font-size: 0.8rem; }
.masks-editor { display: flex; flex-direction: column; gap: 8px; margin-top: 12px; }
.masks-body { min-height: 160px; font-family: monospace; }
.masks-editor-actions { display: flex; gap: 8px; }
</style>

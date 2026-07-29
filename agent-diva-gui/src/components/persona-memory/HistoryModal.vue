<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  ref,
  watch,
} from 'vue';
import { useI18n } from 'vue-i18n';
import { X } from '@lucide/vue';
import { listLaputaChangelog, type ChangelogRecord } from '../../api/desktop';

const { t } = useI18n();

import type { LaputaSectionName } from '../../api/desktop';

const props = defineProps<{
  sectionName: LaputaSectionName;
  open: boolean;
}>();

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const records = ref<ChangelogRecord[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const copiedId = ref<string | null>(null);

const sectionDisplayName = computed(() => t('laputa.sections.' + props.sectionName));
const modalCardRef = ref<HTMLElement | null>(null);
const previousFocusRef = ref<Element | null>(null);
const titleId = `history-modal-title-${Math.random().toString(36).slice(2)}`;

watch(() => props.open, async (isOpen) => {
  if (!isOpen) return;
  previousFocusRef.value = document.activeElement;
  loading.value = true;
  error.value = null;
  try {
    const page = await listLaputaChangelog({ section: props.sectionName, limit: 50 });
    records.value = page.items.slice(0, 50);
    await nextTick();
    focusFirst();
    attachFocusTrap();
  } catch (err) {
    error.value = String(err);
  } finally {
    loading.value = false;
  }
}, { immediate: true });

function close() {
  emit('close');
}

function restoreFocus() {
  const target = previousFocusRef.value;
  if (target instanceof HTMLElement) {
    nextTick(() => target.focus());
  }
}

function onScrimClick(event: MouseEvent) {
  if (event.target === event.currentTarget) {
    close();
  }
}

function onKeyDown(event: KeyboardEvent) {
  if (event.key === 'Escape' && props.open) {
    event.preventDefault();
    close();
    return;
  }
  if (event.key !== 'Tab' || !modalCardRef.value) return;
  const focusable = getFocusable(modalCardRef.value);
  if (focusable.length === 0) return;
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

function getFocusable(root: HTMLElement): HTMLElement[] {
  const selector =
    'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])';
  return Array.from(root.querySelectorAll<HTMLElement>(selector)).filter(
    (el) => !('disabled' in el && (el as HTMLButtonElement).disabled),
  );
}

function focusFirst() {
  if (!modalCardRef.value) return;
  const focusable = getFocusable(modalCardRef.value);
  focusable[0]?.focus();
}

let trapAttached = false;
function attachFocusTrap() {
  if (trapAttached) return;
  document.addEventListener('keydown', onKeyDown);
  trapAttached = true;
}

function detachFocusTrap() {
  if (!trapAttached) return;
  document.removeEventListener('keydown', onKeyDown);
  trapAttached = false;
}

onBeforeUnmount(detachFocusTrap);

watch(() => props.open, (isOpen) => {
  if (!isOpen) {
    detachFocusTrap();
    restoreFocus();
  }
});

function formatDate(value: string): string {
  try {
    return new Date(value).toLocaleString();
  } catch {
    return value;
  }
}

function excerpt(text: string): string {
  const normalized = text.replace(/\s+/g, ' ').trim();
  if (normalized.length <= 120) return normalized;
  return normalized.slice(0, 120) + '…';
}

async function copyAfter(record: ChangelogRecord) {
  try {
    await navigator.clipboard.writeText(record.after);
    copiedId.value = record.id;
    window.setTimeout(() => {
      copiedId.value = null;
    }, 1000);
  } catch {
    // silently ignore clipboard errors
  }
}

async function retry() {
  loading.value = true;
  error.value = null;
  try {
    const page = await listLaputaChangelog({ section: props.sectionName, limit: 50 });
    records.value = page.items.slice(0, 50);
  } catch (err) {
    error.value = String(err);
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal-fade">
      <div
        v-if="open"
        class="history-modal-scrim"
        @click.self="onScrimClick"
      >
        <div
          ref="modalCardRef"
          class="history-modal-card"
          role="dialog"
          aria-modal="true"
          :aria-labelledby="titleId"
        >
          <header class="history-modal-header">
            <h2 :id="titleId" class="history-modal-title">
              {{ t('laputa.historyModal.title', { section: sectionDisplayName }) }}
            </h2>
            <button
              type="button"
              class="history-modal-close"
              :aria-label="t('laputa.historyModal.close')"
              @click="close"
            >
              <X :size="18" />
            </button>
          </header>

          <div class="history-modal-body">
            <div v-if="loading" class="history-modal-loading">
              {{ t('app.loading') }}
            </div>

            <div v-else-if="error" class="history-modal-error" role="alert">
              <span>{{ t('laputa.historyModal.loadError') }}: {{ error }}</span>
              <button type="button" class="history-modal-retry" @click="retry">
                {{ t('laputa.historyModal.retry') }}
              </button>
            </div>

            <div v-else-if="records.length === 0" class="history-modal-empty">
              {{ t('laputa.historyModal.empty') }}
            </div>

            <ul v-else class="history-modal-list" role="list">
              <li
                v-for="record in records"
                :key="record.id"
                class="history-modal-item"
              >
                <div class="history-modal-meta">
                  <span class="history-modal-time">{{ formatDate(record.created_at) }}</span>
                  <span class="history-modal-badge">{{ record.action }}</span>
                  <span class="history-modal-actor">{{ record.applied_by }}</span>
                </div>
                <p class="history-modal-excerpt">{{ excerpt(record.after) }}</p>
                <button
                  type="button"
                  class="history-modal-copy"
                  @click="copyAfter(record)"
                >
                  {{ copiedId === record.id ? t('laputa.historyModal.copied') : t('laputa.historyModal.copy') }}
                </button>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.history-modal-scrim {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(2px);
  z-index: 600;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
}

.history-modal-card {
  width: 100%;
  max-width: 640px;
  background: var(--panel-solid);
  border-radius: var(--radius);
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.22);
  display: flex;
  flex-direction: column;
  max-height: calc(100vh - 48px);
  border: 1px solid var(--line);
}

.history-modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 16px;
  border-bottom: 1px solid var(--line);
  flex-shrink: 0;
}

.history-modal-title {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
  color: var(--text);
}

.history-modal-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--line);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.15s ease;
}

.history-modal-close:hover {
  background: var(--accent-bg-light);
  color: var(--text);
}

.history-modal-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 16px;
}

.history-modal-loading,
.history-modal-empty {
  text-align: center;
  padding: 32px 16px;
  color: var(--text-muted);
  font-size: 0.875rem;
}

.history-modal-error {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 24px 16px;
  color: var(--danger);
  font-size: 0.875rem;
  text-align: center;
}

.history-modal-retry {
  padding: 6px 14px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--accent-border);
  background: var(--accent);
  color: #fff;
  font-size: 13px;
  cursor: pointer;
}

.history-modal-list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.history-modal-item {
  border-bottom: 1px solid var(--line);
  padding: 12px 0;
}

.history-modal-item:last-child {
  border-bottom: none;
}

.history-modal-meta {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  margin-bottom: 6px;
}

.history-modal-time,
.history-modal-actor {
  font-size: 0.75rem;
  color: var(--text-muted);
}

.history-modal-badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  background: var(--accent-bg-light);
  color: var(--accent);
  font-size: 0.75rem;
  font-weight: 500;
  text-transform: uppercase;
}

.history-modal-excerpt {
  margin: 0 0 10px;
  font-size: 0.875rem;
  line-height: 1.5;
  color: var(--text);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  white-space: pre-wrap;
}

.history-modal-copy {
  padding: 5px 12px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--line);
  background: var(--panel);
  color: var(--text);
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.history-modal-copy:hover {
  border-color: var(--accent-border);
  background: var(--accent-bg-light);
}

.history-modal-copy:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--accent-glow), 0 0 0 4px var(--accent);
}

.history-modal-retry:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--accent-glow), 0 0 0 4px var(--accent);
}

.modal-fade-enter-active,
.modal-fade-leave-active {
  transition: opacity 0.2s ease;
}

.modal-fade-enter-from,
.modal-fade-leave-to {
  opacity: 0;
}
</style>

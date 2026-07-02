<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import StructuredEventsTab from './StructuredEventsTab.vue';
import RawLogTab from './RawLogTab.vue';
import type { AuditEvent } from './types';

const activeTab = ref<'structured' | 'raw'>('structured');
const selectedDate = ref(new Date().toISOString().slice(0, 10));
const events = ref<AuditEvent[]>([]);
const rawLines = ref<string[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

async function loadEvents() {
  loading.value = true;
  error.value = null;
  try {
    events.value = await invoke('get_audit_events', { date: selectedDate.value });
  } catch (e) {
    error.value = String(e);
    events.value = [];
  } finally {
    loading.value = false;
  }
}

async function loadRawLog() {
  loading.value = true;
  error.value = null;
  try {
    rawLines.value = await invoke('get_raw_log_lines', {
      date: selectedDate.value,
      maxLines: 500,
    });
  } catch (e) {
    error.value = String(e);
    rawLines.value = [];
  } finally {
    loading.value = false;
  }
}

function onTabChange(tab: 'structured' | 'raw') {
  activeTab.value = tab;
  if (tab === 'structured') loadEvents();
  else loadRawLog();
}

function onDateChange(event: Event) {
  const input = event.target as HTMLInputElement;
  selectedDate.value = input.value;
  if (activeTab.value === 'structured') loadEvents();
  else loadRawLog();
}

function onTabKeydown(event: KeyboardEvent, tab: 'structured' | 'raw') {
  if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
    const next = tab === 'structured' ? 'raw' : 'structured';
    onTabChange(next);
    // Focus the next tab button
    const buttons = (event.currentTarget as HTMLElement).parentElement?.querySelectorAll('.audit-tab');
    if (buttons) {
      const nextIdx = next === 'structured' ? 0 : 1;
      (buttons[nextIdx] as HTMLElement)?.focus();
    }
  }
}

function onRawLogRefresh() {
  loadRawLog();
}

onMounted(() => {
  loadEvents();
});
</script>

<template>
  <div class="audit-page" role="region" aria-label="Audit Log">
    <!-- Header -->
    <div class="audit-header">
      <h2 class="audit-title">Audit Log</h2>
      <input
        type="date"
        :value="selectedDate"
        @change="onDateChange"
        class="audit-date-picker"
        aria-label="Select date for audit log"
      />
    </div>

    <!-- Tab Bar -->
    <div class="audit-tabs" role="tablist" aria-label="Audit view mode">
      <button
        role="tab"
        :aria-selected="activeTab === 'structured'"
        :tabindex="activeTab === 'structured' ? 0 : -1"
        :class="['audit-tab', { active: activeTab === 'structured' }]"
        @click="onTabChange('structured')"
        @keydown="onTabKeydown($event, 'structured')"
      >
        Structured Events
      </button>
      <button
        role="tab"
        :aria-selected="activeTab === 'raw'"
        :tabindex="activeTab === 'raw' ? 0 : -1"
        :class="['audit-tab', { active: activeTab === 'raw' }]"
        @click="onTabChange('raw')"
        @keydown="onTabKeydown($event, 'raw')"
      >
        Raw Log
      </button>
    </div>

    <!-- Error State -->
    <div v-if="error" class="audit-error" role="alert">
      <span class="audit-error-icon" aria-hidden="true">!</span>
      <span>{{ error }}</span>
    </div>

    <!-- Tab Content -->
    <div class="audit-content">
      <StructuredEventsTab
        v-show="activeTab === 'structured'"
        :events="events"
        :loading="loading"
        :selected-date="selectedDate"
      />
      <RawLogTab
        v-show="activeTab === 'raw'"
        :lines="rawLines"
        :loading="loading"
        @refresh="onRawLogRefresh"
      />
    </div>
  </div>
</template>

<style scoped>
.audit-page {
  padding: 24px;
  background: var(--bg-app);
  color: var(--text);
  min-height: 100%;
}

.audit-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.audit-title {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
}

.audit-date-picker {
  padding: 8px 12px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel);
  color: var(--text);
  font-size: 13px;
  outline: none;
  transition: border-color 0.15s ease;
}

.audit-date-picker:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-glow);
}

.audit-tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 16px;
  border-bottom: 1px solid var(--line);
}

.audit-tab {
  padding: 10px 20px;
  border: none;
  border-bottom: 2px solid transparent;
  margin-bottom: -1px;
  background: transparent;
  color: var(--text-muted);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  outline: none;
}

.audit-tab:hover {
  color: var(--text);
  background: var(--accent-bg-light);
}

.audit-tab:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}

.audit-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.audit-error {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  border-radius: var(--radius-sm);
  background: var(--danger-bg, rgba(239, 68, 68, 0.1));
  color: var(--danger, #ef4444);
  font-size: 13px;
  margin-bottom: 16px;
}

.audit-error-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--danger, #ef4444);
  color: white;
  font-size: 12px;
  font-weight: bold;
  flex-shrink: 0;
}

.audit-content {
  flex: 1;
}
</style>

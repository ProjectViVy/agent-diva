<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import StructuredEventsTab from './StructuredEventsTab.vue';
import GatewayBackendLogTab from './RawLogTab.vue';
import GuiLogTab from './GuiLogTab.vue';
import type { AuditEvent } from './types';

const { t } = useI18n();
const activeTab = ref<'structured' | 'gateway' | 'gui'>('structured');
const selectedDate = ref(new Date().toISOString().slice(0, 10));
const events = ref<AuditEvent[]>([]);
const gatewayLines = ref<string[]>([]);
const guiLines = ref<string[]>([]);
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

async function loadGatewayLog() {
  loading.value = true;
  error.value = null;
  try {
    gatewayLines.value = await invoke('get_gateway_log_lines', {
      date: selectedDate.value,
      maxLines: 500,
    });
  } catch (e) {
    error.value = String(e);
    gatewayLines.value = [];
  } finally {
    loading.value = false;
  }
}

async function loadGuiLog() {
  loading.value = true;
  error.value = null;
  try {
    guiLines.value = await invoke('get_gui_log_lines', { date: selectedDate.value, maxLines: 500 });
  } catch (e) {
    error.value = String(e);
    guiLines.value = [];
  } finally {
    loading.value = false;
  }
}

function onTabChange(tab: 'structured' | 'gateway' | 'gui') {
  activeTab.value = tab;
  if (tab === 'structured') loadEvents();
  else if (tab === 'gateway') loadGatewayLog();
  else loadGuiLog();
}

function onDateChange(event: Event) {
  const input = event.target as HTMLInputElement;
  selectedDate.value = input.value;
  if (activeTab.value === 'structured') loadEvents();
  else if (activeTab.value === 'gateway') loadGatewayLog();
  else loadGuiLog();
}

function onTabKeydown(event: KeyboardEvent, tab: 'structured' | 'gateway' | 'gui') {
  if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
    const tabs: Array<'structured' | 'gateway' | 'gui'> = ['structured', 'gateway', 'gui'];
    const delta = event.key === 'ArrowRight' ? 1 : -1;
    const next = tabs[(tabs.indexOf(tab) + delta + tabs.length) % tabs.length];
    onTabChange(next);
    // Focus the next tab button
    const buttons = (event.currentTarget as HTMLElement).parentElement?.querySelectorAll('.audit-tab');
    if (buttons) {
      const nextIdx = tabs.indexOf(next);
      (buttons[nextIdx] as HTMLElement)?.focus();
    }
  }
}

function onGatewayLogRefresh() {
  loadGatewayLog();
}

function onGuiLogRefresh() {
  loadGuiLog();
}

onMounted(() => {
  loadEvents();
});
</script>

<template>
  <div class="audit-page" role="region" :aria-label="t('auditPage.regionLabel')">
    <!-- Header -->
    <div class="audit-header">
      <h2 class="audit-title">{{ t('auditPage.title') }}</h2>
      <input
        type="date"
        :value="selectedDate"
        @change="onDateChange"
        class="audit-date-picker"
        :aria-label="t('auditPage.datePickerLabel')"
      />
    </div>

    <!-- Tab Bar -->
    <div class="audit-tabs" role="tablist" :aria-label="t('auditPage.tabListLabel')">
      <button
        role="tab"
        :aria-selected="activeTab === 'structured'"
        :tabindex="activeTab === 'structured' ? 0 : -1"
        :class="['audit-tab', { active: activeTab === 'structured' }]"
        data-testid="audit-events-tab"
        @click="onTabChange('structured')"
        @keydown="onTabKeydown($event, 'structured')"
      >
        {{ t('auditPage.tabs.structured') }}
      </button>
      <button
        role="tab"
        :aria-selected="activeTab === 'gateway'"
        :tabindex="activeTab === 'gateway' ? 0 : -1"
        :class="['audit-tab', { active: activeTab === 'gateway' }]"
        data-testid="gateway-backend-logs-tab"
        @click="onTabChange('gateway')"
        @keydown="onTabKeydown($event, 'gateway')"
      >
        {{ t('auditPage.tabs.gateway') }}
      </button>
      <button
        role="tab"
        :aria-selected="activeTab === 'gui'"
        :tabindex="activeTab === 'gui' ? 0 : -1"
        :class="['audit-tab', { active: activeTab === 'gui' }]"
        data-testid="frontend-gui-logs-tab"
        @click="onTabChange('gui')"
        @keydown="onTabKeydown($event, 'gui')"
      >
        {{ t('auditPage.tabs.gui') }}
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
      <GatewayBackendLogTab
        v-show="activeTab === 'gateway'"
        :lines="gatewayLines"
        :loading="loading"
        @refresh="onGatewayLogRefresh"
      />
      <GuiLogTab
        v-show="activeTab === 'gui'"
        :lines="guiLines"
        :loading="loading"
        @refresh="onGuiLogRefresh"
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

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { AuditEvent, AuditEventType } from './types';
import { EVENT_LABELS, EVENT_ICONS } from './types';

const { t } = useI18n();

defineProps<{
  events: AuditEvent[];
  loading: boolean;
  selectedDate: string;
}>();

function formatTime(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  } catch {
    return iso;
  }
}

function getEventLabel(type: AuditEventType): string {
  return EVENT_LABELS[type] || type;
}

function getEventIcon(type: AuditEventType): string {
  return EVENT_ICONS[type] || '馃搵';
}

function getEventDataPreview(data: Record<string, unknown>): string {
  const entries = Object.entries(data).slice(0, 3);
  if (entries.length === 0) return '';
  return entries
    .map(([k, v]) => `${k}: ${typeof v === 'object' ? JSON.stringify(v) : String(v)}`)
    .join(', ');
}
</script>

<template>
  <div
    class="structured-events-tab"
    role="tabpanel"
    :aria-label="t('auditPage.structured.panelLabel')"
    :aria-busy="loading"
  >
    <div v-if="loading && events.length === 0" class="skeleton-container" aria-hidden="true">
      <div
        v-for="i in 5"
        :key="i"
        class="skeleton-row"
      >
        <div class="skeleton-cell skeleton-icon" />
        <div class="skeleton-cell skeleton-label" />
        <div class="skeleton-cell skeleton-time" />
        <div class="skeleton-cell skeleton-data" />
      </div>
    </div>

    <div
      v-else-if="!loading && events.length === 0"
      class="empty-state"
      role="status"
    >
      <span class="empty-icon" aria-hidden="true">馃搵</span>
      <p class="empty-title">{{ t('auditPage.structured.emptyTitle') }}</p>
      <p class="empty-hint">{{ t('auditPage.structured.emptyHint') }}</p>
    </div>

    <div
      v-else
      class="event-table-wrapper"
    >
      <div
        class="event-table"
        role="table"
        :aria-label="t('auditPage.structured.tableLabel')"
        :aria-rowcount="events.length"
      >
        <div class="event-table-header" role="row" aria-hidden="true">
          <span class="col-icon" role="columnheader">{{ t('auditPage.structured.columns.type') }}</span>
          <span class="col-label" role="columnheader">{{ t('auditPage.structured.columns.event') }}</span>
          <span class="col-time" role="columnheader">{{ t('auditPage.structured.columns.time') }}</span>
          <span class="col-data" role="columnheader">{{ t('auditPage.structured.columns.details') }}</span>
        </div>

        <div
          v-for="(event, idx) in events"
          :key="idx"
          class="event-row"
          role="row"
          tabindex="0"
          :aria-rowindex="idx + 1"
          :aria-label="`${getEventLabel(event.eventType)} at ${formatTime(event.timestamp)}`"
        >
          <span class="event-icon" role="cell" aria-hidden="true">
            {{ getEventIcon(event.eventType) }}
          </span>
          <span class="event-label" role="cell">
            {{ getEventLabel(event.eventType) }}
          </span>
          <span class="event-time" role="cell">
            {{ formatTime(event.timestamp) }}
          </span>
          <span class="event-data-preview" role="cell">
            {{ getEventDataPreview(event.data) }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.structured-events-tab {
  min-height: 200px;
}

.skeleton-container {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 4px 0;
}

.skeleton-row {
  display: grid;
  grid-template-columns: 28px 1fr 100px 1.5fr;
  gap: 12px;
  align-items: center;
  padding: 12px 8px;
  border-radius: var(--radius-sm, 8px);
}

.skeleton-cell {
  height: 14px;
  border-radius: 4px;
  background: var(--line, #e5e7eb);
  animation: skeleton-pulse 1.5s ease-in-out infinite;
}

.skeleton-icon {
  width: 20px;
  height: 20px;
  border-radius: 50%;
}

.skeleton-label {
  width: 80%;
}

.skeleton-time {
  width: 70%;
}

.skeleton-data {
  width: 60%;
}

@keyframes skeleton-pulse {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 0.8; }
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 64px 24px;
  text-align: center;
}

.empty-icon {
  font-size: 40px;
  margin-bottom: 16px;
  opacity: 0.5;
}

.empty-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-muted, #6b7280);
  margin: 0 0 4px;
}

.empty-hint {
  font-size: 12px;
  color: var(--text-muted, #9ca3af);
  margin: 0;
}

.event-table-wrapper {
  border: 1px solid var(--line, #e5e7eb);
  border-radius: var(--radius-sm, 8px);
  overflow: hidden;
}

.event-table-header {
  display: grid;
  grid-template-columns: 28px 1fr 100px 1.5fr;
  gap: 12px;
  padding: 10px 8px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-muted, #6b7280);
  background: var(--accent-bg-light, rgba(0,0,0,0.02));
  border-bottom: 1px solid var(--line, #e5e7eb);
}

.event-row {
  display: grid;
  grid-template-columns: 28px 1fr 100px 1.5fr;
  gap: 12px;
  align-items: center;
  padding: 10px 8px;
  font-size: 13px;
  color: var(--text, #111827);
  border-bottom: 1px solid var(--line, #e5e7eb);
  transition: background 0.1s ease;
  cursor: default;
  outline: none;
}

.event-row:last-child {
  border-bottom: none;
}

.event-row:hover {
  background: var(--accent-bg-light, rgba(0,0,0,0.02));
}

.event-row:focus-visible {
  outline: 2px solid var(--accent, #3b82f6);
  outline-offset: -2px;
  border-radius: 2px;
}

.event-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
}

.event-label {
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.event-time {
  font-size: 12px;
  color: var(--text-muted, #6b7280);
  font-variant-numeric: tabular-nums;
}

.event-data-preview {
  font-size: 12px;
  color: var(--text-muted, #6b7280);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: 'SF Mono', 'Fira Code', monospace;
}
</style>

<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { ChevronDown, ChevronRight, FileText } from '@lucide/vue';
import type { LaputaSectionName } from '../../api/desktop';

const { t } = useI18n();

interface SnapshotSection {
  status: 'owned' | 'tbd';
  last_modified?: string | null;
}

interface LaputaSnapshot {
  sections: Record<string, SnapshotSection>;
}

interface Props {
  snapshot: LaputaSnapshot | null;
  selectedSection: LaputaSectionName;
}

const props = defineProps<Props>();
const emit = defineEmits<{
  (e: 'select', sectionName: LaputaSectionName): void;
}>();

const GROUPS: { key: string; sections: LaputaSectionName[] }[] = [
  { key: 'persona', sections: ['identity', 'relationship', 'commitment', 'preferences'] },
  { key: 'memory', sections: ['memory_md', 'history_md'] },
  { key: 'indexes', sections: ['journal_reflective', 'proposal_inbox', 'changelog', 'report_indexes', 'aaak_summaries'] },
];

const expanded = ref<Record<string, boolean>>({
  persona: true,
  memory: true,
  indexes: true,
});

function toggleGroup(groupKey: string): void {
  expanded.value[groupKey] = !expanded.value[groupKey];
}

function selectSection(name: LaputaSectionName): void {
  emit('select', name);
}

function getSectionStatus(name: LaputaSectionName): 'owned' | 'tbd' {
  return props.snapshot?.sections[name]?.status ?? 'tbd';
}

function getSectionLastModified(name: LaputaSectionName): string | null | undefined {
  return props.snapshot?.sections[name]?.last_modified;
}

function formatDate(value?: string | null): string {
  if (!value) return '';
  try {
    return new Date(value).toLocaleString();
  } catch {
    return value;
  }
}
</script>

<template>
  <nav
    class="section-group-list"
    role="navigation"
    aria-label="Laputa sections"
  >
    <div
      v-for="group in GROUPS"
      :key="group.key"
      class="section-group"
    >
      <button
        type="button"
        class="group-header"
        :aria-expanded="expanded[group.key]"
        @click="toggleGroup(group.key)"
      >
        <span class="group-title">{{ t('laputa.groups.' + group.key) }}</span>
        <span class="group-chevron" aria-hidden="true">
          <ChevronDown v-if="expanded[group.key]" :size="16" />
          <ChevronRight v-else :size="16" />
        </span>
      </button>

      <div
        v-show="expanded[group.key]"
        class="group-items"
      >
        <button
          v-for="section in group.sections"
          :key="section"
          type="button"
          class="section-item"
          :class="{ 'section-item--active': section === selectedSection }"
          :aria-current="section === selectedSection ? 'true' : undefined"
          :aria-label="t('laputa.a11y.sectionItem', { name: t('laputa.sections.' + section) })"
          @click="selectSection(section)"
        >
          <FileText :size="15" aria-hidden="true" />
          <div class="section-meta">
            <span class="section-name">{{ t('laputa.sections.' + section) }}</span>
            <span class="section-key">{{ section }}</span>
            <span
              v-if="getSectionLastModified(section)"
              class="section-updated"
            >
              {{ formatDate(getSectionLastModified(section)) }}
            </span>
          </div>
          <span
            class="section-status"
            :class="[
              'section-status--' + getSectionStatus(section),
            ]"
          >
            {{ t('laputa.status.' + getSectionStatus(section)) }}
          </span>
        </button>
      </div>
    </div>
  </nav>
</template>

<style scoped>
.section-group-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px;
}

.section-group {
  display: flex;
  flex-direction: column;
}

.group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  width: 100%;
  padding: 10px 12px;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
  user-select: none;
  text-align: left;
}

.group-header:hover {
  background: var(--accent-bg-light);
}

.group-header:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--accent-glow), 0 0 0 4px var(--accent);
}

.group-title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.group-chevron {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: var(--text-muted);
  transition: transform 0.2s ease, color 0.15s ease;
}

.group-header:hover .group-chevron {
  color: var(--text);
}

.group-items {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 4px 0 8px 8px;
}

.section-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 10px 12px;
  border: none;
  border-left: 3px solid transparent;
  border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
  background: transparent;
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s ease;
  text-align: left;
}

.section-item:hover {
  background: var(--accent-bg-light);
}

.section-item:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--accent-glow), 0 0 0 4px var(--accent);
}

.section-item svg {
  flex-shrink: 0;
  color: var(--text-muted);
  transition: color 0.15s ease;
}

.section-item:hover svg {
  color: var(--accent);
}

.section-item--active {
  border-left-color: var(--accent);
  background: var(--panel-solid);
  color: var(--accent);
  box-shadow: 0 2px 8px var(--accent-glow);
}

.section-item--active svg {
  color: var(--accent);
}

.section-meta {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.section-name {
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.section-key {
  font-size: 11px;
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.section-updated {
  font-size: 11px;
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.section-item--active .section-key {
  color: var(--accent);
  opacity: 0.8;
}

.section-item--active .section-updated {
  color: var(--accent);
  opacity: 0.8;
}

.section-status {
  flex-shrink: 0;
  padding: 2px 8px;
  border-radius: 9999px;
  font-size: 0.625rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.02em;
}

.section-status--owned {
  background: var(--accent-bg-light);
  color: var(--accent);
  border: 1px solid var(--accent-border);
}

.section-status--tbd {
  background: transparent;
  color: var(--text-muted);
  border: 1px solid var(--line);
}
</style>

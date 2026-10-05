<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { RefreshCcw, CircleOff, Search, Power, PowerOff } from '@lucide/vue';
import { useI18n } from 'vue-i18n';

import { isTauriRuntime } from '../../api/desktop';
import {
  loadInstalledSkills,
  setSkillEnabled,
  type InstalledSkill,
} from '../../api/settings';

const { t } = useI18n();

const skills = ref<InstalledSkill[]>([]);
const loading = ref(false);
const toggling = ref('');
const error = ref('');
const searchQuery = ref('');
const previewMode = computed(() => !isTauriRuntime());

const filteredSkills = computed(() => {
  const q = searchQuery.value.toLowerCase().trim();
  if (!q) return skills.value;
  return skills.value.filter(
    (s) =>
      s.name.toLowerCase().includes(q) ||
      s.description.toLowerCase().includes(q)
  );
});

async function refreshSkills() {
  if (previewMode.value) {
    skills.value = [];
    error.value = '';
    return;
  }

  loading.value = true;
  error.value = '';
  try {
    skills.value = await loadInstalledSkills();
  } catch (err) {
    error.value = String(err);
  } finally {
    loading.value = false;
  }
}

async function onToggle(skill: InstalledSkill) {
  if (toggling.value) return;
  toggling.value = skill.name;
  error.value = '';
  try {
    await setSkillEnabled(skill.name, !skill.enabled, skill.hash);
    await refreshSkills();
  } catch (err) {
    // -32009 conflict: the document changed under us — re-list, then surface it.
    const msg = String(err);
    await refreshSkills();
    error.value = msg;
  } finally {
    toggling.value = '';
  }
}

const statusClass = (skill: InstalledSkill) => {
  if (skill.enabled) {
    return 'skills-status-badge active';
  }
  return 'skills-status-badge available';
};

const statusLabel = (skill: InstalledSkill) => {
  if (skill.enabled) return t('general.skillStatusActive');
  return t('general.skillStatusAvailable');
};

onMounted(refreshSkills);

defineExpose({ refreshSkills });
</script>

<template>
  <div class="space-y-4">
    <!-- Search and Actions Bar -->
    <div class="flex flex-wrap items-center gap-3">
      <div class="relative flex-1 min-w-[200px]">
        <Search :size="14" class="absolute left-3 top-2.5" style="color: var(--muted-foreground);" />
        <input
          v-model="searchQuery"
          type="text"
          class="ui-input ui-input--leading-icon skills-search-input"
          :placeholder="t('general.searchInstalled')"
        />
      </div>

      <button
        class="ui-button ui-button--ghost skills-btn"
        :disabled="loading"
        @click="refreshSkills"
      >
        <RefreshCcw :size="14" />
        {{ t('general.refreshSkills') }}
      </button>
    </div>

    <!-- Error Display -->
    <p v-if="error" class="text-xs" style="color: var(--destructive); break-words;">{{ error }}</p>

    <!-- Loading State -->
    <div v-if="loading" class="text-sm" style="color: var(--muted-foreground);">{{ t('general.loadingSkills') }}</div>

    <!-- Empty State -->
    <div v-else-if="filteredSkills.length === 0 && !error" class="text-sm" style="color: var(--muted-foreground);">
      {{ searchQuery ? t('general.noSearchResults') : t('general.emptySkills') }}
    </div>

    <!-- Skills List -->
    <div v-else class="space-y-3">
      <div
        v-for="skill in filteredSkills"
        :key="`${skill.origin}-${skill.name}`"
        class="skills-list-item"
      >
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div class="min-w-0 space-y-2">
            <div class="flex flex-wrap items-center gap-2">
              <div class="skills-item-name">{{ skill.name }}</div>
              <span
                class="skills-status-badge"
                :class="statusClass(skill)"
              >
                {{ statusLabel(skill) }}
              </span>
              <span
                class="skills-source-badge"
                :class="{ builtin: skill.origin === 'builtin' }"
              >
                {{ skill.origin === 'builtin' ? t('general.skillSourceBuiltin') : t('general.skillSourceWorkspace') }}
              </span>
            </div>
            <p class="skills-item-desc">{{ skill.description }}</p>
          </div>

          <button
            class="ui-button ui-button--ghost skills-btn"
            :disabled="Boolean(toggling) || previewMode"
            @click="onToggle(skill)"
          >
            <PowerOff v-if="skill.enabled" :size="14" />
            <Power v-else :size="14" />
            {{ skill.enabled ? t('general.disableSkill') : t('general.enableSkill') }}
          </button>
        </div>
        <div v-if="skill.warnings.length" class="mt-3 flex items-center gap-2 text-xs" style="color: var(--warning);">
          <CircleOff :size="14" />
          <span>{{ skill.warnings.join('; ') }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

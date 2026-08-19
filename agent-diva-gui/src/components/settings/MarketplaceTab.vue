<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { Store, Search, Download, CheckCircle } from '@lucide/vue';
import { useI18n } from 'vue-i18n';

import {
  searchMarketplaceSkills,
  installMarketplaceSkill,
  getSkills,
  isTauriRuntime,
  type MarketplaceSkillEntry,
} from '../../api/desktop';

const { t } = useI18n();

const previewMode = computed(() => !isTauriRuntime());

const marketplaceSkills = ref<MarketplaceSkillEntry[]>([]);
const loading = ref(false);
const error = ref('');
const searchQuery = ref('');
const hasSearched = ref(false);
const installingSkillId = ref('');
const installedSkillNames = ref<Set<string>>(new Set());

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

function formatCount(count: number): string {
  if (count >= 1000) {
    return `${(count / 1000).toFixed(1)}k`;
  }
  return String(count);
}

function slugOf(skill: MarketplaceSkillEntry): string {
  return skill.id.split('/').pop() || skill.name;
}

function isAlreadyInstalled(skill: MarketplaceSkillEntry): boolean {
  return installedSkillNames.value.has(slugOf(skill));
}

async function loadInstalledSkills() {
  if (previewMode.value) return;
  try {
    const skills = await getSkills();
    installedSkillNames.value = new Set(skills.map((s) => s.name));
  } catch {
    // Ignore errors, just won't show installed status
  }
}

async function searchMarketplace() {
  if (previewMode.value) {
    marketplaceSkills.value = [];
    return;
  }

  const query = searchQuery.value.trim();
  if (query.length < 2) {
    marketplaceSkills.value = [];
    hasSearched.value = false;
    return;
  }

  loading.value = true;
  error.value = '';
  try {
    const skills = await searchMarketplaceSkills(query, 20);
    marketplaceSkills.value = [...skills].sort((a, b) => b.installs - a.installs);
    hasSearched.value = true;
  } catch (err) {
    error.value = String(err);
    marketplaceSkills.value = [];
    hasSearched.value = true;
  } finally {
    loading.value = false;
  }
}

function triggerSearch() {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }
  debounceTimer = setTimeout(() => {
    searchMarketplace();
  }, 300);
}

async function installSkill(skill: MarketplaceSkillEntry) {
  if (previewMode.value || installingSkillId.value || isAlreadyInstalled(skill)) {
    return;
  }

  installingSkillId.value = skill.id;
  error.value = '';
  try {
    await installMarketplaceSkill(skill.id);
    await loadInstalledSkills();
  } catch (err) {
    error.value = t('general.installFailed', { error: String(err) });
  } finally {
    installingSkillId.value = '';
  }
}

onUnmounted(() => {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }
});

onMounted(() => {
  loadInstalledSkills();
});
</script>

<template>
  <div class="space-y-4">
    <!-- Search Bar -->
    <div class="flex flex-wrap items-center gap-3">
      <div class="relative flex-1 min-w-[200px]">
        <Search :size="14" class="absolute left-3 top-2.5" style="color: var(--text-muted);" />
        <input
          v-model="searchQuery"
          type="text"
          class="skills-search-input"
          :placeholder="t('general.searchMarketplace')"
          @input="triggerSearch"
          @keyup.enter="searchMarketplace"
        />
      </div>
    </div>

    <!-- Error Display -->
    <div v-if="error" class="skills-hint-box" style="border-color: var(--danger); background: var(--danger-bg);">
      <p class="text-sm" style="color: var(--danger);">{{ error }}</p>
      <button
        class="mt-2 text-xs font-medium"
        style="color: var(--danger);"
        @click="searchMarketplace"
      >
        {{ t('general.retry') }}
      </button>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="text-sm" style="color: var(--text-muted);">{{ t('general.loadingMarketplace') }}</div>

    <!-- Search Prompt -->
    <div v-else-if="!hasSearched" class="marketplace-empty">
      <Store :size="32" class="mx-auto mb-3" style="color: var(--text-muted); opacity: 0.5;" />
      <p class="text-sm" style="color: var(--text-muted);">
        {{ t('general.marketplaceSearchPrompt') }}
      </p>
    </div>

    <!-- Empty Results -->
    <div
      v-else-if="marketplaceSkills.length === 0"
      class="marketplace-empty"
    >
      <Store :size="32" class="mx-auto mb-3" style="color: var(--text-muted); opacity: 0.5;" />
      <p class="text-sm" style="color: var(--text-muted);">
        {{ t('general.noSearchResults') }}
      </p>
      <button
        class="mt-3 text-xs font-medium"
        style="color: var(--accent);"
        @click="searchMarketplace"
      >
        {{ t('general.retry') }}
      </button>
    </div>

    <!-- Skills Grid -->
    <div v-else class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
      <div
        v-for="skill in marketplaceSkills"
        :key="skill.id"
        class="marketplace-card"
      >
        <div class="space-y-2">
          <div class="flex items-start justify-between gap-2">
            <h4 class="marketplace-card-name">{{ skill.name }}</h4>
          </div>
          <p class="marketplace-card-desc">{{ skill.source }}</p>
        </div>
        <div class="mt-3 flex items-center justify-between">
          <span class="text-[11px] flex items-center gap-1" style="color: var(--text-muted); opacity: 0.7;">
            <Download :size="12" />
            {{ formatCount(skill.installs) }}
          </span>
          <button
            class="skills-btn skills-btn-primary"
            :disabled="installingSkillId === skill.id || isAlreadyInstalled(skill)"
            @click="installSkill(skill)"
          >
            <CheckCircle v-if="isAlreadyInstalled(skill)" :size="12" />
            <Download v-else-if="installingSkillId === skill.id" :size="12" class="animate-spin" />
            <Download v-else :size="12" />
            {{
              isAlreadyInstalled(skill)
                ? t('general.installed')
                : installingSkillId === skill.id
                ? t('general.installing')
                : t('general.install')
            }}
          </button>
        </div>
      </div>
    </div>

    <!-- Preview Mode Notice -->
    <div v-if="previewMode" class="skills-hint-box warning">
      {{ t('general.skillsPreviewOnly') }}
    </div>
  </div>
</template>

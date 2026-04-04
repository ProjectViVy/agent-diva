<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import {
  Store,
  Search,
  Download,
  CheckCircle,
  ShieldCheck,
  BadgeCheck,
  Users,
  Filter,
} from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

import {
  searchMarketplaceSkills,
  installSkillFromUrl,
  getSkills,
  isTauriRuntime,
  type MarketplaceSkillEntry,
} from '../../api/desktop';

const { t } = useI18n();

const previewMode = computed(() => !isTauriRuntime());

// State
const marketplaceSkills = ref<MarketplaceSkillEntry[]>([]);
const loading = ref(false);
const error = ref('');
const searchQuery = ref('');
const selectedCategory = ref('');
const selectedTrustLevel = ref('');
const sortBy = ref<'installs' | 'rating' | 'newest' | 'stars'>('installs');
const currentPage = ref(1);
const totalResults = ref(0);
const installingSkillId = ref('');
const installedSkillNames = ref<Set<string>>(new Set());

// Debounce timer
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

// Categories
const categories = computed(() => [
  { value: '', label: t('general.allCategories') },
  { value: 'development', label: t('general.categoryDevelopment') },
  { value: 'automation', label: t('general.categoryAutomation') },
  { value: 'productivity', label: t('general.categoryProductivity') },
  { value: 'data', label: t('general.categoryData') },
  { value: 'other', label: t('general.categoryOther') },
]);

// Trust levels
const trustLevels = computed(() => [
  { value: '', label: t('general.allTrustLevels') },
  { value: 'official', label: t('general.trustOfficial') },
  { value: 'certified', label: t('general.trustCertified') },
  { value: 'community', label: t('general.trustCommunity') },
]);

// Sort options
const sortOptions = computed(() => [
  { value: 'installs', label: t('general.sortInstalls') },
  { value: 'rating', label: t('general.sortRating') },
  { value: 'newest', label: t('general.sortNewest') },
  { value: 'stars', label: t('general.sortStars') },
]);

// Format count (e.g., 1200 -> 1.2k)
function formatCount(count: number): string {
  if (count >= 1000) {
    return `${(count / 1000).toFixed(1)}k`;
  }
  return String(count);
}

// Trust level class
function trustLevelClass(trustLevel: string): string {
  switch (trustLevel) {
    case 'official':
      return 'skills-source-badge official';
    case 'certified':
      return 'skills-source-badge certified';
    case 'community':
    default:
      return 'skills-source-badge';
  }
}

// Trust level icon
function trustLevelIcon(trustLevel: string) {
  switch (trustLevel) {
    case 'official':
      return ShieldCheck;
    case 'certified':
      return BadgeCheck;
    case 'community':
      return Users;
    default:
      return Users;
  }
}

// Check if skill is already installed
function isAlreadyInstalled(skillName: string): boolean {
  return installedSkillNames.value.has(skillName);
}

// Load installed skills
async function loadInstalledSkills() {
  if (previewMode.value) return;
  try {
    const skills = await getSkills();
    installedSkillNames.value = new Set(skills.map((s) => s.name));
  } catch {
    // Ignore errors, just won't show installed status
  }
}

// Search marketplace
async function searchMarketplace() {
  if (previewMode.value) {
    marketplaceSkills.value = [];
    totalResults.value = 0;
    return;
  }

  loading.value = true;
  error.value = '';
  try {
    const result = await searchMarketplaceSkills({
      query: searchQuery.value || undefined,
      category: selectedCategory.value || undefined,
      trustLevel: selectedTrustLevel.value || undefined,
      sort: sortBy.value,
      page: currentPage.value,
      limit: 20,
    });
    marketplaceSkills.value = result.skills;
    totalResults.value = result.total;
  } catch (err) {
    error.value = String(err);
    marketplaceSkills.value = [];
    totalResults.value = 0;
  } finally {
    loading.value = false;
  }
}

// Debounced search
function triggerSearch() {
  currentPage.value = 1;
  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }
  debounceTimer = setTimeout(() => {
    searchMarketplace();
  }, 300);
}

// Install skill
async function installSkill(skill: MarketplaceSkillEntry) {
  if (previewMode.value || installingSkillId.value || isAlreadyInstalled(skill.name)) {
    return;
  }

  installingSkillId.value = skill.id;
  error.value = '';
  try {
    await installSkillFromUrl(skill.installUrl);
    await loadInstalledSkills();
  } catch (err) {
    error.value = t('general.installFailed', { error: String(err) });
  } finally {
    installingSkillId.value = '';
  }
}

// Watchers for filter changes
watch([selectedCategory, selectedTrustLevel, sortBy], () => {
  currentPage.value = 1;
  searchMarketplace();
});

// Cleanup
onUnmounted(() => {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }
});

onMounted(() => {
  loadInstalledSkills();
  searchMarketplace();
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
        />
      </div>

      <select
        v-model="sortBy"
        class="skills-btn"
        style="min-width: 100px;"
      >
        <option v-for="opt in sortOptions" :key="opt.value" :value="opt.value">
          {{ opt.label }}
        </option>
      </select>
    </div>

    <!-- Filters -->
    <div class="flex flex-wrap items-center gap-2">
      <Filter :size="14" style="color: var(--text-muted);" />
      <span class="text-xs" style="color: var(--text-muted);">{{ t('general.allCategories') }}:</span>
      <button
        v-for="cat in categories"
        :key="cat.value"
        class="marketplace-filter-btn"
        :class="{ active: selectedCategory === cat.value }"
        @click="selectedCategory = cat.value"
      >
        {{ cat.label }}
      </button>

      <span class="text-xs ml-2" style="color: var(--text-muted);">{{ t('general.allTrustLevels') }}:</span>
      <button
        v-for="tl in trustLevels"
        :key="tl.value"
        class="marketplace-filter-btn"
        :class="{ active: selectedTrustLevel === tl.value }"
        @click="selectedTrustLevel = tl.value"
      >
        <component :is="trustLevelIcon(tl.value)" v-if="tl.value" :size="12" />
        {{ tl.label }}
      </button>
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

    <!-- Empty State -->
    <div
      v-else-if="marketplaceSkills.length === 0"
      class="marketplace-empty"
    >
      <Store :size="32" class="mx-auto mb-3" style="color: var(--text-muted); opacity: 0.5;" />
      <p class="text-sm" style="color: var(--text-muted);">
        {{ searchQuery ? t('general.noSearchResults') : t('general.emptyMarketplace') }}
      </p>
      <p v-if="!searchQuery && !previewMode" class="mt-2 text-xs" style="color: var(--text-muted); opacity: 0.7;">
        {{ t('general.marketplaceUnavailableDesc') }}
      </p>
      <button
        v-if="!searchQuery && !previewMode"
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
            <span
              class="skills-source-badge flex items-center gap-1 flex-shrink-0"
              :class="{ builtin: skill.trustLevel === 'official' || skill.trustLevel === 'certified' }"
            >
              <component :is="trustLevelIcon(skill.trustLevel)" :size="10" />
              {{
                skill.trustLevel === 'official'
                  ? t('general.trustOfficial')
                  : skill.trustLevel === 'certified'
                  ? t('general.trustCertified')
                  : t('general.trustCommunity')
              }}
            </span>
          </div>
          <p class="marketplace-card-desc">{{ skill.description }}</p>
          <div v-if="skill.tags && skill.tags.length > 0" class="flex flex-wrap gap-1">
            <span
              v-for="tag in skill.tags.slice(0, 3)"
              :key="tag"
              class="px-1.5 py-0.5 rounded text-[10px]"
              style="background: var(--line); color: var(--text-muted);"
            >
              {{ tag }}
            </span>
          </div>
        </div>
        <div class="mt-3 flex items-center justify-between">
          <span class="text-[11px] flex items-center gap-1" style="color: var(--text-muted); opacity: 0.7;">
            <Download :size="12" />
            {{ formatCount(skill.installCount) }}
          </span>
          <button
            class="skills-btn skills-btn-primary"
            :disabled="installingSkillId === skill.id || isAlreadyInstalled(skill.name)"
            @click="installSkill(skill)"
          >
            <CheckCircle v-if="isAlreadyInstalled(skill.name)" :size="12" />
            <Download v-else-if="installingSkillId === skill.id" :size="12" class="animate-spin" />
            <Download v-else :size="12" />
            {{
              isAlreadyInstalled(skill.name)
                ? t('general.installed')
                : installingSkillId === skill.id
                ? t('general.installing')
                : t('general.install')
            }}
          </button>
        </div>
      </div>
    </div>

    <!-- Pagination -->
    <div v-if="totalResults > 20" class="flex items-center justify-center gap-4 pt-2">
      <button
        class="pagination-btn"
        :disabled="currentPage <= 1"
        @click="currentPage--; searchMarketplace()"
      >
        {{ t('general.previousPage') }}
      </button>
      <span class="text-xs" style="color: var(--text-muted);">
        {{ t('general.pageInfo', { current: currentPage, total: Math.ceil(totalResults / 20) }) }}
      </span>
      <button
        class="pagination-btn"
        :disabled="currentPage * 20 >= totalResults"
        @click="currentPage++; searchMarketplace()"
      >
        {{ t('general.nextPage') }}
      </button>
    </div>

    <!-- Preview Mode Notice -->
    <div v-if="previewMode" class="skills-hint-box warning">
      {{ t('general.skillsPreviewOnly') }}
    </div>
  </div>
</template>

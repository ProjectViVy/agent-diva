<script setup lang="ts">
import { ref } from 'vue';
import { Boxes, Store } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

import InstalledSkillsTab from './InstalledSkillsTab.vue';
import MarketplaceTab from './MarketplaceTab.vue';

const { t } = useI18n();

type TabId = 'installed' | 'marketplace';
const activeTab = ref<TabId>('installed');

const installedRef = ref<InstanceType<typeof InstalledSkillsTab> | null>(null);

function onTabChange(tab: TabId) {
  activeTab.value = tab;
  // Refresh marketplace data when switching to marketplace tab
  if (tab === 'marketplace' && installedRef.value) {
    // Installed tab will auto-refresh on mount
  }
}
</script>

<template>
  <section class="settings-card">
    <!-- Tab Switcher -->
    <div class="skills-tab-switcher">
      <button
        class="skills-tab-btn"
        :class="{ active: activeTab === 'installed' }"
        @click="onTabChange('installed')"
      >
        <Boxes :size="14" />
        {{ t('general.skillsTabInstalled') }}
      </button>
      <button
        class="skills-tab-btn"
        :class="{ active: activeTab === 'marketplace' }"
        @click="onTabChange('marketplace')"
      >
        <Store :size="14" />
        {{ t('general.skillsTabMarketplace') }}
      </button>
    </div>

    <!-- Tab Content -->
    <InstalledSkillsTab v-if="activeTab === 'installed'" ref="installedRef" />
    <MarketplaceTab v-else-if="activeTab === 'marketplace'" />
  </section>
</template>

<script setup lang="ts">
import { Package } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

import SkillManagementCard from './SkillManagementCard.vue';

const { t } = useI18n();
</script>

<template>
  <div class="p-6 space-y-6 fade-in">
    <div class="flex items-center space-x-3">
      <div class="settings-dashboard-icon">
        <Package :size="20" />
      </div>
      <div>
        <h3 class="settings-dashboard-title">{{ t('settings.skills') }}</h3>
        <p class="settings-dashboard-desc">{{ t('dashboard.skillsDesc') }}</p>
      </div>
    </div>

    <SkillManagementCard />
  </div>
</template>

<style scoped>
.fade-in {
  animation: slideIn 0.3s ease-out;
}

@keyframes slideIn {
  from { opacity: 0; transform: translateX(20px); }
  to { opacity: 1; transform: translateX(0); }
}
</style>

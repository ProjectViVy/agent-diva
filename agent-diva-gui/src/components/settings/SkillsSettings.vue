<script setup lang="ts">
import { WandSparkles } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

import SkillManagementCard from './SkillManagementCard.vue';
import CapabilityManifestPanel from './CapabilityManifestPanel.vue';

const { t } = useI18n();
</script>

<template>
  <div class="p-6 space-y-6 fade-in">
    <div class="flex items-center space-x-3">
      <div class="w-10 h-10 rounded-lg bg-cyan-100 text-cyan-600 flex items-center justify-center">
        <WandSparkles :size="20" />
      </div>
      <div>
        <h3 class="text-lg font-bold text-gray-800">{{ t('settings.skills') }}</h3>
        <p class="text-sm text-gray-500">{{ t('dashboard.skillsDesc') }}</p>
      </div>
    </div>

    <SkillManagementCard />

    <CapabilityManifestPanel />
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

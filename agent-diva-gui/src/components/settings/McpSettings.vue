<script setup lang="ts">
import { Bot } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

import McpManagementCard from './McpManagementCard.vue';

const { t } = useI18n();
</script>

<template>
  <div class="p-6 space-y-6 fade-in">
    <div class="flex items-center space-x-3">
      <div class="w-10 h-10 rounded-lg bg-amber-100 text-amber-700 flex items-center justify-center">
        <Bot :size="20" />
      </div>
      <div>
        <h3 class="text-lg font-bold text-gray-800">{{ t('settings.mcp') }}</h3>
        <p class="text-sm text-gray-500">{{ t('dashboard.mcpDesc') }}</p>
      </div>
    </div>

    <McpManagementCard />
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

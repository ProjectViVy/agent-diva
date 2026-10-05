<script setup lang="ts">
import { Heart } from '@lucide/vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
</script>

<template>
  <div class="p-8 fade-in max-w-2xl mx-auto text-center">
    <div class="mb-8">
      <div class="w-24 h-24 bg-card   rounded-3xl flex items-center justify-center text-foreground shadow-xl  mx-auto mb-6 transform hover:rotate-6 transition-transform">
        <Heart :size="48" fill="currentColor" />
      </div>
      <h2 class="text-3xl font-bold settings-label mb-2">Agent Diva</h2>
      <p class="settings-muted max-w-md mx-auto">{{ t('about.desc') }}</p>
    </div>

    <div class="settings-section text-left">
      <div class="p-4 border-b flex justify-between items-center hover:bg-[var(--accent)] transition-colors" style="border-color: var(--border);">
        <span class="settings-muted">{{ t('about.version') }}</span>
        <span class="font-mono settings-label font-medium">0.4.0</span>
      </div>
      <div class="p-4 border-b flex justify-between items-center hover:bg-[var(--accent)] transition-colors" style="border-color: var(--border);">
        <span class="settings-muted">{{ t('about.license') }}</span>
        <span class="font-medium settings-label">MIT</span>
      </div>
      <div class="p-4 flex justify-between items-center hover:bg-[var(--accent)] transition-colors">
        <span class="settings-muted">{{ t('about.author') }}</span>
        <span class="font-medium settings-label">mastwet / projectViVY / undefine foundation</span>
      </div>
    </div>

    <div class="mt-8 text-xs settings-muted">
      Made with love by mastwet / projectViVY / undefine foundation
    </div>
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

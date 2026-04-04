<script setup lang="ts">
import { computed } from 'vue';
import { Bot, Server, MessageSquare, Globe, Info, Search, SlidersHorizontal, WandSparkles, Palette } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const emit = defineEmits<{
  (e: 'navigate', view: 'general' | 'mcp' | 'skills' | 'providers' | 'channels' | 'network' | 'language' | 'theme' | 'about'): void;
}>();

const cards = computed(() => [
  { id: 'general', icon: SlidersHorizontal, title: t('dashboard.general'), desc: t('dashboard.generalDesc') },
  { id: 'mcp', icon: Bot, title: t('dashboard.mcp'), desc: t('dashboard.mcpDesc') },
  { id: 'skills', icon: WandSparkles, title: t('dashboard.skills'), desc: t('dashboard.skillsDesc') },
  { id: 'providers', icon: Server, title: t('dashboard.providers'), desc: t('dashboard.providersDesc') },
  { id: 'channels', icon: MessageSquare, title: t('dashboard.channels'), desc: t('dashboard.channelsDesc') },
  { id: 'network', icon: Search, title: t('dashboard.network'), desc: t('dashboard.networkDesc') },
  { id: 'language', icon: Globe, title: t('dashboard.language'), desc: t('dashboard.languageDesc') },
  { id: 'about', icon: Info, title: t('dashboard.about'), desc: t('dashboard.aboutDesc') },
  { id: 'theme', icon: Palette, title: t('dashboard.theme'), desc: t('dashboard.themeDesc') },
]);
</script>

<template>
  <div class="grid grid-cols-1 md:grid-cols-2 gap-6 p-6 fade-in">
    <button
      v-for="card in cards"
      :key="card.id"
      @click="emit('navigate', card.id as any)"
      class="settings-dashboard-card group flex flex-col items-start text-left h-full"
    >
      <div class="settings-dashboard-icon">
        <component :is="card.icon" :size="24" />
      </div>
      <h3 class="settings-dashboard-title">{{ card.title }}</h3>
      <p class="settings-dashboard-desc">{{ card.desc }}</p>
    </button>
  </div>
</template>

<style scoped>
.fade-in {
  animation: fadeIn 0.3s ease-out;
}

@keyframes fadeIn {
  from { opacity: 0; transform: scale(0.95); }
  to { opacity: 1; transform: scale(1); }
}
</style>

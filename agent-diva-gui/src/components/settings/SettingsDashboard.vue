<script setup lang="ts">
import { computed } from 'vue';
import { Server, MessageSquare, Globe, Info } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const emit = defineEmits<{
  (e: 'navigate', view: 'providers' | 'channels' | 'language' | 'about'): void;
}>();

const cards = computed(() => [
  { id: 'providers', icon: Server, title: t('dashboard.providers'), desc: t('dashboard.providersDesc'), color: 'text-purple-600', bg: 'bg-purple-100' },
  { id: 'channels', icon: MessageSquare, title: t('dashboard.channels'), desc: t('dashboard.channelsDesc'), color: 'text-pink-600', bg: 'bg-pink-100' },
  { id: 'language', icon: Globe, title: t('dashboard.language'), desc: t('dashboard.languageDesc'), color: 'text-blue-600', bg: 'bg-blue-100' },
  { id: 'about', icon: Info, title: t('dashboard.about'), desc: t('dashboard.aboutDesc'), color: 'text-green-600', bg: 'bg-green-100' },
]);
</script>

<template>
  <div class="grid grid-cols-1 md:grid-cols-2 gap-6 p-6 fade-in">
    <button
      v-for="card in cards"
      :key="card.id"
      @click="emit('navigate', card.id as any)"
      class="flex flex-col items-start p-6 bg-white border border-gray-100 rounded-xl shadow-sm hover:shadow-md hover:border-pink-200 transition-all text-left group h-full"
    >
      <div class="w-12 h-12 rounded-lg flex items-center justify-center mb-4 transition-transform group-hover:scale-110" :class="[card.bg, card.color]">
        <component :is="card.icon" :size="24" />
      </div>
      <h3 class="text-lg font-bold text-gray-800 mb-2">{{ card.title }}</h3>
      <p class="text-sm text-gray-500">{{ card.desc }}</p>
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

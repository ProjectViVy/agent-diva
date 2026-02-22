<script setup lang="ts">
import { ref, computed } from 'vue';
import { ChevronLeft } from 'lucide-vue-next';
import SettingsDashboard from './settings/SettingsDashboard.vue';
import ProvidersSettings from './settings/ProvidersSettings.vue';
import ChannelsSettings from './settings/ChannelsSettings.vue';
import LanguageSettings from './settings/LanguageSettings.vue';
import AboutSettings from './settings/AboutSettings.vue';

interface SavedModel {
  id: string;
  provider: string;
  model: string;
  apiBase: string;
  apiKey: string;
  displayName: string;
}

const props = defineProps<{
  config: {
    apiBase: string;
    apiKey: string;
    model: string;
  };
  savedModels?: SavedModel[];
}>();

const emit = defineEmits<{
  (e: 'save', config: typeof props.config): void;
  (e: 'update-saved-models', models: SavedModel[]): void;
}>();

const currentView = ref<'dashboard' | 'providers' | 'channels' | 'language' | 'about'>('dashboard');
const lang = ref<'zh' | 'en'>('zh');

const t = computed(() => {
  return lang.value === 'zh' ? {
    settings: '设置',
    back: '返回',
    providers: '供应商',
    channels: '频道',
    language: '语言',
    about: '关于',
  } : {
    settings: 'Settings',
    back: 'Back',
    providers: 'Providers',
    channels: 'Channels',
    language: 'Language',
    about: 'About',
  };
});

const pageTitle = computed(() => {
  if (currentView.value === 'dashboard') return t.value.settings;
  const titles = {
    providers: t.value.providers,
    channels: t.value.channels,
    language: t.value.language,
    about: t.value.about
  };
  return titles[currentView.value] || t.value.settings;
});

const toggleLang = () => {
  lang.value = lang.value === 'zh' ? 'en' : 'zh';
};

const handleNavigate = (view: 'providers' | 'channels' | 'language' | 'about') => {
  currentView.value = view;
};

const goBack = () => {
  currentView.value = 'dashboard';
};
</script>

<template>
  <div class="h-full flex flex-col bg-white rounded-xl overflow-hidden min-w-[320px]">
    <!-- Top Bar -->
    <div class="p-6 border-b border-gray-100 flex justify-between items-center h-20">
      <div class="flex items-center space-x-2">
        <button 
          v-if="currentView !== 'dashboard'"
          @click="goBack"
          class="p-2 hover:bg-gray-100 rounded-lg text-gray-500 transition-colors"
        >
          <ChevronLeft :size="24" />
        </button>
        <h2 class="text-xl font-bold text-gray-800 flex items-center animate-in fade-in slide-in-from-left-2 duration-200" :key="pageTitle">
          <span v-if="currentView === 'dashboard'" class="mr-2">⚙️</span>
          {{ pageTitle }}
        </h2>
      </div>
    </div>
    
    <!-- Content Area -->
    <div class="flex-1 overflow-hidden relative bg-gray-50/30">
       <Transition name="page" mode="out-in">
          <div :key="currentView" class="h-full w-full">
            <SettingsDashboard 
              v-if="currentView === 'dashboard'"
              :lang="lang"
              @navigate="handleNavigate"
            />
            
            <ProvidersSettings 
              v-else-if="currentView === 'providers'"
              :config="config"
              :saved-models="savedModels"
              :lang="lang"
              @save="(c) => emit('save', c)"
              @update-saved-models="(m) => emit('update-saved-models', m)"
            />
            
            <ChannelsSettings 
              v-else-if="currentView === 'channels'"
              :lang="lang"
            />
            
            <LanguageSettings 
              v-else-if="currentView === 'language'"
              :lang="lang"
              @toggle-lang="toggleLang"
            />
            
            <AboutSettings 
              v-else-if="currentView === 'about'"
              :lang="lang"
            />
          </div>
       </Transition>
    </div>
  </div>
</template>

<style scoped>
.page-enter-active,
.page-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.page-enter-from {
  opacity: 0;
  transform: translateY(10px);
}

.page-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}
</style>

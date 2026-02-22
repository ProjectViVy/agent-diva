<script setup lang="ts">
import { computed } from 'vue';
import { Globe } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t, locale } = useI18n();

const toggleLang = () => {
  locale.value = locale.value === 'zh' ? 'en' : 'zh';
  localStorage.setItem('agent-diva-locale', locale.value);
};
</script>

<template>
  <div class="p-8 fade-in max-w-2xl mx-auto">
    <div class="text-center mb-8">
      <div class="w-16 h-16 bg-blue-100 text-blue-600 rounded-2xl flex items-center justify-center mx-auto mb-4">
        <Globe :size="32" />
      </div>
      <h2 class="text-2xl font-bold text-gray-800">{{ t('language.title') }}</h2>
      <p class="text-gray-500 mt-2">{{ t('language.desc') }}</p>
    </div>

    <div class="bg-gray-50 rounded-xl p-6 border border-gray-100">
      <div class="flex items-center justify-between p-4 bg-white rounded-lg border border-gray-200 shadow-sm">
        <div class="flex items-center space-x-4">
           <div class="w-10 h-10 rounded-full bg-gray-100 flex items-center justify-center text-lg">
             {{ locale === 'zh' ? '🇨🇳' : '🇺🇸' }}
           </div>
           <div>
             <div class="font-medium text-gray-800">{{ t('language.current') }}</div>
             <div class="text-sm text-gray-500">{{ locale === 'zh' ? t('language.chinese') : t('language.english') }}</div>
           </div>
        </div>
        
        <button 
          @click="toggleLang"
          class="px-4 py-2 bg-white border border-gray-300 hover:border-blue-500 hover:text-blue-600 text-gray-700 rounded-lg transition-all text-sm font-medium"
        >
          {{ t('language.switch') }}
        </button>
      </div>
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

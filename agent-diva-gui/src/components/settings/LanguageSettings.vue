<script setup lang="ts">
import { computed } from 'vue';
import { Globe } from 'lucide-vue-next';

const props = defineProps<{
  lang: 'zh' | 'en';
}>();

const emit = defineEmits<{
  (e: 'toggle-lang'): void;
}>();

const t = computed(() => {
  return props.lang === 'zh' ? {
    language: '语言设置',
    currentLang: '当前语言',
    switchLang: '切换语言',
    chinese: '中文 (Chinese)',
    english: '英文 (English)',
    desc: '选择界面显示的语言。目前支持中文和英文。'
  } : {
    language: 'Language Settings',
    currentLang: 'Current Language',
    switchLang: 'Switch Language',
    chinese: 'Chinese (中文)',
    english: 'English (英文)',
    desc: 'Select the language for the interface. Currently supports Chinese and English.'
  };
});
</script>

<template>
  <div class="p-8 fade-in max-w-2xl mx-auto">
    <div class="text-center mb-8">
      <div class="w-16 h-16 bg-blue-100 text-blue-600 rounded-2xl flex items-center justify-center mx-auto mb-4">
        <Globe :size="32" />
      </div>
      <h2 class="text-2xl font-bold text-gray-800">{{ t.language }}</h2>
      <p class="text-gray-500 mt-2">{{ t.desc }}</p>
    </div>

    <div class="bg-gray-50 rounded-xl p-6 border border-gray-100">
      <div class="flex items-center justify-between p-4 bg-white rounded-lg border border-gray-200 shadow-sm">
        <div class="flex items-center space-x-4">
           <div class="w-10 h-10 rounded-full bg-gray-100 flex items-center justify-center text-lg">
             {{ props.lang === 'zh' ? '🇨🇳' : '🇺🇸' }}
           </div>
           <div>
             <div class="font-medium text-gray-800">{{ t.currentLang }}</div>
             <div class="text-sm text-gray-500">{{ props.lang === 'zh' ? t.chinese : t.english }}</div>
           </div>
        </div>
        
        <button 
          @click="emit('toggle-lang')"
          class="px-4 py-2 bg-white border border-gray-300 hover:border-blue-500 hover:text-blue-600 text-gray-700 rounded-lg transition-all text-sm font-medium"
        >
          {{ t.switchLang }}
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

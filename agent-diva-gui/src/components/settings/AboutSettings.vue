<script setup lang="ts">
import { computed } from 'vue';
import { Info, Heart } from 'lucide-vue-next';

const props = defineProps<{
  lang: 'zh' | 'en';
}>();

const t = computed(() => {
  return props.lang === 'zh' ? {
    about: '关于 Agent Diva',
    desc: 'Agent Diva 是一个跨平台的 AI 代理框架，旨在提供可爱的、人性化的 AI 伴侣体验。',
    version: '版本',
    author: '作者',
    license: '许可证',
    website: '项目地址',
  } : {
    about: 'About Agent Diva',
    desc: 'Agent Diva is a cross-platform AI agent framework designed to provide a cute, human-like AI companion experience.',
    version: 'Version',
    author: 'Author',
    license: 'License',
    website: 'Website',
  };
});
</script>

<template>
  <div class="p-8 fade-in max-w-2xl mx-auto text-center">
    <div class="mb-8">
      <div class="w-24 h-24 bg-gradient-to-br from-pink-400 to-purple-500 rounded-3xl flex items-center justify-center text-white shadow-xl shadow-pink-500/20 mx-auto mb-6 transform hover:rotate-6 transition-transform">
        <Heart :size="48" fill="currentColor" />
      </div>
      <h2 class="text-3xl font-bold text-gray-800 mb-2">Agent Diva</h2>
      <p class="text-gray-500 max-w-md mx-auto">{{ t.desc }}</p>
    </div>

    <div class="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden text-left">
      <div class="p-4 border-b border-gray-50 flex justify-between items-center hover:bg-gray-50 transition-colors">
        <span class="text-gray-500">{{ t.version }}</span>
        <span class="font-mono text-gray-800 font-medium">0.1.0</span>
      </div>
      <div class="p-4 border-b border-gray-50 flex justify-between items-center hover:bg-gray-50 transition-colors">
        <span class="text-gray-500">{{ t.license }}</span>
        <span class="font-medium text-gray-800">MIT</span>
      </div>
      <div class="p-4 flex justify-between items-center hover:bg-gray-50 transition-colors">
        <span class="text-gray-500">{{ t.author }}</span>
        <span class="font-medium text-gray-800">VivyCore</span>
      </div>
    </div>
    
    <div class="mt-8 text-xs text-gray-400">
      Made with ❤️ by VivyCore
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

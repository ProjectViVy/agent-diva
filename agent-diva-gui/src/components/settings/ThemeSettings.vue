<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { Palette, Heart, Moon, Sun } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const props = defineProps<{
  currentTheme: string;
}>();

const emit = defineEmits<{
  (e: 'change-theme', theme: string): void;
}>();

// 可用主题列表
const themes = [
  {
    id: 'love',
    name: 'theme.love',
    desc: 'theme.loveDesc',
    icon: Heart,
    preview: 'linear-gradient(135deg, #fff5f7 0%, #ffe4ef 50%, #ffd6e8 100%)',
    accent: '#ec4899',
    bgLight: 'rgba(236, 72, 153, 0.1)',
    borderLight: 'rgba(236, 72, 153, 0.3)',
  },
  {
    id: 'dark',
    name: 'theme.dark',
    desc: 'theme.darkDesc',
    icon: Moon,
    preview: 'linear-gradient(135deg, #0f172a 0%, #1e293b 50%, #334155 100%)',
    accent: '#60a5fa',
    bgLight: 'rgba(96, 165, 250, 0.1)',
    borderLight: 'rgba(96, 165, 250, 0.3)',
  },
  {
    id: 'default',
    name: 'theme.default',
    desc: 'theme.defaultDesc',
    icon: Sun,
    preview: 'linear-gradient(135deg, #ffffff 0%, #fff5f7 40%, #ffe4ef 100%)',
    accent: '#ec4899',
    bgLight: 'rgba(236, 72, 153, 0.1)',
    borderLight: 'rgba(236, 72, 153, 0.3)',
  },
];

const localTheme = ref(props.currentTheme);

watch(
  () => props.currentTheme,
  (val) => {
    localTheme.value = val;
  }
);

const selectTheme = (themeId: string) => {
  localTheme.value = themeId;
  emit('change-theme', themeId);
};

// 获取当前选中主题的强调色
const currentAccent = computed(() => {
  const theme = themes.find(t => t.id === localTheme.value);
  return theme?.accent || '#6b7280';
});
</script>

<template>
  <div class="p-6 space-y-6 fade-in">
    <!-- 标题 -->
    <div class="flex items-center space-x-3">
      <div
        class="w-10 h-10 rounded-lg flex items-center justify-center"
        :style="{
          backgroundColor: currentAccent + '20',
          color: currentAccent
        }"
      >
        <Palette :size="20" />
      </div>
      <div>
        <h3 class="text-lg font-bold" style="color: var(--text);">{{ t('theme.title') }}</h3>
        <p class="text-sm" style="color: var(--text-muted);">{{ t('theme.desc') }}</p>
      </div>
    </div>

    <!-- 主题选择卡片 -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <button
        v-for="theme in themes"
        :key="theme.id"
        @click="selectTheme(theme.id)"
        class="theme-card group relative overflow-hidden rounded-2xl border-2 transition-all duration-300"
        :style="{
          borderColor: localTheme === theme.id ? theme.accent : '#e5e7eb',
          boxShadow: localTheme === theme.id ? `0 8px 24px ${theme.accent}30` : undefined,
          transform: localTheme === theme.id ? 'scale(1.02)' : undefined,
        }"
      >
        <!-- 预览背景 -->
        <div
          class="h-28 w-full transition-transform duration-300 group-hover:scale-105"
          :style="{ background: theme.preview }"
        >
          <!-- 预览内容 -->
          <div class="h-full p-4 flex flex-col">
            <div class="flex items-center gap-2 mb-2">
              <div class="w-6 h-6 rounded-full bg-white/80 border border-gray-200/50 flex items-center justify-center text-xs">
                💕
              </div>
              <div class="h-2 w-16 rounded-full bg-white/60"></div>
            </div>
            <div class="space-y-1.5 mt-auto">
              <div class="h-2 w-full rounded-full bg-white/50"></div>
              <div class="h-2 w-3/4 rounded-full bg-white/40"></div>
            </div>
          </div>
        </div>

        <!-- 主题信息 -->
        <div class="p-4" style="background: var(--panel-solid);">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <component
                :is="theme.icon"
                :size="18"
                :style="{ color: theme.accent }"
                class="transition-transform duration-300 group-hover:scale-110"
              />
              <span class="font-semibold" style="color: var(--text);">{{ t(theme.name) }}</span>
            </div>

            <!-- 选中指示器 -->
            <div
              v-if="localTheme === theme.id"
              class="w-5 h-5 rounded-full flex items-center justify-center"
              :style="{ backgroundColor: theme.accent }"
            >
              <svg class="w-3 h-3 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
                <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
              </svg>
            </div>
          </div>
          <p class="text-xs mt-1" style="color: var(--text-muted);">{{ t(theme.desc) }}</p>
        </div>

        <!-- 浮动效果提示（仅 love 主题） -->
        <div
          v-if="theme.id === 'love'"
          class="absolute top-2 right-2 px-2 py-0.5 rounded-full text-[10px] font-medium"
          :style="{
            backgroundColor: theme.accent + '20',
            color: theme.accent
          }"
        >
          {{ t('theme.floatingEffect') }}
        </div>
      </button>
    </div>

    <!-- 提示信息 -->
    <div class="bg-amber-50 border border-amber-200 rounded-xl p-4">
      <div class="flex items-start gap-3">
        <div class="w-5 h-5 rounded-full bg-amber-100 text-amber-600 flex items-center justify-center flex-shrink-0 mt-0.5">
          <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path stroke-linecap="round" stroke-linejoin="round" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        </div>
        <div>
          <p class="text-sm font-medium text-amber-800">{{ t('theme.tip') }}</p>
          <p class="text-xs text-amber-700 mt-1">{{ t('theme.tipDesc') }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.fade-in {
  animation: slideIn 0.3s ease-out;
}

@keyframes slideIn {
  from {
    opacity: 0;
    transform: translateX(20px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

.theme-card {
  cursor: pointer;
}
</style>

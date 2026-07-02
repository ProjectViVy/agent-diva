<script setup lang="ts">
import { computed } from 'vue';
import { Globe } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { LOCALE_STORAGE_KEY } from '../../utils/localStorageAgentDiva';

const { t, locale } = useI18n();
const currentLocale = computed(() => locale.value);

const toggleLang = () => {
  locale.value = locale.value === 'zh' ? 'en' : 'zh';
  localStorage.setItem(LOCALE_STORAGE_KEY, locale.value);
};
</script>

<template>
  <div class="p-8 fade-in max-w-2xl mx-auto">
    <div class="text-center mb-8">
      <div class="settings-dashboard-icon !w-16 !h-16 mx-auto mb-4">
        <Globe :size="32" />
      </div>
      <h2 class="text-2xl font-bold settings-label mb-2">{{ t('language.title') }}</h2>
      <p class="settings-muted mt-2">{{ t('language.desc') }}</p>
    </div>

    <div class="settings-section">
      <div class="flex items-center justify-between p-4 settings-card shadow-sm">
        <div class="flex items-center space-x-4">
          <div class="w-10 h-10 rounded-full flex items-center justify-center text-lg" style="background: var(--accent-bg-light); color: var(--accent);">
            {{ currentLocale === 'zh' ? 'CN' : 'EN' }}
          </div>
          <div>
            <div class="font-medium settings-label">{{ t('language.current') }}</div>
            <div class="text-sm settings-muted">
              {{ currentLocale === 'zh' ? t('language.chinese') : t('language.english') }}
            </div>
          </div>
        </div>

        <button
          @click="toggleLang"
          class="settings-btn settings-btn-secondary"
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

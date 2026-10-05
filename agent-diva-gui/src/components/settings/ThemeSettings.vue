<script setup lang="ts">
import { ref, watch } from 'vue';
import { Palette, Heart, Moon, Sun, Music, Check, Info } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import { THEME_IDS } from '../../composables/useTheme';

const { t } = useI18n();
const props = defineProps<{ currentTheme: string }>();
const emit = defineEmits<{ (e: 'change-theme', theme: string): void }>();
const icons = { love: Heart, dark: Moon, default: Sun, miku: Music };
const themes = THEME_IDS.map(id => ({ id, icon: icons[id], name: 'theme.' + id, desc: 'theme.' + id + 'Desc' }));
const localTheme = ref(props.currentTheme);
watch(() => props.currentTheme, value => { localTheme.value = value; });
const selectTheme = (id: string) => { localTheme.value = id; emit('change-theme', id); };
</script>

<template>
  <div class="p-6 space-y-6 fade-in">
    <div class="flex items-center space-x-3">
      <div class="w-10 h-10 rounded-lg flex items-center justify-center bg-accent text-primary"><Palette :size="20" /></div>
      <div>
        <h3 class="text-lg font-semibold text-foreground">{{ t('theme.title') }}</h3>
        <p class="text-sm text-muted-foreground">{{ t('theme.desc') }}</p>
      </div>
    </div>
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <button v-for="theme in themes" :key="theme.id" type="button"
        class="theme-card group relative overflow-hidden rounded-xl border-2 transition-colors"
        :class="{ 'theme-card--selected': localTheme === theme.id }"
        :aria-pressed="localTheme === theme.id" @click="selectTheme(theme.id)">
        <div class="theme-preview h-28 w-full bg-background" :data-theme="theme.id">
          <div class="h-full p-4 flex flex-col">
            <div class="flex items-center gap-2 mb-2">
              <div class="w-6 h-6 rounded-full bg-card border border-border flex items-center justify-center text-primary"><Heart :size="12" /></div>
              <div class="h-2 w-16 rounded-full bg-primary"></div>
            </div>
            <div class="space-y-1.5 mt-auto">
              <div class="h-2 w-full rounded-full bg-secondary"></div>
              <div class="h-2 w-3/4 rounded-full bg-accent"></div>
            </div>
          </div>
        </div>
        <div class="p-4 bg-card">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2"><component :is="theme.icon" :size="18" class="text-primary" /><span class="font-semibold text-foreground">{{ t(theme.name) }}</span></div>
            <div v-if="localTheme === theme.id" class="w-5 h-5 rounded-full flex items-center justify-center bg-primary text-primary-foreground"><Check :size="12" /></div>
          </div>
          <p class="text-xs mt-1 text-muted-foreground">{{ t(theme.desc) }}</p>
        </div>
      </button>
    </div>
    <div class="bg-muted border border-border rounded-xl p-4">
      <div class="flex items-start gap-3">
        <Info :size="20" class="text-muted-foreground flex-shrink-0 mt-0.5" />
        <div><p class="text-sm font-medium text-foreground">{{ t('theme.tip') }}</p><p class="text-xs text-muted-foreground mt-1">{{ t('theme.tipDesc') }}</p></div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.theme-card { cursor: pointer; border-color: var(--border); background: var(--card); text-align: left; }
.theme-card:hover { border-color: var(--border-strong); }
.theme-card--selected { border-color: var(--primary); }
.fade-in { animation: slideIn .2s ease-out; }
@keyframes slideIn { from { opacity: 0; transform: translateX(20px); } to { opacity: 1; transform: translateX(0); } }
@media (prefers-reduced-motion: reduce) { .fade-in { animation: none; } }
</style>

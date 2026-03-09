<script setup lang="ts">
import { ref, watch } from 'vue';
import { SlidersHorizontal, MessageSquareText, ServerCog } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import GatewayControlPanel from '../GatewayControlPanel.vue';

const { t } = useI18n();

interface ChatDisplayPrefs {
  autoExpandReasoning: boolean;
  autoExpandToolDetails: boolean;
  showRawMetaByDefault: boolean;
}

const props = defineProps<{
  chatDisplayPrefs: ChatDisplayPrefs;
}>();

const emit = defineEmits<{
  (e: 'save-chat-display-prefs', prefs: ChatDisplayPrefs): void;
}>();

const localPrefs = ref<ChatDisplayPrefs>({ ...props.chatDisplayPrefs });

watch(
  () => props.chatDisplayPrefs,
  (val) => {
    localPrefs.value = { ...val };
  },
  { deep: true }
);

const emitPrefs = () => {
  emit('save-chat-display-prefs', { ...localPrefs.value });
};
</script>

<template>
  <div class="p-6 space-y-6 fade-in">
    <div class="flex items-center space-x-3">
      <div class="w-10 h-10 rounded-lg bg-violet-100 text-violet-600 flex items-center justify-center">
        <SlidersHorizontal :size="20" />
      </div>
      <div>
        <h3 class="text-lg font-bold text-gray-800">{{ t('general.title') }}</h3>
        <p class="text-sm text-gray-500">{{ t('general.desc') }}</p>
      </div>
    </div>

    <div class="bg-white border border-gray-100 rounded-xl p-4 space-y-4">
      <div class="flex items-center space-x-2 text-gray-700">
        <MessageSquareText :size="16" class="text-violet-500" />
        <span class="text-sm font-semibold">{{ t('general.chatSettings') }}</span>
      </div>

      <div class="space-y-3 pl-1">
        <label class="text-sm text-gray-700 flex items-center space-x-2">
          <input type="checkbox" v-model="localPrefs.autoExpandReasoning" @change="emitPrefs" />
          <span>{{ t('general.autoExpandReasoning') }}</span>
        </label>
        <label class="text-sm text-gray-700 flex items-center space-x-2">
          <input type="checkbox" v-model="localPrefs.autoExpandToolDetails" @change="emitPrefs" />
          <span>{{ t('general.autoExpandToolDetails') }}</span>
        </label>
        <label class="text-sm text-gray-700 flex items-center space-x-2">
          <input type="checkbox" v-model="localPrefs.showRawMetaByDefault" @change="emitPrefs" />
          <span>{{ t('general.autoExpandRawMeta') }}</span>
        </label>
      </div>
    </div>

    <div class="space-y-4">
      <div class="flex items-center gap-2 px-1">
        <ServerCog :size="16" class="text-violet-500" />
        <div>
          <h4 class="text-sm font-semibold text-gray-800">{{ t('general.serviceTitle') }}</h4>
          <p class="text-xs text-gray-500">{{ t('console.gatewayDesc') }}</p>
        </div>
      </div>

      <GatewayControlPanel />
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

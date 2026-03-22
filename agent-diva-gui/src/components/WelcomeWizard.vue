<script setup lang="ts">
import { ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { openExternalUrl } from '../utils/openExternal';

const { t } = useI18n();

const DEEPSEEK_PLATFORM_URL = 'https://platform.deepseek.com/';
const BOCHA_OPEN_URL = 'https://open.bocha.cn/';
const BOCHA_GUIDE_FEISHU =
  'https://aq6ky2b8nql.feishu.cn/wiki/HmtOw1z6vik14Fkdu5uc9VaInBb';

type WelcomeNavigateTarget = 'chat' | 'providers' | 'network' | 'console';

interface WelcomeDonePayload {
  skipped: boolean;
  deepseekApiKey: string;
  bochaApiKey: string;
  navigate: WelcomeNavigateTarget;
}

const props = defineProps<{
  open: boolean;
  config: {
    provider: string;
    apiBase: string;
    apiKey: string;
    model: string;
  };
  toolsConfig: {
    web: {
      search: {
        provider: string;
        enabled: boolean;
        api_key: string;
        max_results: number;
      };
      fetch: {
        enabled: boolean;
      };
    };
  };
}>();

const emit = defineEmits<{
  (e: 'done', payload: WelcomeDonePayload): void;
}>();

const step = ref(0);
const deepseekKey = ref('');
const bochaKey = ref('');

const resetFromProps = () => {
  deepseekKey.value = props.config.apiKey || '';
  bochaKey.value = props.toolsConfig.web.search.api_key || '';
};

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      step.value = 0;
      resetFromProps();
    }
  }
);

const finish = (payload: Omit<WelcomeDonePayload, 'skipped'> & { skipped?: boolean }) => {
  emit('done', {
    skipped: payload.skipped ?? false,
    deepseekApiKey: payload.deepseekApiKey,
    bochaApiKey: payload.bochaApiKey,
    navigate: payload.navigate,
  });
};

const skipAll = () => {
  finish({
    skipped: true,
    deepseekApiKey: '',
    bochaApiKey: '',
    navigate: 'chat',
  });
};

const openDeepseekSite = () => {
  void openExternalUrl(DEEPSEEK_PLATFORM_URL);
};

const openBochaSite = () => {
  void openExternalUrl(BOCHA_OPEN_URL);
};

const openBochaDoc = () => {
  void openExternalUrl(BOCHA_GUIDE_FEISHU);
};
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="app-modal-dimmer fixed inset-0 z-[200] flex items-center justify-center bg-black/45 p-4 backdrop-blur-[2px]"
      role="dialog"
      aria-modal="true"
      :aria-label="t('welcome.title')"
    >
      <div
        class="app-modal-panel flex max-h-[90vh] w-full max-w-lg flex-col overflow-hidden rounded-2xl border border-gray-100 bg-white shadow-2xl"
      >
        <div class="px-6 pt-5 pb-3 border-b border-gray-100 shrink-0 bg-white">
          <h2 class="text-lg font-bold text-gray-900">{{ t('welcome.title') }}</h2>
          <p class="text-sm text-gray-500 mt-1">{{ t('welcome.subtitle') }}</p>
          <div class="flex flex-wrap gap-2 mt-3 text-xs">
            <span
              v-for="(label, i) in [
                t('welcome.stepIntro'),
                t('welcome.stepDeepseek'),
                t('welcome.stepBocha'),
                t('welcome.stepDone'),
              ]"
              :key="i"
              class="px-2 py-0.5 rounded-full"
              :class="
                step === i
                  ? 'bg-pink-100 text-pink-700 font-medium'
                  : 'bg-gray-100 text-gray-500'
              "
            >
              {{ i + 1 }}. {{ label }}
            </span>
          </div>
        </div>

        <div
          class="flex-1 min-h-0 overflow-y-auto px-6 py-4 text-sm text-gray-700 space-y-4 bg-white"
        >
          <template v-if="step === 0">
            <p class="leading-relaxed">{{ t('welcome.introBody') }}</p>
          </template>

          <template v-else-if="step === 1">
            <p class="leading-relaxed">{{ t('welcome.deepseekBody') }}</p>
            <div class="flex flex-wrap gap-2">
              <button
                type="button"
                class="px-3 py-1.5 rounded-lg bg-pink-500 text-white text-xs font-medium hover:bg-pink-600"
                @click="openDeepseekSite"
              >
                {{ t('welcome.openInBrowser') }}
              </button>
            </div>
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{
                t('welcome.deepseekApiKey')
              }}</label>
              <input
                v-model="deepseekKey"
                type="password"
                autocomplete="off"
                class="w-full px-3 py-2 border border-gray-200 rounded-lg font-mono text-xs"
                :placeholder="t('welcome.deepseekPlaceholder')"
              />
            </div>
          </template>

          <template v-else-if="step === 2">
            <p class="leading-relaxed">{{ t('welcome.bochaBody') }}</p>
            <div class="flex flex-wrap gap-2">
              <button
                type="button"
                class="px-3 py-1.5 rounded-lg bg-pink-500 text-white text-xs font-medium hover:bg-pink-600"
                @click="openBochaSite"
              >
                {{ t('welcome.openInBrowser') }}
              </button>
              <button
                type="button"
                class="px-3 py-1.5 rounded-lg border border-gray-200 text-xs font-medium text-gray-700 hover:bg-gray-50"
                @click="openBochaDoc"
              >
                {{ t('welcome.openBochaGuide') }}
              </button>
            </div>
            <div class="space-y-1">
              <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">{{
                t('welcome.bochaApiKey')
              }}</label>
              <input
                v-model="bochaKey"
                type="password"
                autocomplete="off"
                class="w-full px-3 py-2 border border-gray-200 rounded-lg font-mono text-xs"
                :placeholder="t('welcome.bochaPlaceholder')"
              />
            </div>
          </template>

          <template v-else>
            <p class="leading-relaxed">{{ t('welcome.doneBody') }}</p>
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 pt-2">
              <button
                type="button"
                class="px-3 py-2 rounded-lg bg-pink-500 text-white text-xs font-medium hover:bg-pink-600"
                @click="
                  finish({
                    skipped: false,
                    deepseekApiKey: deepseekKey,
                    bochaApiKey: bochaKey,
                    navigate: 'chat',
                  })
                "
              >
                {{ t('welcome.startChat') }}
              </button>
              <button
                type="button"
                class="px-3 py-2 rounded-lg border border-gray-200 text-xs font-medium text-gray-800 hover:bg-gray-50"
                @click="
                  finish({
                    skipped: false,
                    deepseekApiKey: deepseekKey,
                    bochaApiKey: bochaKey,
                    navigate: 'providers',
                  })
                "
              >
                {{ t('welcome.goProviders') }}
              </button>
              <button
                type="button"
                class="px-3 py-2 rounded-lg border border-gray-200 text-xs font-medium text-gray-800 hover:bg-gray-50"
                @click="
                  finish({
                    skipped: false,
                    deepseekApiKey: deepseekKey,
                    bochaApiKey: bochaKey,
                    navigate: 'network',
                  })
                "
              >
                {{ t('welcome.goNetwork') }}
              </button>
              <button
                type="button"
                class="px-3 py-2 rounded-lg border border-gray-200 text-xs font-medium text-gray-800 hover:bg-gray-50"
                @click="
                  finish({
                    skipped: false,
                    deepseekApiKey: deepseekKey,
                    bochaApiKey: bochaKey,
                    navigate: 'console',
                  })
                "
              >
                {{ t('welcome.openConsole') }}
              </button>
            </div>
          </template>
        </div>

        <div
          class="px-6 py-3 border-t border-gray-100 flex flex-wrap items-center justify-between gap-2 shrink-0 bg-gray-50/80"
        >
          <button
            v-if="step === 0"
            type="button"
            class="text-xs text-gray-500 hover:text-gray-800 underline"
            @click="skipAll"
          >
            {{ t('welcome.skip') }}
          </button>
          <button
            v-else
            type="button"
            class="text-xs text-gray-500 hover:text-gray-800"
            @click="step--"
          >
            {{ t('welcome.back') }}
          </button>

          <div class="flex gap-2 ml-auto">
            <template v-if="step < 3">
              <button
                type="button"
                class="px-4 py-2 rounded-lg bg-gray-900 text-white text-xs font-medium hover:bg-gray-800"
                @click="step++"
              >
                {{ t('welcome.next') }}
              </button>
            </template>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

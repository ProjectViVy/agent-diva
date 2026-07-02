<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { openExternalUrl } from '../utils/openExternal';
import {
  MessageSquare,
  Settings,
  Server,
  Zap,
  ArrowRight,
  ArrowLeft,
  SkipForward,
  ExternalLink,
  BookOpen,
  Heart,
  Sparkles,
} from 'lucide-vue-next';

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
const isTransitioning = ref(false);

const steps = computed(() => [
  { id: 'intro', icon: Sparkles, label: t('welcome.stepIntro') },
  { id: 'deepseek', icon: Zap, label: t('welcome.stepDeepseek') },
  { id: 'bocha', icon: Heart, label: t('welcome.stepBocha') },
  { id: 'done', icon: MessageSquare, label: t('welcome.stepDone') },
]);

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

const goNext = () => {
  if (step.value < steps.value.length - 1) {
    isTransitioning.value = true;
    setTimeout(() => {
      step.value++;
      isTransitioning.value = false;
    }, 150);
  }
};

const goBack = () => {
  if (step.value > 0) {
    isTransitioning.value = true;
    setTimeout(() => {
      step.value--;
      isTransitioning.value = false;
    }, 150);
  }
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

const handleFinalAction = (navigate: WelcomeNavigateTarget) => {
  finish({
    skipped: false,
    deepseekApiKey: deepseekKey.value,
    bochaApiKey: bochaKey.value,
    navigate,
  });
};
</script>

<template>
  <Teleport to="body">
    <Transition name="welcome-fade">
      <div
        v-if="open"
        class="welcome-overlay fixed inset-0 z-[200] flex items-center justify-center p-4"
        role="dialog"
        aria-modal="true"
        :aria-label="t('welcome.title')"
      >
        <!-- Floating hearts background -->
        <div class="welcome-hearts">
          <span
            v-for="(h, i) in [
              { left: '5%', top: '10%', size: 20, delay: 0 },
              { left: '15%', top: '75%', size: 14, delay: 0.5 },
              { left: '85%', top: '15%', size: 18, delay: 1 },
              { left: '75%', top: '80%', size: 16, delay: 1.5 },
              { left: '90%', top: '50%', size: 12, delay: 0.8 },
              { left: '3%', top: '60%', size: 22, delay: 1.2 },
            ]"
            :key="i"
            class="welcome-heart"
            :style="{
              left: h.left,
              top: h.top,
              width: `${h.size}px`,
              height: `${h.size}px`,
              animationDelay: `${h.delay}s`,
            }"
          />
        </div>

        <div class="welcome-card">
          <!-- Brand Header -->
          <div class="welcome-header">
            <div class="welcome-brand">
              <div class="welcome-logo">
                <Heart :size="24" class="text-pink-500" />
              </div>
              <div class="welcome-brand-text">
                <h1 class="welcome-title">DiVA</h1>
                <p class="welcome-subtitle">Project ViVY</p>
              </div>
            </div>
            <p class="welcome-tagline">{{ t('welcome.title') }}</p>
          </div>

          <!-- Step Progress -->
          <div class="welcome-progress">
            <div class="welcome-progress-line">
              <div
                class="welcome-progress-fill"
                :style="{ width: `${(step / (steps.length - 1)) * 100}%` }"
              />
            </div>
            <div class="welcome-steps">
              <button
                v-for="(s, i) in steps"
                :key="s.id"
                class="welcome-step"
                :class="{
                  'welcome-step-active': step === i,
                  'welcome-step-completed': step > i,
                }"
                :disabled="i > step"
                @click="i <= step && (step = i)"
              >
                <span class="welcome-step-icon">
                  <component :is="s.icon" :size="14" />
                </span>
                <span class="welcome-step-label">{{ s.label }}</span>
              </button>
            </div>
          </div>

          <!-- Content Area -->
          <div class="welcome-content">
            <Transition name="welcome-slide" mode="out-in">
              <div :key="step" class="welcome-step-content" :class="{ 'is-transitioning': isTransitioning }">
                <!-- Step 0: Introduction -->
                <template v-if="step === 0">
                  <div class="welcome-intro">
                    <div class="welcome-intro-icon">
                      <Sparkles :size="48" class="text-pink-500" />
                    </div>
                    <h2 class="welcome-intro-title">{{ t('welcome.introTitle') }}</h2>
                    <p class="welcome-intro-body">{{ t('welcome.introBody') }}</p>
                    <div class="welcome-features">
                      <div class="welcome-feature">
                        <MessageSquare :size="20" class="text-pink-400" />
                        <span>{{ t('welcome.featureChat') }}</span>
                      </div>
                      <div class="welcome-feature">
                        <Zap :size="20" class="text-pink-400" />
                        <span>{{ t('welcome.featureSearch') }}</span>
                      </div>
                      <div class="welcome-feature">
                        <Settings :size="20" class="text-pink-400" />
                        <span>{{ t('welcome.featureTools') }}</span>
                      </div>
                    </div>
                  </div>
                </template>

                <!-- Step 1: DeepSeek -->
                <template v-else-if="step === 1">
                  <div class="welcome-provider">
                    <div class="welcome-provider-header">
                      <div class="welcome-provider-icon">
                        <Zap :size="24" class="text-pink-500" />
                      </div>
                      <div>
                        <h3 class="welcome-provider-title">{{ t('welcome.deepseekTitle') }}</h3>
                        <p class="welcome-provider-desc">{{ t('welcome.deepseekBody') }}</p>
                      </div>
                    </div>
                    <div class="welcome-provider-actions">
                      <button
                        type="button"
                        class="welcome-btn welcome-btn-outline"
                        @click="openDeepseekSite"
                      >
                        <ExternalLink :size="14" />
                        {{ t('welcome.openInBrowser') }}
                      </button>
                    </div>
                    <div class="welcome-input-group">
                      <label class="welcome-label">{{ t('welcome.deepseekApiKey') }}</label>
                      <input
                        v-model="deepseekKey"
                        type="password"
                        autocomplete="off"
                        class="welcome-input"
                        :placeholder="t('welcome.deepseekPlaceholder')"
                      />
                    </div>
                  </div>
                </template>

                <!-- Step 2: Bocha -->
                <template v-else-if="step === 2">
                  <div class="welcome-provider">
                    <div class="welcome-provider-header">
                      <div class="welcome-provider-icon">
                        <Heart :size="24" class="text-pink-500" />
                      </div>
                      <div>
                        <h3 class="welcome-provider-title">{{ t('welcome.bochaTitle') }}</h3>
                        <p class="welcome-provider-desc">{{ t('welcome.bochaBody') }}</p>
                      </div>
                    </div>
                    <div class="welcome-provider-actions">
                      <button
                        type="button"
                        class="welcome-btn welcome-btn-outline"
                        @click="openBochaSite"
                      >
                        <ExternalLink :size="14" />
                        {{ t('welcome.openInBrowser') }}
                      </button>
                      <button
                        type="button"
                        class="welcome-btn welcome-btn-ghost"
                        @click="openBochaDoc"
                      >
                        <BookOpen :size="14" />
                        {{ t('welcome.openBochaGuide') }}
                      </button>
                    </div>
                    <div class="welcome-input-group">
                      <label class="welcome-label">{{ t('welcome.bochaApiKey') }}</label>
                      <input
                        v-model="bochaKey"
                        type="password"
                        autocomplete="off"
                        class="welcome-input"
                        :placeholder="t('welcome.bochaPlaceholder')"
                      />
                    </div>
                  </div>
                </template>

                <!-- Step 3: Done -->
                <template v-else>
                  <div class="welcome-done">
                    <div class="welcome-done-icon">
                      <Heart :size="48" class="text-pink-500" />
                    </div>
                    <h2 class="welcome-done-title">{{ t('welcome.doneTitle') }}</h2>
                    <p class="welcome-done-body">{{ t('welcome.doneBody') }}</p>
                    <div class="welcome-nav-cards">
                      <button
                        type="button"
                        class="welcome-nav-card welcome-nav-card-primary"
                        @click="handleFinalAction('chat')"
                      >
                        <MessageSquare :size="24" class="text-pink-500" />
                        <div class="welcome-nav-card-text">
                          <span class="welcome-nav-card-title">{{ t('welcome.startChat') }}</span>
                          <span class="welcome-nav-card-desc">{{ t('welcome.startChatDesc') }}</span>
                        </div>
                        <ArrowRight :size="18" class="welcome-nav-card-arrow" />
                      </button>
                      <button
                        type="button"
                        class="welcome-nav-card"
                        @click="handleFinalAction('providers')"
                      >
                        <Settings :size="20" class="text-gray-400" />
                        <div class="welcome-nav-card-text">
                          <span class="welcome-nav-card-title">{{ t('welcome.goProviders') }}</span>
                          <span class="welcome-nav-card-desc">{{ t('welcome.goProvidersDesc') }}</span>
                        </div>
                      </button>
                      <button
                        type="button"
                        class="welcome-nav-card"
                        @click="handleFinalAction('network')"
                      >
                        <Zap :size="20" class="text-gray-400" />
                        <div class="welcome-nav-card-text">
                          <span class="welcome-nav-card-title">{{ t('welcome.goNetwork') }}</span>
                          <span class="welcome-nav-card-desc">{{ t('welcome.goNetworkDesc') }}</span>
                        </div>
                      </button>
                      <button
                        type="button"
                        class="welcome-nav-card"
                        @click="handleFinalAction('console')"
                      >
                        <Server :size="20" class="text-gray-400" />
                        <div class="welcome-nav-card-text">
                          <span class="welcome-nav-card-title">{{ t('welcome.openConsole') }}</span>
                          <span class="welcome-nav-card-desc">{{ t('welcome.openConsoleDesc') }}</span>
                        </div>
                      </button>
                    </div>
                  </div>
                </template>
              </div>
            </Transition>
          </div>

          <!-- Footer -->
          <div class="welcome-footer">
            <div class="welcome-footer-left">
              <button
                v-if="step === 0"
                type="button"
                class="welcome-btn welcome-btn-skip"
                @click="skipAll"
              >
                <SkipForward :size="14" />
                {{ t('welcome.skip') }}
              </button>
              <button
                v-else
                type="button"
                class="welcome-btn welcome-btn-back"
                @click="goBack"
              >
                <ArrowLeft :size="14" />
                {{ t('welcome.back') }}
              </button>
            </div>
            <div class="welcome-footer-right">
              <button
                v-if="step < steps.length - 1"
                type="button"
                class="welcome-btn welcome-btn-primary"
                @click="goNext"
              >
                {{ t('welcome.next') }}
                <ArrowRight :size="14" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.welcome-overlay {
  background: linear-gradient(135deg, rgba(255, 240, 246, 0.98) 0%, rgba(255, 220, 235, 0.95) 50%, rgba(255, 200, 220, 0.98) 100%);
  backdrop-filter: blur(8px);
}

.welcome-hearts {
  position: absolute;
  inset: 0;
  pointer-events: none;
  overflow: hidden;
}

.welcome-heart {
  position: absolute;
  transform: rotate(45deg);
  background: radial-gradient(circle at 30% 30%, rgba(255, 200, 215, 0.8), rgba(255, 122, 156, 0.4));
  border-radius: 4px;
  animation: heart-float 6s ease-in-out infinite;
}

.welcome-heart::before,
.welcome-heart::after {
  content: '';
  position: absolute;
  background: radial-gradient(circle at 30% 30%, rgba(255, 215, 226, 0.85), rgba(255, 122, 156, 0.35));
  border-radius: 50%;
}

.welcome-heart::before {
  width: 100%;
  height: 100%;
  left: -50%;
  top: 0;
}

.welcome-heart::after {
  width: 100%;
  height: 100%;
  left: 0;
  top: -50%;
}

@keyframes heart-float {
  0%, 100% {
    transform: translateY(0) rotate(45deg) scale(1);
    opacity: 0.6;
  }
  50% {
    transform: translateY(-15px) rotate(50deg) scale(1.05);
    opacity: 0.8;
  }
}

.welcome-card {
  position: relative;
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 520px;
  max-height: 90vh;
  background: rgba(255, 255, 255, 0.95);
  border-radius: 24px;
  box-shadow:
    0 4px 6px rgba(236, 72, 153, 0.05),
    0 10px 40px rgba(236, 72, 153, 0.12),
    0 0 0 1px rgba(255, 182, 193, 0.2);
  overflow: hidden;
}

.welcome-header {
  padding: 24px 28px 20px;
  text-align: center;
  border-bottom: 1px solid rgba(255, 182, 193, 0.15);
}

.welcome-brand {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  margin-bottom: 12px;
}

.welcome-logo {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  background: linear-gradient(135deg, rgba(255, 182, 193, 0.3), rgba(255, 200, 215, 0.2));
  border-radius: 14px;
  box-shadow: 0 2px 8px rgba(236, 72, 153, 0.15);
}

.welcome-brand-text {
  text-align: left;
}

.welcome-title {
  font-size: 22px;
  font-weight: 700;
  color: #be185d;
  letter-spacing: 0.5px;
}

.welcome-subtitle {
  font-size: 11px;
  color: #9d174d;
  opacity: 0.7;
  letter-spacing: 1px;
  text-transform: uppercase;
}

.welcome-tagline {
  font-size: 13px;
  color: #6b2737;
  opacity: 0.8;
}

.welcome-progress {
  padding: 16px 28px;
  background: rgba(255, 245, 248, 0.6);
  border-bottom: 1px solid rgba(255, 182, 193, 0.1);
}

.welcome-progress-line {
  position: relative;
  height: 3px;
  background: rgba(255, 182, 193, 0.2);
  border-radius: 2px;
  margin-bottom: 16px;
}

.welcome-progress-fill {
  position: absolute;
  left: 0;
  top: 0;
  height: 100%;
  background: linear-gradient(90deg, #ec4899, #f472b6);
  border-radius: 2px;
  transition: width 0.3s ease;
}

.welcome-steps {
  display: flex;
  justify-content: space-between;
}

.welcome-step {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  background: transparent;
  border: none;
  cursor: pointer;
  opacity: 0.5;
  transition: all 0.2s ease;
}

.welcome-step:hover:not(:disabled) {
  opacity: 0.7;
}

.welcome-step-active {
  opacity: 1;
}

.welcome-step-completed {
  opacity: 0.8;
}

.welcome-step-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  background: rgba(255, 255, 255, 0.8);
  border: 2px solid rgba(255, 182, 193, 0.3);
  border-radius: 50%;
  color: #9d174d;
  transition: all 0.2s ease;
}

.welcome-step-active .welcome-step-icon {
  background: linear-gradient(135deg, #ec4899, #f472b6);
  border-color: #ec4899;
  color: white;
  box-shadow: 0 2px 8px rgba(236, 72, 153, 0.3);
}

.welcome-step-completed .welcome-step-icon {
  background: rgba(236, 72, 153, 0.1);
  border-color: rgba(236, 72, 153, 0.3);
}

.welcome-step-label {
  font-size: 10px;
  font-weight: 500;
  color: #9d174d;
  white-space: nowrap;
}

.welcome-content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 24px 28px;
}

.welcome-step-content {
  opacity: 1;
  transform: translateX(0);
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.welcome-step-content.is-transitioning {
  opacity: 0;
}

/* Introduction Step */
.welcome-intro {
  text-align: center;
}

.welcome-intro-icon {
  display: flex;
  justify-content: center;
  margin-bottom: 16px;
}

.welcome-intro-title {
  font-size: 18px;
  font-weight: 600;
  color: #be185d;
  margin-bottom: 12px;
}

.welcome-intro-body {
  font-size: 13px;
  color: #6b2737;
  line-height: 1.6;
  margin-bottom: 24px;
}

.welcome-features {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.welcome-feature {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: rgba(255, 245, 248, 0.6);
  border-radius: 12px;
  font-size: 13px;
  color: #6b2737;
}

/* Provider Step */
.welcome-provider {
}

.welcome-provider-header {
  display: flex;
  gap: 16px;
  margin-bottom: 20px;
}

.welcome-provider-icon {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  background: linear-gradient(135deg, rgba(255, 182, 193, 0.3), rgba(255, 200, 215, 0.2));
  border-radius: 12px;
}

.welcome-provider-title {
  font-size: 16px;
  font-weight: 600;
  color: #be185d;
  margin-bottom: 6px;
}

.welcome-provider-desc {
  font-size: 12px;
  color: #6b2737;
  line-height: 1.5;
}

.welcome-provider-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 20px;
}

.welcome-input-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.welcome-label {
  font-size: 11px;
  font-weight: 600;
  color: #9d174d;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.welcome-input {
  width: 100%;
  padding: 12px 14px;
  font-size: 13px;
  font-family: 'Inter', ui-monospace, monospace;
  color: #6b2737;
  background: rgba(255, 255, 255, 0.8);
  border: 1px solid rgba(255, 182, 193, 0.4);
  border-radius: 12px;
  outline: none;
  transition: all 0.2s ease;
}

.welcome-input:focus {
  border-color: #ec4899;
  box-shadow: 0 0 0 3px rgba(236, 72, 153, 0.1);
}

.welcome-input::placeholder {
  color: #d1a9b5;
}

/* Done Step */
.welcome-done {
  text-align: center;
}

.welcome-done-icon {
  display: flex;
  justify-content: center;
  margin-bottom: 16px;
}

.welcome-done-title {
  font-size: 18px;
  font-weight: 600;
  color: #be185d;
  margin-bottom: 12px;
}

.welcome-done-body {
  font-size: 13px;
  color: #6b2737;
  line-height: 1.6;
  margin-bottom: 24px;
}

.welcome-nav-cards {
  display: flex;
  flex-direction: column;
  gap: 10px;
  text-align: left;
}

.welcome-nav-card {
  display: flex;
  align-items: center;
  gap: 14px;
  width: 100%;
  padding: 14px 16px;
  background: rgba(255, 255, 255, 0.6);
  border: 1px solid rgba(255, 182, 193, 0.25);
  border-radius: 14px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.welcome-nav-card:hover {
  background: rgba(255, 255, 255, 0.9);
  border-color: rgba(236, 72, 153, 0.4);
  box-shadow: 0 2px 8px rgba(236, 72, 153, 0.1);
}

.welcome-nav-card-primary {
  background: linear-gradient(135deg, rgba(255, 240, 246, 0.95), rgba(255, 230, 240, 0.9));
  border-color: rgba(236, 72, 153, 0.3);
  padding: 18px 16px;
}

.welcome-nav-card-primary:hover {
  background: linear-gradient(135deg, rgba(255, 235, 245, 1), rgba(255, 225, 238, 1));
  border-color: #ec4899;
  box-shadow: 0 4px 16px rgba(236, 72, 153, 0.2);
}

.welcome-nav-card-text {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.welcome-nav-card-title {
  font-size: 14px;
  font-weight: 600;
  color: #be185d;
}

.welcome-nav-card-desc {
  font-size: 11px;
  color: #9d174d;
  opacity: 0.7;
}

.welcome-nav-card-arrow {
  color: #ec4899;
}

/* Footer */
.welcome-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 28px;
  background: rgba(255, 250, 252, 0.8);
  border-top: 1px solid rgba(255, 182, 193, 0.15);
}

.welcome-footer-left,
.welcome-footer-right {
  display: flex;
  gap: 10px;
}

/* Buttons */
.welcome-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 10px 16px;
  font-size: 13px;
  font-weight: 500;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s ease;
  border: none;
}

.welcome-btn-primary {
  background: linear-gradient(135deg, #ec4899, #f472b6);
  color: white;
  box-shadow: 0 2px 8px rgba(236, 72, 153, 0.25);
}

.welcome-btn-primary:hover {
  box-shadow: 0 4px 12px rgba(236, 72, 153, 0.35);
  transform: translateY(-1px);
}

.welcome-btn-outline {
  background: transparent;
  border: 1px solid rgba(236, 72, 153, 0.3);
  color: #be185d;
}

.welcome-btn-outline:hover {
  background: rgba(236, 72, 153, 0.05);
  border-color: #ec4899;
}

.welcome-btn-ghost {
  background: transparent;
  color: #9d174d;
  opacity: 0.8;
}

.welcome-btn-ghost:hover {
  opacity: 1;
  background: rgba(236, 72, 153, 0.05);
}

.welcome-btn-skip {
  background: transparent;
  color: #9d174d;
  opacity: 0.6;
  font-size: 12px;
  padding: 8px 12px;
}

.welcome-btn-skip:hover {
  opacity: 1;
}

.welcome-btn-back {
  background: rgba(255, 182, 193, 0.15);
  color: #be185d;
}

.welcome-btn-back:hover {
  background: rgba(255, 182, 193, 0.25);
}

/* Transitions */
.welcome-fade-enter-active,
.welcome-fade-leave-active {
  transition: opacity 0.25s ease;
}

.welcome-fade-enter-from,
.welcome-fade-leave-to {
  opacity: 0;
}

.welcome-slide-enter-active,
.welcome-slide-leave-active {
  transition: all 0.2s ease;
}

.welcome-slide-enter-from {
  opacity: 0;
  transform: translateX(20px);
}

.welcome-slide-leave-to {
  opacity: 0;
  transform: translateX(-20px);
}

/* Scrollbar */
.welcome-content::-webkit-scrollbar {
  width: 6px;
}

.welcome-content::-webkit-scrollbar-track {
  background: transparent;
}

.welcome-content::-webkit-scrollbar-thumb {
  background: rgba(236, 72, 153, 0.2);
  border-radius: 3px;
}

.welcome-content::-webkit-scrollbar-thumb:hover {
  background: rgba(236, 72, 153, 0.35);
}
</style>
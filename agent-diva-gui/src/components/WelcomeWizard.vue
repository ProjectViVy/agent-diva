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
  Heart,
} from '@lucide/vue';

const { t } = useI18n();

const DEEPSEEK_PLATFORM_URL = 'https://platform.deepseek.com/';

type WelcomeNavigateTarget = 'chat' | 'providers' | 'network' | 'console';

interface WelcomeDonePayload {
  skipped: boolean;
  deepseekApiKey: string;
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
}>();

const emit = defineEmits<{
  (e: 'done', payload: WelcomeDonePayload): void;
}>();

const step = ref(0);
const deepseekKey = ref('');
const isTransitioning = ref(false);

const steps = computed(() => [
  { id: 'intro', icon: MessageSquare, label: t('welcome.stepIntro') },
  { id: 'deepseek', icon: Zap, label: t('welcome.stepDeepseek') },
  { id: 'done', icon: MessageSquare, label: t('welcome.stepDone') },
]);

const resetFromProps = () => {
  deepseekKey.value = props.config.apiKey || '';
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
    navigate: payload.navigate,
  });
};

const skipAll = () => {
  finish({
    skipped: true,
    deepseekApiKey: '',
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

const handleFinalAction = (navigate: WelcomeNavigateTarget) => {
  finish({
    skipped: false,
    deepseekApiKey: deepseekKey.value,
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


        <div class="welcome-card">
          <!-- Brand Header -->
          <div class="welcome-header">
            <div class="welcome-brand">
              <div class="welcome-logo">
                <Heart :size="24" class="text-primary" />
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
                      <MessageSquare :size="48" class="text-primary" />
                    </div>
                    <h2 class="welcome-intro-title">{{ t('welcome.introTitle') }}</h2>
                    <p class="welcome-intro-body">{{ t('welcome.introBody') }}</p>
                    <div class="welcome-features">
                      <div class="welcome-feature">
                        <MessageSquare :size="20" class="text-primary" />
                        <span>{{ t('welcome.featureChat') }}</span>
                      </div>
                      <div class="welcome-feature">
                        <Zap :size="20" class="text-primary" />
                        <span>{{ t('welcome.featureSearch') }}</span>
                      </div>
                      <div class="welcome-feature">
                        <Settings :size="20" class="text-primary" />
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
                        <Zap :size="24" class="text-primary" />
                      </div>
                      <div>
                        <h3 class="welcome-provider-title">{{ t('welcome.deepseekTitle') }}</h3>
                        <p class="welcome-provider-desc">{{ t('welcome.deepseekBody') }}</p>
                      </div>
                    </div>
                    <div class="welcome-provider-actions">
                      <button
                        type="button"
                        class="ui-button ui-button--ghost welcome-btn welcome-btn-outline"
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
                        class="ui-input welcome-input"
                        :placeholder="t('welcome.deepseekPlaceholder')"
                      />
                    </div>
                  </div>
                </template>

                <!-- Step 2: Done -->
                <template v-else>
                  <div class="welcome-done">
                    <div class="welcome-done-icon">
                      <Heart :size="48" class="text-primary" />
                    </div>
                    <h2 class="welcome-done-title">{{ t('welcome.doneTitle') }}</h2>
                    <p class="welcome-done-body">{{ t('welcome.doneBody') }}</p>
                    <div class="welcome-nav-cards">
                      <button
                        type="button"
                        class="welcome-nav-card welcome-nav-card-primary"
                        @click="handleFinalAction('chat')"
                      >
                        <MessageSquare :size="24" class="text-primary-foreground" />
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
                        <Settings :size="20" class="text-muted-foreground" />
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
                        <Zap :size="20" class="text-muted-foreground" />
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
                        <Server :size="20" class="text-muted-foreground" />
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
                class="ui-button ui-button--ghost welcome-btn welcome-btn-skip"
                @click="skipAll"
              >
                <SkipForward :size="14" />
                {{ t('welcome.skip') }}
              </button>
              <button
                v-else
                type="button"
                class="ui-button ui-button--ghost welcome-btn welcome-btn-back"
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
                class="ui-button ui-button--primary welcome-btn welcome-btn-primary"
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
  background: var(--background);
}

.welcome-card {
  position: relative;
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 520px;
  max-height: 90vh;
  background: var(--card);
  border-radius: 24px;
  box-shadow:
    var(--shadow-sm);
  overflow: hidden;
}

.welcome-header {
  padding: 24px 28px 20px;
  text-align: center;
  border-bottom: 1px solid var(--border);
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
  background: var(--card);
  border-radius: 14px;
  box-shadow: var(--shadow-sm);
}

.welcome-brand-text {
  text-align: left;
}

.welcome-title {
  font-size: 22px;
  font-weight: 700;
  color: var(--foreground);
  letter-spacing: 0.5px;
}

.welcome-subtitle {
  font-size: var(--font-size-xs);
  color: var(--muted-foreground);
  letter-spacing: 1px;
  text-transform: uppercase;
}

.welcome-tagline {
  font-size: var(--font-size-base);
  color: var(--muted-foreground);
}

.welcome-progress {
  padding: 16px 28px;
  background: var(--card);
  border-bottom: 1px solid var(--border);
}

.welcome-progress-line {
  position: relative;
  height: 3px;
  background: var(--accent);
  border-radius: 2px;
  margin-bottom: 16px;
}

.welcome-progress-fill {
  position: absolute;
  left: 0;
  top: 0;
  height: 100%;
  background: var(--primary);
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
  background: var(--card);
  border: 2px solid var(--border);
  border-radius: 50%;
  color: var(--muted-foreground);
  transition: all 0.2s ease;
}

.welcome-step-active .welcome-step-icon {
  background: var(--primary-action, var(--primary));
  border-color: var(--primary-action, var(--primary));
  color: var(--primary-action-foreground, var(--primary-foreground));
  box-shadow: var(--shadow-sm);
}

.welcome-step-completed .welcome-step-icon {
  background: var(--accent);
  border-color: var(--primary);
  color: var(--primary);
}

.welcome-step-label {
  font-size: var(--font-size-xs);
  font-weight: 500;
  color: var(--muted-foreground);
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
  color: var(--foreground);
  margin-bottom: 12px;
}

.welcome-intro-body {
  font-size: var(--font-size-base);
  color: var(--muted-foreground);
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
  background: var(--card);
  border-radius: 12px;
  font-size: var(--font-size-base);
  color: var(--foreground);
}

/* Provider Step */

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
  background: var(--card);
  border-radius: 12px;
}

.welcome-provider-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--foreground);
  margin-bottom: 6px;
}

.welcome-provider-desc {
  font-size: 12px;
  color: var(--muted-foreground);
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
  font-size: var(--font-size-xs);
  font-weight: 600;
  color: var(--foreground);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.welcome-input {
  width: 100%;
  font-family: 'Inter', ui-monospace, monospace;
  transition: all 0.2s ease;
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
  color: var(--foreground);
  margin-bottom: 12px;
}

.welcome-done-body {
  font-size: var(--font-size-base);
  color: var(--muted-foreground);
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
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 14px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.welcome-nav-card:hover {
  background: var(--accent);
  border-color: var(--border-strong);
  box-shadow: var(--shadow-sm);
}

.welcome-nav-card-primary {
  background: var(--primary-action, var(--primary));
  border-color: var(--primary-action, var(--primary));
  padding: 18px 16px;
  color: var(--primary-action-foreground, var(--primary-foreground));
}

.welcome-nav-card-primary:hover {
  background: var(--primary-action-hover, var(--primary-hover));
  border-color: var(--primary-action-hover, var(--primary-hover));
  color: var(--primary-action-hover-foreground, var(--primary-action-foreground, var(--primary-foreground)));
  box-shadow: var(--shadow-sm);
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
  color: var(--foreground);
}

.welcome-nav-card-desc {
  font-size: var(--font-size-xs);
  color: var(--muted-foreground);
}

.welcome-nav-card-arrow {
  color: var(--primary);
}

/* Footer */
.welcome-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 28px;
  background: var(--card);
  border-top: 1px solid var(--border);
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
  transition: all 0.2s ease;
}

.welcome-btn-skip {
  opacity: 0.6;
}

.welcome-btn-skip:hover {
  opacity: 1;
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
  background: var(--border-strong);
  border-radius: 3px;
}

.welcome-content::-webkit-scrollbar-thumb:hover {
  background: var(--border-strong);
}

.welcome-nav-card-primary .welcome-nav-card-title, .welcome-nav-card-primary .welcome-nav-card-desc, .welcome-nav-card-primary .welcome-nav-card-arrow {
  color: inherit;
}
</style>

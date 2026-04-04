<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import {
  X,
  ChevronRight,
  LoaderCircle,
  Check,
  CircleAlert,
  PlugZap,
  Eye,
  EyeOff,
  BookOpen,
  Monitor,
  Terminal,
  FileText,
  Lightbulb,
} from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import {
  PLATFORM_ICONS,
  PLATFORM_DISPLAY_NAMES,
  PLATFORM_DESCRIPTIONS,
} from './channel-icons';
import {
  CHANNEL_PLATFORMS,
  validateConfig,
  type ChannelPlatformInfo,
  type WizardFormField,
} from './channel-platforms';
import TutorialModal from './TutorialModal.vue';

const { t } = useI18n();

interface ChannelWizardData {
  platform: string;
  name: string;
  credentials: Record<string, any>;
  extra?: Record<string, any>;
}

const props = defineProps<{
  open: boolean;
  initialData?: Partial<ChannelWizardData>;
}>();

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void;
  (e: 'test', data: ChannelWizardData): Promise<{ success: boolean; message: string }>;
  (e: 'complete', data: ChannelWizardData): void;
}>();

type StepKey = 'platform' | 'credentials' | 'test' | 'done';

interface Step {
  key: StepKey;
  title: string;
}

const steps: Step[] = [
  { key: 'platform', title: t('channels.wizardStepPlatform') },
  { key: 'credentials', title: t('channels.wizardStepCredentials') },
  { key: 'test', title: t('channels.wizardStepTest') },
];

const currentStep = ref<StepKey>('platform');

const formData = ref<ChannelWizardData>({
  platform: props.initialData?.platform || '',
  name: props.initialData?.name || '',
  credentials: props.initialData?.credentials || {},
  extra: props.initialData?.extra,
});

const isTesting = ref(false);
const testResult = ref<'idle' | 'success' | 'failed'>('idle');
const testMessage = ref('');
const revealedSecrets = ref<Set<string>>(new Set());

const currentStepIndex = computed(() => steps.findIndex((s) => s.key === currentStep.value));

const canNext = computed(() => {
  if (currentStep.value === 'platform') return formData.value.platform;
  if (currentStep.value === 'credentials') {
    const { valid } = validateConfig(formData.value.platform, formData.value.credentials);
    return valid;
  }
  if (currentStep.value === 'test') return testResult.value === 'success';
  return false;
});

const credentialFields = computed<WizardFormField[]>(() => {
  if (!formData.value.platform) return [];
  return CHANNEL_PLATFORMS[formData.value.platform]?.credentialFields || [];
});

const currentPlatform = computed<ChannelPlatformInfo | null>(() => {
  if (!formData.value.platform) return null;
  return CHANNEL_PLATFORMS[formData.value.platform] || null;
});

const tutorialOpen = ref(false);

const openTutorial = () => {
  tutorialOpen.value = true;
};

const nextStep = () => {
  const currentIndex = currentStepIndex.value;
  if (currentIndex < steps.length - 1) {
    currentStep.value = steps[currentIndex + 1].key;
    if (currentStep.value === 'test') {
      // Auto-test on enter test step
      testConnection();
    }
  }
};

const prevStep = () => {
  const currentIndex = currentStepIndex.value;
  if (currentIndex > 0) {
    currentStep.value = steps[currentIndex - 1].key;
  }
};

const testConnection = async () => {
  isTesting.value = true;
  testResult.value = 'idle';
  testMessage.value = '';

  try {
    const result = await emit('test', { ...formData.value });
    testResult.value = result.success ? 'success' : 'failed';
    testMessage.value = result.message;
  } catch (error) {
    testResult.value = 'failed';
    testMessage.value = error instanceof Error ? error.message : t('channels.testFailed');
  } finally {
    isTesting.value = false;
  }
};

const complete = () => {
  emit('complete', { ...formData.value });
  emit('update:open', false);
};

const close = () => {
  emit('update:open', false);
  resetForm();
};

const resetForm = () => {
  currentStep.value = 'platform';
  formData.value = {
    platform: '',
    name: '',
    credentials: {},
    extra: undefined,
  };
  testResult.value = 'idle';
  testMessage.value = '';
  revealedSecrets.value = new Set();
};

const selectPlatform = (platform: string) => {
  formData.value.platform = platform;
  // Auto-generate name from platform display name
  formData.value.name = PLATFORM_DISPLAY_NAMES[platform] || platform;
};

const toggleRevealed = (key: string) => {
  if (revealedSecrets.value.has(key)) {
    revealedSecrets.value.delete(key);
  } else {
    revealedSecrets.value.add(key);
  }
};

// Watch for open state changes
watch(
  () => props.open,
  (newOpen) => {
    if (!newOpen) {
      resetForm();
    }
  }
);
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="open" class="wizard-overlay" @click.self="close">
        <div class="wizard-modal">
          <!-- Header -->
          <div class="wizard-header">
            <h3 class="wizard-title">{{ t('channels.wizardTitle') }}</h3>
            <button class="wizard-close" @click="close">
              <X :size="18" />
            </button>
          </div>

          <!-- Progress Steps -->
          <div class="wizard-progress">
            <div
              v-for="(step, index) in steps"
              :key="step.key"
              class="wizard-progress-item"
              :class="{
                active: step.key === currentStep,
                completed: index < currentStepIndex,
              }"
            >
              <div class="wizard-progress-indicator">
                <Check v-if="index < currentStepIndex" :size="12" />
                <span v-else>{{ index + 1 }}</span>
              </div>
              <span class="wizard-progress-label">{{ step.title }}</span>
            </div>
          </div>

          <!-- Content -->
          <div class="wizard-content">
            <!-- Step 1: Select Platform -->
            <div v-if="currentStep === 'platform'" class="wizard-step">
              <label class="wizard-label">{{ t('channels.choosePlatform') }}</label>
              <div class="platform-grid">
                <div
                  v-for="(Icon, platform) in PLATFORM_ICONS"
                  :key="platform"
                  class="platform-card"
                  :class="{ selected: formData.platform === platform }"
                  @click="selectPlatform(platform)"
                >
                  <Icon :size="32" />
                  <span class="platform-name">{{ PLATFORM_DISPLAY_NAMES[platform] }}</span>
                  <span class="platform-desc">{{ PLATFORM_DESCRIPTIONS[platform] }}</span>
                </div>
              </div>
            </div>

            <!-- Step 2: Credentials -->
            <div v-if="currentStep === 'credentials'" class="wizard-step">
              <label class="wizard-label">{{ t('channels.enterCredentials') }}</label>
              
              <!-- 快速获取凭证指引 -->
              <div v-if="currentPlatform" class="quick-guide-panel">
                <div class="quick-guide-header">
                  <Lightbulb :size="18" />
                  <h4>如何获取 {{ currentPlatform.displayName }} 凭证？</h4>
                </div>
                <div class="quick-guide-steps">
                  <ol>
                    <li v-for="(step, index) in currentPlatform.quickGuideSteps" :key="index">
                      {{ step }}
                    </li>
                  </ol>
                </div>
                <button class="view-full-tutorial-btn" @click="openTutorial">
                  📖 查看完整配置教程（含截图）
                </button>
              </div>
              
              <div class="credential-form">
                <div
                  v-for="field in credentialFields"
                  :key="field.key"
                  class="credential-field"
                >
                  <label class="credential-label">
                    {{ field.label }}
                    <span v-if="field.required" class="required-mark">*</span>
                  </label>

                  <!-- Select Type -->
                  <select
                    v-if="field.type === 'select'"
                    v-model="formData.credentials[field.key]"
                    class="credential-input"
                  >
                    <option
                      v-for="opt in field.options"
                      :key="opt.value"
                      :value="opt.value"
                    >
                      {{ opt.label }}
                    </option>
                  </select>

                  <!-- Textarea Type -->
                  <textarea
                    v-else-if="field.type === 'textarea'"
                    v-model="formData.credentials[field.key]"
                    :placeholder="field.placeholder"
                    class="credential-input"
                    rows="3"
                  />

                  <!-- Password Type with Toggle -->
                  <div v-else-if="field.type === 'password'" class="credential-input-wrapper">
                    <input
                      v-model="formData.credentials[field.key]"
                      :type="revealedSecrets.has(field.key) ? 'text' : 'password'"
                      :placeholder="field.placeholder"
                      class="credential-input"
                      autocomplete="off"
                    />
                    <button
                      v-if="field.secret"
                      type="button"
                      class="input-toggle"
                      @click="toggleRevealed(field.key)"
                      :title="revealedSecrets.has(field.key) ? '隐藏' : '显示'"
                    >
                      <EyeOff v-if="revealedSecrets.has(field.key)" :size="16" />
                      <Eye v-else :size="16" />
                    </button>
                  </div>

                  <!-- Default: Text/Number -->
                  <input
                    v-else
                    v-model="formData.credentials[field.key]"
                    :type="field.type || 'text'"
                    :placeholder="field.placeholder"
                    :value="formData.credentials[field.key] ?? field.default ?? ''"
                    class="credential-input"
                  />

                  <!-- Hint -->
                  <p v-if="field.hint" class="credential-hint">{{ field.hint }}</p>
                </div>
              </div>
            </div>

            <!-- Step 3: Test -->
            <div v-if="currentStep === 'test'" class="wizard-step">
              <label class="wizard-label">{{ t('channels.testConnection') }}</label>
              <div class="test-area">
                <button
                  class="test-btn"
                  @click="testConnection"
                  :disabled="isTesting"
                >
                  <LoaderCircle v-if="isTesting" :size="16" class="animate-spin" />
                  <PlugZap v-else :size="16" />
                  <span>{{ isTesting ? t('channels.testingConnection') : t('channels.testConnection') }}</span>
                </button>

                <div v-if="testResult === 'success'" class="test-result success">
                  <Check :size="16" />
                  <span>{{ testMessage }}</span>
                </div>
                <div v-if="testResult === 'failed'" class="test-result failed">
                  <CircleAlert :size="16" />
                  <span>{{ testMessage }}</span>
                </div>
              </div>
            </div>

            <!-- Step 4: Done -->
            <div v-if="currentStep === 'done'" class="wizard-step">
              <div class="done-area">
                <div class="done-icon">
                  <Check :size="48" />
                </div>
                <h4 class="done-title">{{ t('channels.wizardDone') }}</h4>
                <p class="done-desc">{{ t('channels.wizardDoneHint') }}</p>
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="wizard-footer">
            <button
              v-if="currentStepIndex > 0 && currentStep !== 'done'"
              class="wizard-btn wizard-btn-secondary"
              @click="prevStep"
            >
              {{ t('channels.wizardBack') }}
            </button>

            <button
              v-if="currentStep !== 'done' && currentStep !== 'test'"
              class="wizard-btn wizard-btn-primary"
              :disabled="!canNext"
              @click="nextStep"
            >
              {{ t('channels.wizardNext') }}
              <ChevronRight :size="16" />
            </button>

            <button
              v-if="currentStep === 'test'"
              class="wizard-btn wizard-btn-primary"
              :disabled="testResult !== 'success'"
              @click="nextStep"
            >
              {{ t('channels.wizardNext') }}
              <ChevronRight :size="16" />
            </button>

            <button
              v-if="currentStep === 'done'"
              class="wizard-btn wizard-btn-primary"
              @click="complete"
            >
              {{ t('channels.wizardFinish') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
    
    <!-- Tutorial Modal -->
    <TutorialModal 
      v-model:open="tutorialOpen"
      :platform="currentPlatform"
      @start-config="nextStep"
    />
  </Teleport>
</template>

<style scoped>
/* 继承 ProviderWizardModal 的基础样式，添加通道特定样式 */
.wizard-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.wizard-modal {
  background: var(--panel-solid);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  width: 100%;
  max-width: 720px;
  max-height: 90vh;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.wizard-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid var(--line);
}

.wizard-title {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--text);
}

.wizard-close {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.wizard-close:hover {
  background: var(--accent-bg-light);
  color: var(--accent);
}

.wizard-progress {
  display: flex;
  gap: 1rem;
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid var(--line);
}

.wizard-progress-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  flex: 1;
}

.wizard-progress-indicator {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: var(--panel);
  border: 2px solid var(--line);
  color: var(--text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.75rem;
  font-weight: 600;
  transition: all 0.15s ease;
}

.wizard-progress-item.active .wizard-progress-indicator {
  border-color: var(--accent);
  color: var(--accent);
}

.wizard-progress-item.completed .wizard-progress-indicator {
  background: var(--accent);
  border-color: var(--accent);
  color: white;
}

.wizard-progress-label {
  font-size: 0.625rem;
  color: var(--text-muted);
  text-align: center;
}

.wizard-content {
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem;
}

.wizard-step {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.wizard-label {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
}

/* Platform Grid */
.platform-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 1rem;
}

.platform-card {
  padding: 1.5rem 1rem;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel);
  cursor: pointer;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.75rem;
  transition: all 0.15s ease;
}

.platform-card:hover {
  border-color: var(--accent);
  background: var(--accent-bg-light);
  transform: translateY(-2px);
}

.platform-card.selected {
  border-color: var(--accent);
  background: var(--accent);
  color: white;
}

.platform-card.selected .platform-desc {
  color: rgba(255, 255, 255, 0.8);
}

.platform-name {
  font-size: 0.875rem;
  font-weight: 600;
}

.platform-desc {
  font-size: 0.75rem;
  color: var(--text-muted);
  text-align: center;
}

/* Credential Form */
.credential-form {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.credential-field {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.credential-label {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text);
  display: flex;
  align-items: center;
  gap: 0.25rem;
}

.required-mark {
  color: var(--danger);
}

.credential-input,
.credential-input-wrapper {
  width: 100%;
}

.credential-input {
  width: 100%;
  padding: 0.75rem 1rem;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel);
  color: var(--text);
  font-size: 0.875rem;
  transition: all 0.15s ease;
}

.credential-input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-glow);
}

.credential-input-wrapper {
  position: relative;
}

.credential-input-wrapper .credential-input {
  padding-right: 3rem;
}

.input-toggle {
  position: absolute;
  right: 0.75rem;
  top: 50%;
  transform: translateY(-50%);
  width: 24px;
  height: 24px;
  border-radius: 4px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.input-toggle:hover {
  background: var(--accent-bg-light);
  color: var(--accent);
}

.credential-hint {
  font-size: 0.75rem;
  color: var(--text-muted);
  margin-top: 0.25rem;
}

/* 快速获取凭证指引面板 */
.quick-guide-panel {
  padding: 1.25rem;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.05) 0%, rgba(59, 130, 246, 0.02) 100%);
  border: 1px solid rgba(59, 130, 246, 0.2);
  border-radius: var(--radius);
  margin-bottom: 1.5rem;
}

.quick-guide-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 1rem;
  color: var(--accent);
}

.quick-guide-header h4 {
  font-size: 0.875rem;
  font-weight: 600;
  margin: 0;
  color: var(--text);
}

.quick-guide-steps {
  margin-bottom: 1rem;
}

.quick-guide-steps ol {
  margin: 0;
  padding-left: 1.5rem;
}

.quick-guide-steps li {
  font-size: 0.875rem;
  color: var(--text);
  padding: 0.25rem 0;
  line-height: 1.6;
}

.view-full-tutorial-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  background: var(--accent);
  color: white;
  border: none;
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.view-full-tutorial-btn:hover {
  filter: brightness(1.1);
  transform: translateY(-1px);
}

/* Test Area */
.test-area {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.test-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.75rem 1.5rem;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel);
  color: var(--text);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.test-btn:hover:not(:disabled) {
  background: var(--accent-bg-light);
  border-color: var(--accent);
  color: var(--accent);
}

.test-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.test-result {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  font-weight: 500;
}

.test-result.success {
  background: var(--success);
  color: white;
}

.test-result.failed {
  background: var(--danger);
  color: white;
}

/* Done Area */
.done-area {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 2rem 0;
}

.done-icon {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  background: var(--success);
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 1.5rem;
}

.done-title {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 0.5rem;
}

.done-desc {
  font-size: 0.875rem;
  color: var(--text-muted);
}

.wizard-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 1.25rem 1.5rem;
  border-top: 1px solid var(--line);
}

.wizard-btn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.625rem 1.25rem;
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.wizard-btn-primary {
  background: var(--accent);
  color: white;
  border: none;
}

.wizard-btn-primary:hover:not(:disabled) {
  filter: brightness(1.1);
}

.wizard-btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.wizard-btn-secondary {
  background: var(--panel);
  color: var(--text);
  border: 1px solid var(--line);
}

.wizard-btn-secondary:hover {
  background: var(--accent-bg-light);
}

/* Modal transitions */
.modal-enter-active,
.modal-leave-active {
  transition: all 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .wizard-modal,
.modal-leave-to .wizard-modal {
  transform: scale(0.95) translateY(-20px);
}
</style>

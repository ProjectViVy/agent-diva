<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import { X, ChevronRight, LoaderCircle, Check, CircleAlert, PlugZap, Eye, EyeOff } from '@lucide/vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

interface ProviderSpec {
  name: string;
  display_name: string;
  default_api_base: string;
}

interface WizardData {
  selectedProvider: string;
  apiKey: string;
  apiBase: string;
}

const props = defineProps<{
  open: boolean;
  providers: ProviderSpec[];
  initialData?: Partial<WizardData>;
}>();

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void;
  (e: 'test', data: WizardData): Promise<{ success: boolean; message: string; latency?: number }>;
  (e: 'complete', data: WizardData): void;
}>();

type StepKey = 'select' | 'apikey' | 'apibase' | 'test' | 'done';

interface Step {
  key: StepKey;
  title: string;
}

const steps: Step[] = [
  { key: 'select', title: t('providers.wizardStepSelect') },
  { key: 'apikey', title: t('providers.wizardStepApiKey') },
  { key: 'apibase', title: t('providers.wizardStepApiBase') },
  { key: 'test', title: t('providers.wizardStepTest') },
  { key: 'done', title: t('providers.wizardDone') },
];

const currentStep = ref<StepKey>('select');

const formData = ref<WizardData>({
  selectedProvider: props.initialData?.selectedProvider || '',
  apiKey: props.initialData?.apiKey || '',
  apiBase: props.initialData?.apiBase || '',
});

const isTesting = ref(false);
const testResult = ref<'idle' | 'success' | 'failed'>('idle');
const testMessage = ref('');
const testLatency = ref<number | undefined>(undefined);
const isApiKeyVisible = ref(false);

const currentStepIndex = computed(() =>
  steps.findIndex(s => s.key === currentStep.value)
);

const canNext = computed(() => {
  if (currentStep.value === 'select') return formData.value.selectedProvider;
  if (currentStep.value === 'apikey') return formData.value.apiKey.length > 0;
  if (currentStep.value === 'apibase') return true;
  if (currentStep.value === 'test') return testResult.value === 'success';
  return false;
});

const selectedProviderSpec = computed(() =>
  props.providers.find(p => p.name === formData.value.selectedProvider)
);

const nextStep = () => {
  const currentIndex = currentStepIndex.value;
  if (currentIndex < steps.length - 1) {
    currentStep.value = steps[currentIndex + 1].key;
    if (currentStep.value === 'test') {
      // Auto-fill API base with default if empty
      if (!formData.value.apiBase && selectedProviderSpec.value) {
        formData.value.apiBase = selectedProviderSpec.value.default_api_base;
      }
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
  testLatency.value = undefined;

  try {
    const result = await emit('test', { ...formData.value });
    testResult.value = result.success ? 'success' : 'failed';
    testMessage.value = result.message;
    testLatency.value = result.latency;
  } catch (error) {
    testResult.value = 'failed';
    testMessage.value = error instanceof Error ? error.message : t('providers.testFailed');
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
  currentStep.value = 'select';
  formData.value = {
    selectedProvider: '',
    apiKey: '',
    apiBase: '',
  };
  testResult.value = 'idle';
  testMessage.value = '';
  testLatency.value = undefined;
  isApiKeyVisible.value = false;
};

// Keyboard handler: Escape to close
const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && props.open) {
    close();
  }
};

onMounted(() => {
  document.addEventListener('keydown', onKeydown);
});

onUnmounted(() => {
  document.removeEventListener('keydown', onKeydown);
});

// Watch for initial data changes
watch(() => props.initialData, (newData) => {
  if (newData) {
    formData.value = {
      selectedProvider: newData.selectedProvider || '',
      apiKey: newData.apiKey || '',
      apiBase: newData.apiBase || '',
    };
  }
}, { immediate: true });
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="open" class="wizard-overlay" @click.self="close">
        <div class="wizard-modal">
          <!-- Header -->
          <div class="wizard-header">
            <h3 class="wizard-title">{{ t('providers.wizardTitle') }}</h3>
            <button class="ui-button ui-button--ghost wizard-close ui-button--compact ui-button--icon" :aria-label="t('app.close')" @click="close">
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
                completed: index < currentStepIndex
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
            <!-- Step 1: Select Provider -->
            <div v-if="currentStep === 'select'" class="wizard-step">
              <label class="wizard-label">{{ t('providers.selectProviderType') }}</label>
              <select
                v-model="formData.selectedProvider"
                class="ui-input wizard-select"
              >
                <option value="" disabled>{{ t('providers.chooseProvider') }}</option>
                <option
                  v-for="provider in providers"
                  :key="provider.name"
                  :value="provider.name"
                >
                  {{ provider.display_name }}
                </option>
              </select>
            </div>

            <!-- Step 2: API Key -->
            <div v-if="currentStep === 'apikey'" class="wizard-step">
              <label class="wizard-label">{{ t('providers.apiKey') }}</label>
              <div class="wizard-input-wrapper">
                <input
                  v-model="formData.apiKey"
                  :type="isApiKeyVisible ? 'text' : 'password'"
                  :placeholder="t('providers.enterApiKey')"
                  class="ui-input ui-input--trailing-icon wizard-input"
                  autocomplete="off"
                />
                <button
                  type="button"
                  class="ui-button ui-button--ghost wizard-input-toggle ui-button--compact ui-button--icon"
                  @click="isApiKeyVisible = !isApiKeyVisible"
                  :title="isApiKeyVisible ? t('providers.hideApiKey') : t('providers.showApiKey')"
                >
                  <EyeOff v-if="isApiKeyVisible" :size="16" />
                  <Eye v-else :size="16" />
                </button>
              </div>
            </div>

            <!-- Step 3: API Base -->
            <div v-if="currentStep === 'apibase'" class="wizard-step">
              <label class="wizard-label">{{ t('providers.apiBaseUrl') }}</label>
              <input
                v-model="formData.apiBase"
                type="text"
                :placeholder="selectedProviderSpec?.default_api_base || t('providers.placeholderLocalCustom')"
                class="ui-input wizard-input"
              />
              <p class="wizard-hint">{{ t('providers.apiBaseHint') }}</p>
            </div>

            <!-- Step 4: Test -->
            <div v-if="currentStep === 'test'" class="wizard-step">
              <label class="wizard-label">{{ t('providers.testConnection') }}</label>
              <div class="wizard-test-area">
                <button
                  class="ui-button ui-button--ghost wizard-test-btn"
                  @click="testConnection"
                  :disabled="isTesting"
                >
                  <LoaderCircle v-if="isTesting" :size="16" class="animate-spin" />
                  <PlugZap v-else :size="16" />
                  <span>{{ isTesting ? t('providers.testingConnection') : t('providers.testConnection') }}</span>
                </button>

                <div v-if="testResult === 'success'" class="wizard-test-result success">
                  <Check :size="16" />
                  <span>
                    {{ testMessage }}
                    <span v-if="testLatency" class="wizard-test-latency">
                      ({{ testLatency }}ms)
                    </span>
                  </span>
                </div>
                <div v-if="testResult === 'failed'" class="wizard-test-result failed">
                  <CircleAlert :size="16" />
                  <span>{{ testMessage }}</span>
                </div>
              </div>
            </div>

            <!-- Step 5: Done -->
            <div v-if="currentStep === 'done'" class="wizard-step">
              <div class="wizard-done">
                <div class="wizard-done-icon">
                  <Check :size="48" />
                </div>
                <h4 class="wizard-done-title">{{ t('providers.wizardDone') }}</h4>
                <p class="wizard-done-desc">{{ t('providers.wizardDoneHint') }}</p>
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="wizard-footer">
            <button
              v-if="currentStepIndex > 0 && currentStep !== 'done'"
              class="ui-button ui-button--outline wizard-btn wizard-btn-secondary"
              @click="prevStep"
            >
              {{ t('providers.wizardBack') }}
            </button>

            <button
              v-if="currentStep !== 'done' && currentStep !== 'test'"
              class="ui-button ui-button--primary wizard-btn wizard-btn-primary"
              :disabled="!canNext"
              @click="nextStep"
            >
              {{ t('providers.wizardNext') }}
              <ChevronRight :size="16" />
            </button>

            <button
              v-if="currentStep === 'test'"
              class="ui-button ui-button--primary wizard-btn wizard-btn-primary"
              :disabled="testResult !== 'success'"
              @click="nextStep"
            >
              {{ t('providers.wizardNext') }}
              <ChevronRight :size="16" />
            </button>

            <button
              v-if="currentStep === 'done'"
              class="ui-button ui-button--primary wizard-btn wizard-btn-primary"
              @click="complete"
            >
              {{ t('providers.wizardFinish') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.wizard-overlay {
  position: fixed;
  inset: 0;
  background: var(--overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.wizard-modal {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  width: 100%;
  max-width: 560px;
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
  border-bottom: 1px solid var(--border);
}

.wizard-title {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--foreground);
}

.wizard-close {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.wizard-progress {
  display: flex;
  gap: 1rem;
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid var(--border);
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
  background: var(--card);
  border: 2px solid var(--border);
  color: var(--muted-foreground);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.75rem;
  font-weight: 600;
  transition: all 0.15s ease;
}

.wizard-progress-item.active .wizard-progress-indicator {
  border-color: var(--primary);
  color: var(--primary);
}

.wizard-progress-item.completed .wizard-progress-indicator {
  background: var(--primary);
  border-color: var(--primary);
  color: var(--primary-foreground);
}

.wizard-progress-label {
  font-size: 0.625rem;
  color: var(--muted-foreground);
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
  gap: 0.75rem;
}

.wizard-label {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--muted-foreground);
}

.wizard-input-wrapper {
  position: relative;
}

.wizard-input,
.wizard-select {
  width: 100%;
  transition: all 0.15s ease;
}

.wizard-input-toggle {
  position: absolute;
  right: 0.75rem;
  top: 50%;
  transform: translateY(-50%);
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.wizard-hint {
  font-size: 0.75rem;
  color: var(--muted-foreground);
  margin-top: 0.25rem;
}

.wizard-test-area {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.wizard-test-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  transition: all 0.15s ease;
}

.wizard-test-result {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border-radius: var(--radius-md);
  font-size: 0.875rem;
  font-weight: 500;
}

.wizard-test-result.success {
  background: var(--success-soft);
  color: var(--success);
}

.wizard-test-result.failed {
  background: var(--destructive-soft);
  color: var(--destructive);
}

.wizard-test-latency {
  opacity: 0.8;
  font-size: 0.75rem;
}

.wizard-done {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 2rem 0;
}

.wizard-done-icon {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  background: var(--success);
  color: var(--success-foreground);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 1.5rem;
}

.wizard-done-title {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--foreground);
  margin-bottom: 0.5rem;
}

.wizard-done-desc {
  font-size: 0.875rem;
  color: var(--muted-foreground);
}

.wizard-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 1.25rem 1.5rem;
  border-top: 1px solid var(--border);
}

.wizard-btn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  transition: all 0.15s ease;
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

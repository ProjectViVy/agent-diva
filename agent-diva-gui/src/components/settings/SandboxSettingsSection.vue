<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { ShieldCheck, LoaderCircle, AlertTriangle, Plus, X } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { showAppToast } from '../../utils/appToast';
import { getSandboxConfig, saveSandboxConfig, type SandboxConfig } from '../../api/desktop';

const { t } = useI18n();

const SANDBOX_MODES: SandboxConfig['mode'][] = ['danger-full-access', 'read-only', 'workspace-write'];
const APPROVAL_POLICIES: SandboxConfig['approval_policy'][] = ['never', 'on-failure', 'on-request', 'unless-trusted'];

const loading = ref(true);
const loadError = ref<string | null>(null);
const saving = ref(false);
// Computed textarea model: join/split deny_patterns array on newlines
const denyPatternsText = computed({
  get: () => config.value.deny_patterns.join('\n'),
  set: (val: string) => {
    config.value.deny_patterns = val.split('\n').map(s => s.trim()).filter(Boolean);
  },
});

const config = ref<SandboxConfig>({
  mode: 'read-only',
  approval_policy: 'on-failure',
  network_access: true,
  writable_roots: [],
  protected_paths: [],
  deny_patterns: [],
  timeout_seconds: 30,
});
const originalSnapshot = ref('');
const originalMode = ref('');

const newWritableRoot = ref('');
const newProtectedPath = ref('');

const isDirty = computed(() => JSON.stringify(config.value) !== originalSnapshot.value);
const modeChanged = computed(() => config.value.mode !== originalMode.value);

const loadConfig = async () => {
  loading.value = true;
  loadError.value = null;
  try {
    const data = await getSandboxConfig();
    config.value = {
      mode: data.mode ?? 'read-only',
      approval_policy: data.approval_policy ?? 'on-failure',
      network_access: data.network_access ?? true,
      writable_roots: data.writable_roots ?? [],
      protected_paths: data.protected_paths ?? [],
      deny_patterns: data.deny_patterns ?? [],
      timeout_seconds: data.timeout_seconds ?? 30,
    };
    originalSnapshot.value = JSON.stringify(config.value);
    originalMode.value = config.value.mode;
  } catch {
    // Backend command may not exist yet; use defaults so UI is still usable
    config.value = {
      mode: 'read-only',
      approval_policy: 'on-failure',
      network_access: true,
      writable_roots: [],
      protected_paths: [],
      deny_patterns: [],
      timeout_seconds: 30,
    };
    originalSnapshot.value = JSON.stringify(config.value);
    originalMode.value = config.value.mode;
  } finally {
    loading.value = false;
  }
};

const saveConfig = async () => {
  if (saving.value || !isDirty.value) return;
  saving.value = true;
  try {
    await saveSandboxConfig({ ...config.value });
    originalSnapshot.value = JSON.stringify(config.value);
    originalMode.value = config.value.mode;
    showAppToast(t('sandbox.saved'), 'success');
  } catch (e) {
    showAppToast(t('sandbox.saveFailed'), 'error');
  } finally {
    saving.value = false;
  }
};

const addWritableRoot = () => {
  const val = newWritableRoot.value.trim();
  if (val && !config.value.writable_roots.includes(val)) {
    config.value.writable_roots.push(val);
    newWritableRoot.value = '';
  }
};

const removeWritableRoot = (idx: number) => {
  config.value.writable_roots.splice(idx, 1);
};

const addProtectedPath = () => {
  const val = newProtectedPath.value.trim();
  if (val && !config.value.protected_paths.includes(val)) {
    config.value.protected_paths.push(val);
    newProtectedPath.value = '';
  }
};

const removeProtectedPath = (idx: number) => {
  config.value.protected_paths.splice(idx, 1);
};

onMounted(loadConfig);
</script>

<template>
  <div class="p-6 space-y-6 fade-in">
    <!-- Header -->
    <div class="flex items-start justify-between gap-4">
      <div class="flex items-center space-x-3">
        <div class="settings-dashboard-icon">
          <ShieldCheck :size="20" />
        </div>
        <div>
          <h3 class="settings-dashboard-title">{{ t('sandbox.title') }}</h3>
          <p class="settings-dashboard-desc">{{ t('sandbox.desc') }}</p>
        </div>
      </div>
      <button
        type="button"
        class="settings-btn settings-btn-primary inline-flex min-w-[112px] items-center justify-center gap-2"
        :disabled="saving || !isDirty"
        @click="saveConfig"
      >
        <LoaderCircle v-if="saving" :size="16" class="animate-spin" />
        <span>{{ saving ? t('sandbox.saving') : t('sandbox.save') }}</span>
      </button>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center py-12">
      <LoaderCircle :size="24" class="animate-spin" :style="{ color: 'var(--accent)' }" />
      <span class="ml-3 settings-muted">{{ t('sandbox.loading') }}</span>
    </div>

    <!-- Error -->
    <div v-else-if="loadError" class="settings-section" :style="{ borderColor: 'var(--danger)' }">
      <p class="text-sm" :style="{ color: 'var(--danger)' }">{{ t('sandbox.loadError') }}: {{ loadError }}</p>
      <button type="button" class="settings-btn settings-btn-secondary mt-3" @click="loadConfig">
        {{ t('sandbox.retry') }}
      </button>
    </div>

    <!-- Content -->
    <template v-else>
      <!-- Mode Change Warning -->
      <div v-if="modeChanged" class="sandbox-warning-banner">
        <AlertTriangle :size="16" class="shrink-0" />
        <span>{{ t('sandbox.modeChangeWarning') }}</span>
      </div>

      <!-- Core Settings -->
      <div class="settings-section space-y-5">
        <div class="settings-section-header">
          <ShieldCheck :size="16" />
          <span>{{ t('sandbox.coreGroup') }}</span>
        </div>

        <!-- Sandbox Mode Dropdown -->
        <div class="space-y-1">
          <label class="block text-xs font-medium settings-muted uppercase tracking-wider">
            {{ t('sandbox.sandboxMode') }}
          </label>
          <select v-model="config.mode" class="settings-input">
            <option v-for="mode in SANDBOX_MODES" :key="mode" :value="mode">
              {{ t(`sandbox.modes.${mode}`) }}
            </option>
          </select>
          <p class="text-xs settings-muted mt-1">{{ t('sandbox.sandboxModeHint') }}</p>
        </div>

        <!-- Approval Policy Dropdown -->
        <div class="space-y-1">
          <label class="block text-xs font-medium settings-muted uppercase tracking-wider">
            {{ t('sandbox.approvalPolicy') }}
          </label>
          <select v-model="config.approval_policy" class="settings-input">
            <option v-for="policy in APPROVAL_POLICIES" :key="policy" :value="policy">
              {{ t(`sandbox.policies.${policy}`) }}
            </option>
          </select>
        </div>

        <!-- Network Toggle -->
        <label class="settings-label flex items-center justify-between cursor-pointer">
          <span>{{ t('sandbox.networkEnabled') }}</span>
          <button
            type="button"
            role="switch"
            :aria-checked="config.network_access"
            class="sandbox-toggle"
            :class="{ active: config.network_access }"
            @click="config.network_access = !config.network_access"
          >
            <span class="sandbox-toggle-thumb" />
          </button>
        </label>

        <!-- Timeout -->
        <div class="space-y-1">
          <label class="block text-xs font-medium settings-muted uppercase tracking-wider">
            {{ t('sandbox.timeout') }}
          </label>
          <input
            v-model.number="config.timeout_seconds"
            type="number"
            min="1"
            max="600"
            class="settings-input"
          />
        </div>
      </div>

      <!-- Writable Roots -->
      <div class="settings-section space-y-3">
        <div class="settings-section-header">
          <span>{{ t('sandbox.writableRoots') }}</span>
        </div>
        <p class="text-xs settings-muted">{{ t('sandbox.writableRootsHint') }}</p>

        <div class="sandbox-tag-list">
          <span
            v-for="(root, idx) in config.writable_roots"
            :key="'w-' + idx"
            class="sandbox-tag"
          >
            <span class="sandbox-tag-text">{{ root }}</span>
            <button type="button" class="sandbox-tag-remove" @click="removeWritableRoot(idx)">
              <X :size="12" />
            </button>
          </span>
        </div>

        <div class="flex gap-2">
          <input
            v-model="newWritableRoot"
            type="text"
            class="settings-input flex-1"
            :placeholder="t('sandbox.addPathPlaceholder')"
            @keydown.enter.prevent="addWritableRoot"
          />
          <button type="button" class="settings-btn settings-btn-secondary" @click="addWritableRoot">
            <Plus :size="14" />
          </button>
        </div>
      </div>

      <!-- Protected Paths -->
      <div class="settings-section space-y-3">
        <div class="settings-section-header">
          <span>{{ t('sandbox.protectedPaths') }}</span>
        </div>
        <p class="text-xs settings-muted">{{ t('sandbox.protectedPathsHint') }}</p>

        <div class="sandbox-tag-list">
          <span
            v-for="(path, idx) in config.protected_paths"
            :key="'p-' + idx"
            class="sandbox-tag sandbox-tag-protected"
          >
            <span class="sandbox-tag-text">{{ path }}</span>
            <button type="button" class="sandbox-tag-remove" @click="removeProtectedPath(idx)">
              <X :size="12" />
            </button>
          </span>
        </div>

        <div class="flex gap-2">
          <input
            v-model="newProtectedPath"
            type="text"
            class="settings-input flex-1"
            :placeholder="t('sandbox.addPathPlaceholder')"
            @keydown.enter.prevent="addProtectedPath"
          />
          <button type="button" class="settings-btn settings-btn-secondary" @click="addProtectedPath">
            <Plus :size="14" />
          </button>
        </div>
      </div>

      <!-- Deny Patterns -->
      <div class="settings-section space-y-3">
        <div class="settings-section-header">
          <span>{{ t('sandbox.denyPatterns') }}</span>
        </div>
        <p class="text-xs settings-muted">{{ t('sandbox.denyPatternsHint') }}</p>
        <textarea
          v-model="denyPatternsText"
          class="settings-input sandbox-textarea"
          rows="4"
          :placeholder="t('sandbox.denyPatternsPlaceholder')"
        />
      </div>
    </template>
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

.sandbox-warning-banner {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border-radius: var(--radius-sm);
  background: rgba(245, 158, 11, 0.12);
  border: 1px solid rgba(245, 158, 11, 0.3);
  color: var(--warning);
  font-size: 0.875rem;
  font-weight: 500;
}

.sandbox-toggle {
  position: relative;
  width: 44px;
  height: 24px;
  border-radius: 12px;
  border: none;
  cursor: pointer;
  transition: background-color 0.2s ease;
  background: var(--line);
  flex-shrink: 0;
}

.sandbox-toggle.active {
  background: var(--accent);
}

.sandbox-toggle-thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: white;
  transition: transform 0.2s ease;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}

.sandbox-toggle.active .sandbox-toggle-thumb {
  transform: translateX(20px);
}

.sandbox-tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.375rem;
}

.sandbox-tag {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-sm);
  background: var(--accent-bg-light);
  border: 1px solid var(--accent-border);
  font-size: 0.8rem;
  color: var(--text);
}

.sandbox-tag-protected {
  background: var(--danger-bg);
  border-color: rgba(239, 68, 68, 0.3);
}

.sandbox-tag-text {
  font-family: monospace;
  word-break: break-all;
}

.sandbox-tag-remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.15s ease;
  padding: 0;
}

.sandbox-tag-remove:hover {
  background: var(--danger-bg);
  color: var(--danger);
}

.sandbox-textarea {
  resize: vertical;
  min-height: 80px;
  font-family: monospace;
  font-size: 0.8rem;
  line-height: 1.5;
}
</style>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { ShieldCheck, LoaderCircle, AlertTriangle, Plus, X } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import { showAppToast } from '../../utils/appToast';
import { loadSandboxSettings, saveSandboxSettings } from '../../api/settings';
import type { VivySandboxResult } from '../../api/vivy/contracts';

const { t } = useI18n();

const PRESETS = ['cautious', 'smart', 'trusted'] as const;

const loading = ref(true);
const loadError = ref<string | null>(null);
const saving = ref(false);
const effective = ref<VivySandboxResult | null>(null);

const preset = ref<string>('smart');
const denyPrivateIps = ref(true);
const allowedDomains = ref<string[]>([]);
const approvalTimeoutSeconds = ref(0);
const newDomain = ref('');

const originalSnapshot = ref('');

const formSnapshot = () => JSON.stringify({
  preset: preset.value,
  denyPrivateIps: denyPrivateIps.value,
  allowedDomains: allowedDomains.value,
  approvalTimeoutSeconds: approvalTimeoutSeconds.value,
});
const isDirty = computed(() => formSnapshot() !== originalSnapshot.value);

const applyView = (view: VivySandboxResult) => {
  effective.value = view;
  preset.value = view.default_preset;
  denyPrivateIps.value = view.deny_private_ips;
  allowedDomains.value = [...view.allowed_domains];
  approvalTimeoutSeconds.value = view.approval_timeout_seconds;
  originalSnapshot.value = formSnapshot();
};

const load = async () => {
  loading.value = true;
  loadError.value = null;
  try {
    applyView(await loadSandboxSettings());
  } catch (error) {
    loadError.value = String(error);
  } finally {
    loading.value = false;
  }
};

const save = async () => {
  if (saving.value || !isDirty.value) return;
  saving.value = true;
  const timeout = Number(approvalTimeoutSeconds.value);
  const clamped = Number.isFinite(timeout) && timeout >= 0 ? Math.floor(timeout) : 0;
  try {
    applyView(await saveSandboxSettings({
      default_preset: preset.value,
      deny_private_ips: denyPrivateIps.value,
      allowed_domains: allowedDomains.value,
      approval_timeout_seconds: clamped,
    }));
    showAppToast(t('sandbox.saved'), 'success');
  } catch (error) {
    showAppToast(`${t('sandbox.saveFailed')}: ${error instanceof Error ? error.message : String(error)}`, 'error');
  } finally {
    saving.value = false;
  }
};

const addDomain = () => {
  const val = newDomain.value.trim();
  if (val && !allowedDomains.value.includes(val)) {
    allowedDomains.value.push(val);
    newDomain.value = '';
  }
};

const removeDomain = (idx: number) => {
  allowedDomains.value.splice(idx, 1);
};

onMounted(load);
</script>

<template>
  <div class="sandbox-settings space-y-6">
    <div class="flex items-center gap-2">
      <ShieldCheck :size="18" class="text-success" />
      <h3 class="text-base font-semibold">{{ t('sandbox.title') }}</h3>
    </div>

    <div v-if="loading" class="flex items-center gap-2 settings-muted text-sm">
      <LoaderCircle :size="14" class="animate-spin" />
      {{ t('sandbox.loading') }}
    </div>
    <div v-else-if="loadError" class="flex items-center gap-2 text-sm" :style="{ color: 'var(--destructive)' }">
      <AlertTriangle :size="14" />
      {{ loadError }}
    </div>

    <template v-else>
      <!-- Permission preset -->
      <div class="settings-section space-y-3">
        <div class="settings-section-header">
          <span>{{ t('sandbox.preset') }}</span>
        </div>
        <p class="text-xs settings-muted">{{ t('sandbox.presetHint') }}</p>
        <div class="flex flex-wrap gap-2">
          <button
            v-for="p in PRESETS"
            :key="p"
            type="button"
            class="ui-button ui-button--ghost sandbox-preset-btn"
            :class="{ active: preset === p }"
            @click="preset = p"
          >
            {{ t(`sandbox.presets.${p}`) }}
          </button>
        </div>
        <p v-if="preset === 'custom'" class="text-xs settings-muted">
          {{ t('sandbox.customReadonly') }}
        </p>
        <p class="text-xs settings-muted">
          {{ t('sandbox.configDefault') }}: {{ effective?.config_default_preset }}
        </p>
      </div>

      <!-- Network -->
      <div class="settings-section space-y-3">
        <div class="settings-section-header">
          <span>{{ t('sandbox.network') }}</span>
        </div>
        <label class="flex items-center gap-2 text-sm">
          <button
            type="button"
            role="switch"
            :aria-checked="denyPrivateIps"
            class="sandbox-toggle"
            :class="{ active: denyPrivateIps }"
            @click="denyPrivateIps = !denyPrivateIps"
          >
            <span class="sandbox-toggle-thumb" />
          </button>
          {{ t('sandbox.denyPrivateIps') }}
        </label>
        <p class="text-xs settings-muted">{{ t('sandbox.denyPrivateIpsHint') }}</p>

        <div class="settings-section-header">
          <span>{{ t('sandbox.allowedDomains') }}</span>
        </div>
        <p class="text-xs settings-muted">{{ t('sandbox.allowedDomainsHint') }}</p>
        <div class="sandbox-tag-list">
          <span
            v-for="(domain, idx) in allowedDomains"
            :key="'d-' + idx"
            class="sandbox-tag"
          >
            <span class="sandbox-tag-text">{{ domain }}</span>
            <button type="button" class="ui-button ui-button--ghost ui-button--compact sandbox-tag-remove" @click="removeDomain(idx)">
              <X :size="12" />
            </button>
          </span>
        </div>
        <div class="flex gap-2">
          <input
            v-model="newDomain"
            type="text"
            class="ui-input settings-input flex-1"
            :placeholder="t('sandbox.addDomainPlaceholder')"
            @keydown.enter.prevent="addDomain"
          />
          <button type="button" class="ui-button ui-button--outline settings-btn settings-btn-secondary" @click="addDomain">
            <Plus :size="14" />
          </button>
        </div>
      </div>

      <!-- Approval timeout -->
      <div class="settings-section space-y-3">
        <div class="settings-section-header">
          <span>{{ t('sandbox.approvalTimeout') }}</span>
        </div>
        <p class="text-xs settings-muted">{{ t('sandbox.approvalTimeoutHint') }}</p>
        <input
          v-model.number="approvalTimeoutSeconds"
          type="number"
          min="0"
          class="ui-input settings-input w-32"
        />
        <p class="text-xs settings-muted">
          {{ t('sandbox.approvalTimeoutMeta', {
            config: effective?.config_approval_timeout_seconds,
            ceiling: effective?.approval_expiration_seconds || '—',
          }) }}
        </p>
      </div>

      <!-- Read-only runtime facts -->
      <div class="settings-section space-y-3">
        <div class="settings-section-header">
          <span>{{ t('sandbox.runtimeFacts') }}</span>
        </div>
        <p class="text-xs settings-muted">{{ t('sandbox.runtimeFactsHint') }}</p>
        <dl class="sandbox-facts">
          <div class="sandbox-fact">
            <dt>{{ t('sandbox.workspaceRoot') }}</dt>
            <dd><code>{{ effective?.workspace_root || '—' }}</code></dd>
          </div>
          <div class="sandbox-fact">
            <dt>{{ t('sandbox.allowedCommands') }}</dt>
            <dd>
              <code v-if="effective?.execute_allowed_commands?.length">
                {{ effective.execute_allowed_commands.join(', ') }}
              </code>
              <span v-else>—</span>
            </dd>
          </div>
        </dl>
      </div>

      <!-- Save -->
      <div class="flex items-center gap-3">
        <button
          type="button"
          class="ui-button ui-button--ghost settings-btn"
          :disabled="saving || !isDirty"
          @click="save"
        >
          <LoaderCircle v-if="saving" :size="14" class="animate-spin" />
          {{ t('sandbox.save') }}
        </button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.sandbox-preset-btn {
  transition: all 0.2s;
}

.sandbox-preset-btn.active {
  color: var(--primary);
  border-color: var(--primary);
}

.sandbox-tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.375rem;
}

.sandbox-tag {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.125rem 0.5rem;
  font-size: 0.75rem;
  border-radius: 9999px;
  background: var(--accent);
  color: var(--foreground);
}

.sandbox-tag-remove {
  display: inline-flex;
  align-items: center;
}

.sandbox-toggle {
  position: relative;
  width: 2rem;
  height: 1.125rem;
  border-radius: 9999px;
  background: var(--border);
  transition: background 0.2s;
  flex-shrink: 0;
}

.sandbox-toggle.active {
  background: var(--primary);
}

.sandbox-toggle-thumb {
  position: absolute;
  top: 0.125rem;
  left: 0.125rem;
  width: 0.875rem;
  height: 0.875rem;
  border-radius: 9999px;
  background: var(--card);
  transition: transform 0.2s;
}

.sandbox-toggle.active .sandbox-toggle-thumb {
  transform: translateX(0.875rem);
}

.sandbox-facts {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  font-size: 0.8125rem;
}

.sandbox-fact {
  display: grid;
  grid-template-columns: 12rem 1fr;
  gap: 0.5rem;
}

.sandbox-fact dt {
  color: var(--muted-foreground);
}
</style>

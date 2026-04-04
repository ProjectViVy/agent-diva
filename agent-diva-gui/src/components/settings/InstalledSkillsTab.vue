<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { RefreshCcw, Trash2, Upload, ShieldCheck, CircleOff, Search } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

import { deleteSkill, getSkills, isTauriRuntime, uploadSkill, type SkillDto } from '../../api/desktop';

const { t } = useI18n();

const skills = ref<SkillDto[]>([]);
const loading = ref(false);
const uploading = ref(false);
const deletingName = ref('');
const error = ref('');
const searchQuery = ref('');
const previewMode = computed(() => !isTauriRuntime());

const filteredSkills = computed(() => {
  const q = searchQuery.value.toLowerCase().trim();
  if (!q) return skills.value;
  return skills.value.filter(
    (s) =>
      s.name.toLowerCase().includes(q) ||
      s.description.toLowerCase().includes(q)
  );
});

async function refreshSkills() {
  if (previewMode.value) {
    skills.value = [];
    error.value = '';
    return;
  }

  loading.value = true;
  error.value = '';
  try {
    skills.value = await getSkills();
  } catch (err) {
    error.value = String(err);
  } finally {
    loading.value = false;
  }
}

async function onUploadChange(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) {
    return;
  }

  if (!file.name.toLowerCase().endsWith('.zip')) {
    error.value = t('general.skillsZipOnly');
    input.value = '';
    return;
  }

  uploading.value = true;
  error.value = '';
  try {
    const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
    await uploadSkill(file.name, bytes);
    await refreshSkills();
  } catch (err) {
    error.value = String(err);
  } finally {
    uploading.value = false;
    input.value = '';
  }
}

async function removeSkill(name: string) {
  if (previewMode.value) {
    return;
  }
  deletingName.value = name;
  error.value = '';
  try {
    await deleteSkill(name);
    await refreshSkills();
  } catch (err) {
    error.value = String(err);
  } finally {
    deletingName.value = '';
  }
}

const statusClass = (skill: SkillDto) => {
  if (skill.active) {
    return 'skills-status-badge active';
  }
  if (skill.available) {
    return 'skills-status-badge available';
  }
  return 'skills-status-badge unavailable';
};

const statusLabel = (skill: SkillDto) => {
  if (skill.active) return t('general.skillStatusActive');
  if (skill.available) return t('general.skillStatusAvailable');
  return t('general.skillStatusUnavailable');
};

onMounted(refreshSkills);

defineExpose({ refreshSkills });
</script>

<template>
  <div class="space-y-4">
    <!-- Search and Actions Bar -->
    <div class="flex flex-wrap items-center gap-3">
      <div class="relative flex-1 min-w-[200px]">
        <Search :size="14" class="absolute left-3 top-2.5" style="color: var(--text-muted);" />
        <input
          v-model="searchQuery"
          type="text"
          class="skills-search-input"
          :placeholder="t('general.searchInstalled')"
        />
      </div>

      <button
        class="skills-btn"
        :disabled="loading || uploading"
        @click="refreshSkills"
      >
        <RefreshCcw :size="14" />
        {{ t('general.refreshSkills') }}
      </button>

      <label
        class="skills-btn skills-btn-primary"
        :class="{ 'pointer-events-none opacity-60': previewMode || uploading }"
      >
        <Upload :size="14" />
        {{ uploading ? t('general.uploadingSkill') : t('general.uploadSkill') }}
        <input
          class="hidden"
          type="file"
          accept=".zip,application/zip"
          :disabled="previewMode || uploading"
          @change="onUploadChange"
        />
      </label>
    </div>

    <!-- Hint Box -->
    <div class="skills-hint-box">
      <p>{{ t('general.skillsZipHint') }}</p>
      <p v-if="previewMode" class="mt-2 skills-hint-box warning">{{ t('general.skillsPreviewOnly') }}</p>
    </div>

    <!-- Error Display -->
    <p v-if="error" class="text-xs" style="color: var(--danger); break-words;">{{ error }}</p>

    <!-- Loading State -->
    <div v-if="loading" class="text-sm" style="color: var(--text-muted);">{{ t('general.loadingSkills') }}</div>

    <!-- Empty State -->
    <div v-else-if="filteredSkills.length === 0" class="text-sm" style="color: var(--text-muted);">
      {{ searchQuery ? t('general.noSearchResults') : t('general.emptySkills') }}
    </div>

    <!-- Skills List -->
    <div v-else class="space-y-3">
      <div
        v-for="skill in filteredSkills"
        :key="`${skill.source}-${skill.name}`"
        class="skills-list-item"
      >
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div class="min-w-0 space-y-2">
            <div class="flex flex-wrap items-center gap-2">
              <div class="skills-item-name">{{ skill.name }}</div>
              <span
                class="skills-status-badge"
                :class="statusClass(skill)"
              >
                {{ statusLabel(skill) }}
              </span>
              <span
                class="skills-source-badge"
                :class="{ builtin: skill.source === 'builtin' }"
              >
                {{ skill.source === 'builtin' ? t('general.skillSourceBuiltin') : t('general.skillSourceWorkspace') }}
              </span>
            </div>
            <p class="skills-item-desc">{{ skill.description }}</p>
            <p class="skills-item-path">{{ skill.path }}</p>
          </div>

          <button
            class="skills-btn"
            :disabled="!skill.can_delete || deletingName === skill.name || previewMode"
            @click="removeSkill(skill.name)"
          >
            <Trash2 v-if="skill.can_delete" :size="14" />
            <ShieldCheck v-else :size="14" />
            {{ skill.can_delete ? t('general.deleteSkill') : t('general.builtinSkillLocked') }}
          </button>
        </div>
        <div v-if="!skill.available" class="mt-3 flex items-center gap-2 text-xs" style="color: var(--warning);">
          <CircleOff :size="14" />
          <span>{{ t('general.skillUnavailableHint') }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

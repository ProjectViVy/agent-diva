<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Boxes, RefreshCcw, Trash2, Upload, ShieldCheck, CircleOff } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

import { deleteSkill, getSkills, isTauriRuntime, uploadSkill, type SkillDto } from '../../api/desktop';

const { t } = useI18n();

const skills = ref<SkillDto[]>([]);
const loading = ref(false);
const uploading = ref(false);
const deletingName = ref('');
const error = ref('');
const previewMode = computed(() => !isTauriRuntime());

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
    return 'bg-emerald-100 text-emerald-700';
  }
  if (skill.available) {
    return 'bg-sky-100 text-sky-700';
  }
  return 'bg-amber-100 text-amber-700';
};

const statusLabel = (skill: SkillDto) => {
  if (skill.active) return t('general.skillStatusActive');
  if (skill.available) return t('general.skillStatusAvailable');
  return t('general.skillStatusUnavailable');
};

onMounted(refreshSkills);
</script>

<template>
  <section class="bg-white border border-gray-100 rounded-xl p-4 space-y-4">
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div class="flex items-center gap-2 text-gray-700">
        <Boxes :size="16" class="text-cyan-600" />
        <div>
          <div class="text-sm font-semibold">{{ t('general.skillsTitle') }}</div>
          <p class="text-xs text-gray-500">{{ t('general.skillsDesc') }}</p>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <button
          class="inline-flex items-center gap-2 px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-600 hover:bg-gray-50 disabled:opacity-60"
          :disabled="loading || uploading"
          @click="refreshSkills"
        >
          <RefreshCcw :size="14" />
          {{ t('general.refreshSkills') }}
        </button>

        <label
          class="inline-flex items-center gap-2 px-3 py-2 text-xs rounded-lg bg-cyan-600 text-white hover:bg-cyan-700 disabled:opacity-60 cursor-pointer"
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
    </div>

    <div class="rounded-xl border border-dashed border-gray-200 bg-gray-50/80 p-3 text-xs text-gray-500">
      <p>{{ t('general.skillsZipHint') }}</p>
      <p v-if="previewMode" class="mt-2 text-amber-600">{{ t('general.skillsPreviewOnly') }}</p>
    </div>

    <div v-if="loading" class="text-sm text-gray-500">{{ t('general.loadingSkills') }}</div>
    <div v-else-if="skills.length === 0" class="text-sm text-gray-500">
      {{ t('general.emptySkills') }}
    </div>
    <div v-else class="space-y-3">
      <div
        v-for="skill in skills"
        :key="`${skill.source}-${skill.name}`"
        class="rounded-xl border border-gray-100 bg-gray-50 px-4 py-3"
      >
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div class="min-w-0 space-y-2">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-gray-800">{{ skill.name }}</div>
              <span class="px-2 py-0.5 rounded-full text-[10px] font-semibold uppercase tracking-wide" :class="statusClass(skill)">
                {{ statusLabel(skill) }}
              </span>
              <span class="px-2 py-0.5 rounded-full text-[10px] font-semibold uppercase tracking-wide" :class="skill.source === 'builtin' ? 'bg-violet-100 text-violet-700' : 'bg-slate-200 text-slate-700'">
                {{ skill.source === 'builtin' ? t('general.skillSourceBuiltin') : t('general.skillSourceWorkspace') }}
              </span>
            </div>
            <p class="text-xs text-gray-500 break-words">{{ skill.description }}</p>
            <p class="text-[11px] text-gray-400 break-all">{{ skill.path }}</p>
          </div>

          <button
            class="inline-flex items-center gap-2 px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-700 hover:bg-white disabled:opacity-60"
            :disabled="!skill.can_delete || deletingName === skill.name || previewMode"
            @click="removeSkill(skill.name)"
          >
            <Trash2 v-if="skill.can_delete" :size="14" />
            <ShieldCheck v-else :size="14" />
            {{ skill.can_delete ? t('general.deleteSkill') : t('general.builtinSkillLocked') }}
          </button>
        </div>
        <div v-if="!skill.available" class="mt-3 flex items-center gap-2 text-xs text-amber-700">
          <CircleOff :size="14" />
          <span>{{ t('general.skillUnavailableHint') }}</span>
        </div>
      </div>
    </div>

    <p v-if="error" class="text-xs text-red-600 break-words">{{ error }}</p>
  </section>
</template>

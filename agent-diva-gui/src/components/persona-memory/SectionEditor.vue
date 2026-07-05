<script setup lang="ts">
import { watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { LaputaSection, LaputaSectionName } from '../../api/desktop';

const { t } = useI18n();

interface Props {
  sectionName: LaputaSectionName;
  section: LaputaSection | null;
  loading: boolean;
  error: string;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'update:dirty', value: boolean): void;
  (e: 'refresh'): void;
}>();

watch(() => props.sectionName, () => {
  emit('update:dirty', false);
});
</script>

<template>
  <div class="section-editor">
    <div v-if="loading" class="section-editor-loading">{{ t('laputa.loading') }}</div>
    <div v-else-if="error" class="section-editor-placeholder">
      {{ error }}
    </div>
    <div v-else class="section-editor-placeholder">
      <strong>{{ sectionName }}</strong>
      <span>{{ t('laputa.emptyTitle') }}</span>
      <span>{{ t('laputa.emptyDesc') }}</span>
    </div>
  </div>
</template>

<style scoped>
.section-editor {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 20px;
}

.section-editor-loading,
.section-editor-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 8px;
  color: var(--text-muted);
  font-size: 14px;
  text-align: center;
}

.section-editor-placeholder strong {
  color: var(--text);
  font-size: 16px;
}
</style>

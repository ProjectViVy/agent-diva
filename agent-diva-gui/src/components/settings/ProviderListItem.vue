<script setup lang="ts">
import { Server, LoaderCircle, Trash2 } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

interface ProviderSpec {
  name: string;
  api_type: string;
  source?: string;
  display_name: string;
  default_model?: string | null;
  default_api_base: string;
  models: string[];
  custom_models: string[];
}

interface ProviderStatusItem {
  name: string;
  ready: boolean;
  current: boolean;
  model?: string | null;
  default_model?: string | null;
  missing_fields: string[];
}

const props = defineProps<{
  provider: ProviderSpec;
  selectedProvider: ProviderSpec | null;
  status?: ProviderStatusItem;
  isDeleting?: boolean;
}>();

const emit = defineEmits<{
  (e: 'select'): void;
  (e: 'delete'): void;
}>();

const isSelected = () => props.selectedProvider?.name === props.provider.name;
</script>

<template>
  <div
    class="providers-list-item"
    :class="{ selected: isSelected() }"
    @click="emit('select')"
  >
    <button
      type="button"
      class="flex min-w-0 flex-1 items-center px-4 py-3 text-left"
    >
      <div class="flex min-w-0 flex-1 items-center">
        <div
          class="providers-item-icon"
          :class="{ selected: isSelected() }"
        >
          <Server :size="16" />
        </div>
        <div class="min-w-0">
          <div class="font-medium flex items-center gap-2">
            <span class="truncate">{{ provider.display_name }}</span>
          </div>
          <div class="text-[10px] uppercase tracking-wider opacity-70 flex flex-wrap items-center gap-1 providers-tag api-type">
            <span>{{ provider.api_type || t('providers.standardApi') }}</span>
            <span v-if="status?.current" class="providers-tag current">{{ t('providers.currentTag') }}</span>
          </div>
        </div>
      </div>
    </button>
    <div
      v-if="provider.source === 'custom'"
      class="relative z-10 flex shrink-0 items-center pr-2"
    >
      <button
        type="button"
        class="providers-delete-btn"
        :title="t('providers.deleteProvider')"
        :disabled="isDeleting"
        @click.stop="emit('delete')"
      >
        <LoaderCircle
          v-if="isDeleting"
          :size="14"
          class="animate-spin"
        />
        <Trash2 v-else :size="14" />
      </button>
    </div>
  </div>
</template>

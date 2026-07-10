<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useMasks } from '../composables/useMasks';
import MaskCard from './MaskCard.vue';
import type { MaskEntryDto } from '../api/desktop';
import { Search, Plus, Pencil, Trash2, Settings } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

withDefaults(defineProps<{
  mode?: 'selector' | 'manager';
}>(), {
  mode: 'selector',
});

const emit = defineEmits<{
  (e: 'select-mask', name: string): void;
  (e: 'navigate-manage'): void;
  (e: 'edit', mask: MaskEntryDto): void;
  (e: 'create'): void;
}>();

const { masks, activeMask, loading, error, refresh, switchTo, remove } = useMasks();

// Manager mode state
const searchQuery = ref('');
const selectedMaskName = ref<string | null>(null);
const deleteConfirmName = ref<string | null>(null);

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

onMounted(() => {
  refresh();
});

// ---------------------------------------------------------------------------
// Computed
// ---------------------------------------------------------------------------

const filteredMasks = computed(() => {
  if (!searchQuery.value.trim()) return masks.value;
  const q = searchQuery.value.trim().toLowerCase();
  return masks.value.filter((m) => m.name.toLowerCase().includes(q));
});

const selectedMask = computed<MaskEntryDto | null>(() => {
  if (!selectedMaskName.value) return null;
  return masks.value.find((m) => m.name === selectedMaskName.value) ?? null;
});

const isEmpty = computed(() => filteredMasks.value.length === 0);

// ---------------------------------------------------------------------------
// Handlers: Selector mode
// ---------------------------------------------------------------------------

async function handleCardSelect(mask: MaskEntryDto) {
  await switchTo(mask.name);
  emit('select-mask', mask.name);
}

function handleManageClick() {
  emit('navigate-manage');
}

// ---------------------------------------------------------------------------
// Handlers: Manager mode
// ---------------------------------------------------------------------------

function handleCardClick(mask: MaskEntryDto) {
  selectedMaskName.value = mask.name;
}

function handleEditClick(mask: MaskEntryDto) {
  emit('edit', mask);
}

function handleDeleteClick(mask: MaskEntryDto) {
  deleteConfirmName.value = mask.name;
}

async function confirmDelete() {
  if (!deleteConfirmName.value) return;
  await remove(deleteConfirmName.value);
  if (selectedMaskName.value === deleteConfirmName.value) {
    selectedMaskName.value = null;
  }
  deleteConfirmName.value = null;
}

function cancelDelete() {
  deleteConfirmName.value = null;
}

function handleCreateClick() {
  emit('create');
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function isActive(mask: MaskEntryDto): boolean {
  return activeMask.value?.name === mask.name;
}

function modeLabel(mode: string): string {
  if (mode === 'assist') return t('mask.modeAssist');
  return t('mask.modeNormal');
}

function modeColor(mode: string): string {
  if (mode === 'assist') return 'bg-purple-100 text-purple-600';
  return 'bg-gray-100 text-gray-500';
}
</script>

<template>
  <!-- ================================================================== -->
  <!-- SELECTOR MODE                                                        -->
  <!-- ================================================================== -->
  <div v-if="mode === 'selector'" class="mask-selector-panel">
    <!-- Header -->
    <div class="flex items-center justify-between px-1 pb-2">
      <span class="text-xs font-semibold text-gray-500 uppercase tracking-wide">
        🎭 {{ t('mask.title') }}
      </span>
    </div>

    <!-- Loading indicator -->
    <div v-if="loading" class="flex items-center justify-center py-4">
      <svg class="animate-spin h-5 w-5 text-pink-500" viewBox="0 0 24 24">
        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none" />
        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
      </svg>
    </div>

    <!-- Error state -->
    <div v-else-if="error" class="text-xs text-red-500 px-1 py-2">
      {{ error }}
    </div>

    <!-- Mask grid -->
    <div v-else class="grid grid-cols-3 gap-2">
      <MaskCard
        v-for="mask in masks"
        :key="mask.name"
        :mask="mask"
        :active="isActive(mask)"
        @select="handleCardSelect"
      />
    </div>

    <!-- Manage link -->
    <div class="mt-2 pt-2 border-t border-gray-100">
      <button
        type="button"
        class="text-xs text-gray-400 hover:text-pink-500 transition-colors flex items-center gap-1"
        @click="handleManageClick"
      >
        <Settings :size="12" />
        {{ t('mask.manageMasks') }}
      </button>
    </div>
  </div>

  <!-- ================================================================== -->
  <!-- MANAGER MODE                                                        -->
  <!-- ================================================================== -->
  <div v-else class="mask-manager-panel">
    <!-- Header -->
    <div class="flex items-center justify-between mb-3">
      <h3 class="text-base font-semibold text-gray-800">🎭 {{ t('mask.maskSystem') }}</h3>
      <button
        type="button"
        class="inline-flex items-center gap-1 px-3 py-1.5 text-sm font-medium text-white bg-pink-500 hover:bg-pink-600 rounded-lg transition-colors"
        @click="handleCreateClick"
      >
        <Plus :size="14" />
        {{ t('mask.createButton') }}
      </button>
    </div>

    <!-- Search input -->
    <div class="relative mb-3">
      <Search
        :size="14"
        class="absolute left-2.5 top-1/2 -translate-y-1/2 text-gray-400"
      />
      <input
        v-model="searchQuery"
        type="text"
        :placeholder="t('mask.search')"
        class="w-full pl-8 pr-3 py-1.5 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-pink-300 focus:border-pink-400 transition-shadow"
      />
    </div>

    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center py-8">
      <svg class="animate-spin h-6 w-6 text-pink-500" viewBox="0 0 24 24">
        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none" />
        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
      </svg>
    </div>

    <!-- Error -->
    <div v-else-if="error" class="text-sm text-red-500 py-2 px-3 bg-red-50 rounded-lg">
      {{ error }}
    </div>

    <!-- Empty state -->
    <div
      v-else-if="isEmpty"
      class="flex flex-col items-center justify-center py-8 text-gray-400"
    >
      <span class="text-4xl mb-2">🎭</span>
      <p class="text-sm font-medium">{{ t('mask.noMasksFound') }}</p>
      <p
        v-if="searchQuery.trim()"
        class="text-xs mt-1"
      >
        {{ t('mask.noSearchResults') }}
      </p>
      <button
        v-else
        type="button"
        class="mt-3 px-4 py-1.5 text-sm font-medium text-pink-600 border border-pink-300 rounded-lg hover:bg-pink-50 transition-colors"
        @click="handleCreateClick"
      >
        {{ t('mask.createFirst') }}
      </button>
    </div>

    <!-- Mask grid (manager mode) -->
    <div v-else class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3">
      <div
        v-for="mask in filteredMasks"
        :key="mask.name"
        class="relative flex flex-col items-center rounded-lg border p-3 text-left transition-all cursor-pointer"
        :class="[
          selectedMaskName === mask.name
            ? 'ring-2 ring-pink-500 border-pink-500 bg-pink-50'
            : 'border-gray-200 bg-white hover:shadow-md hover:border-gray-300',
        ]"
        @click="handleCardClick(mask)"
      >
        <!-- Active checkmark -->
        <div
          v-if="isActive(mask)"
          class="absolute top-1.5 right-1.5 w-5 h-5 bg-pink-500 text-white rounded-full flex items-center justify-center"
        >
          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
          </svg>
        </div>

        <!-- Emoji -->
        <span class="text-3xl leading-none mb-1.5 mt-1">{{ mask.icon }}</span>

        <!-- Name -->
        <span class="text-sm font-semibold text-gray-800 truncate w-full text-center">{{ mask.name }}</span>

        <!-- Badges -->
        <div class="flex items-center justify-center gap-1 mt-1.5">
          <span
            class="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-medium"
            :class="modeColor(mask.mode)"
          >
            {{ modeLabel(mask.mode) }}
          </span>
          <span
            v-if="mask.readOnly"
            class="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-medium bg-amber-100 text-amber-700"
          >
            {{ t('mask.readOnly') }}
          </span>
        </div>

        <!-- Action buttons -->
        <div class="flex items-center justify-center gap-2 mt-2 pt-2 border-t border-gray-100 w-full">
          <button
            type="button"
            class="inline-flex items-center gap-1 px-2 py-1 text-xs text-gray-500 hover:text-pink-600 hover:bg-pink-50 rounded transition-colors"
            @click.stop="handleEditClick(mask)"
          >
            <Pencil :size="12" />
            {{ t('mask.edit') }}
          </button>
          <button
            type="button"
            class="inline-flex items-center gap-1 px-2 py-1 text-xs text-gray-500 hover:text-red-600 hover:bg-red-50 rounded transition-colors"
            @click.stop="handleDeleteClick(mask)"
          >
            <Trash2 :size="12" />
            {{ t('mask.delete') }}
          </button>
        </div>
      </div>
    </div>

    <!-- Details pane (selected mask) -->
    <div
      v-if="selectedMask && !loading"
      class="mt-4 border border-gray-200 rounded-lg bg-gray-50 p-4"
    >
      <div class="flex items-start gap-3">
        <span class="text-3xl">{{ selectedMask.icon }}</span>
        <div class="flex-1 min-w-0">
          <h4 class="text-base font-semibold text-gray-800">{{ selectedMask.name }}</h4>
          <p class="text-sm text-gray-500 mt-0.5">{{ selectedMask.description || t('mask.noDescription') }}</p>
        </div>
        <span
          class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium"
          :class="modeColor(selectedMask.mode)"
        >
          {{ modeLabel(selectedMask.mode) }}
        </span>
      </div>

      <div class="mt-3 grid grid-cols-2 gap-x-6 gap-y-2 text-sm">
        <div>
          <span class="text-gray-400">{{ t('mask.mode.label') }}:</span>
          <span class="ml-1 text-gray-700">{{ modeLabel(selectedMask.mode) }}</span>
        </div>
        <div>
          <span class="text-gray-400">{{ t('mask.readOnlyLabel') }}:</span>
          <span class="ml-1 text-gray-700">{{ selectedMask.readOnly ? t('mask.yes') : t('mask.no') }}</span>
        </div>
        <div>
          <span class="text-gray-400">{{ t('mask.model') }}:</span>
          <span class="ml-1 text-gray-700">--</span>
        </div>
        <div>
          <span class="text-gray-400">{{ t('mask.toolLimits') }}:</span>
          <span class="ml-1 text-gray-700">--</span>
        </div>
      </div>

      <div class="mt-2 text-sm">
        <span class="text-gray-400">{{ t('mask.bodyLabel') }}:</span>
        <span class="ml-1 text-gray-500 italic">{{ t('mask.bodyRequiresApi') }}</span>
      </div>

      <div class="mt-3 flex items-center gap-2">
        <button
          type="button"
          class="px-3 py-1.5 text-sm font-medium text-white bg-pink-500 hover:bg-pink-600 rounded-lg transition-colors"
          @click="handleCardSelect(selectedMask)"
        >
          {{ t('mask.switchToThis') }}
        </button>
        <button
          type="button"
          class="px-3 py-1.5 text-sm font-medium text-gray-600 border border-gray-300 rounded-lg hover:bg-gray-100 transition-colors"
          @click="handleEditClick(selectedMask)"
        >
          {{ t('mask.edit') }}
        </button>
      </div>
    </div>

    <!-- Delete confirmation dialog -->
    <Teleport to="body">
      <div
        v-if="deleteConfirmName"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
        @click.self="cancelDelete"
      >
        <div class="bg-white rounded-xl shadow-xl p-6 max-w-sm mx-4">
          <h4 class="text-base font-semibold text-gray-800">{{ t('mask.deleteTitle') }}</h4>
          <p class="text-sm text-gray-500 mt-2">
            {{ t('mask.deleteConfirmMessage', { name: deleteConfirmName }) }}
          </p>
          <div class="flex items-center justify-end gap-2 mt-4">
            <button
              type="button"
              class="px-3 py-1.5 text-sm font-medium text-gray-600 border border-gray-300 rounded-lg hover:bg-gray-100 transition-colors"
              @click="cancelDelete"
            >
              {{ t('mask.cancel') }}
            </button>
            <button
              type="button"
              class="px-3 py-1.5 text-sm font-medium text-white bg-red-500 hover:bg-red-600 rounded-lg transition-colors"
              @click="confirmDelete"
            >
              {{ t('mask.delete') }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import {
  X,
  Upload,
  Check,
  FolderOpen,
  Loader2,
  Settings,
  AlertCircle,
  PackageOpen,
} from 'lucide-vue-next'
import type { VrmModelInfo } from '../types'
import { usePetConfig } from '../services/pet-config'
import { toVrmModelId } from '../utils/vrm-model'
import VrmAnimationPanel from './VrmAnimationPanel.vue'
import VrmAppearancePanel from './VrmAppearancePanel.vue'
import { useAppearanceConfig } from '../services/appearance-config'

// ── Tauri availability ──
const isTauri = typeof window !== 'undefined' && ('__TAURI_INTERNALS__' in window || '__TAURI__' in window)

let invoke: ((cmd: string, args?: Record<string, unknown>) => Promise<unknown>) | null = null

async function getInvoke() {
  if (!invoke && isTauri) {
    try {
      const mod = await import('@tauri-apps/api/core')
      invoke = mod.invoke
    } catch {
      console.warn('[DivaPetModelManager] Failed to load @tauri-apps/api/core')
    }
  }
  return invoke
}

// ── Props ──
interface Props {
  visible: boolean
}

const props = defineProps<Props>()

// ── Emits ──
const emit = defineEmits<{
  (e: 'close'): void
  (e: 'modelChanged', modelId: string): void
  (e: 'previewMotion', id: string): void
  (e: 'stopPreview'): void
}>()

// ── Pet config ──
const { config: petConfig, updateConfig } = usePetConfig()

// ── Appearance config ──
const appearanceApi = useAppearanceConfig(petConfig, updateConfig)

// ── Tab state ──
type TabName = 'models' | 'animations' | 'appearances'
const activeTab = ref<TabName>('models')
const tabs = [
  { id: 'models' as TabName, label: '模型' },
  { id: 'animations' as TabName, label: '动画' },
  { id: 'appearances' as TabName, label: '外观' },
]

// ── State ──
const models = ref<VrmModelInfo[]>([])
const isLoading = ref(false)
const loadError = ref<string | null>(null)
const isImporting = ref(false)
const importError = ref<string | null>(null)
const importSuccess = ref<string | null>(null)

// ── Computed ──
const modelLoaded = computed(() => !!petConfig.value.vrmModel)
const activeModelId = computed(() => toVrmModelId(petConfig.value.vrmModel))
const isEmpty = computed(() => !isLoading.value && !loadError.value && models.value.length === 0)

// ── Load models ──
async function loadModels() {
  const inv = await getInvoke()
  if (!inv) {
    loadError.value = 'Tauri runtime not available — model management requires the desktop app'
    return
  }

  isLoading.value = true
  loadError.value = null

  try {
    const result = await inv('pet_list_vrm_models')
    models.value = result as VrmModelInfo[]
  } catch (e) {
    console.warn('[DivaPetModelManager] Failed to load models:', e)
    loadError.value = e instanceof Error ? e.message : 'Failed to load model list'
  } finally {
    isLoading.value = false
  }
}

// ── Import model ──
async function handleImport() {
  const inv = await getInvoke()
  if (!inv) {
    importError.value = 'Import requires Tauri runtime'
    return
  }

  importError.value = null
  importSuccess.value = null
  isImporting.value = true

  try {
    // Try pet_import_vrm_model first; fall back to file dialog hint
    await inv('pet_import_vrm_model')
    importSuccess.value = 'Model imported successfully'
    // Refresh the model list
    await loadModels()
    // Clear success message after 3s
    setTimeout(() => {
      importSuccess.value = null
    }, 3000)
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    // If command doesn't exist, show a helpful message
    if (msg.includes('command not found') || msg.includes('not supported')) {
      importError.value = 'pet_import_vrm_model command not yet available. Place .vrm files in public/vrm/models/ manually.'
    } else {
      console.warn('[DivaPetModelManager] Import failed:', e)
      importError.value = msg
    }
    // Clear error after 5s
    setTimeout(() => {
      importError.value = null
    }, 5000)
  } finally {
    isImporting.value = false
  }
}

// ── Switch model ──
function handleSwitch(model: VrmModelInfo) {
  emit('modelChanged', model.path)
}

// ── Motion preview ──
function handlePreviewMotion(id: string) {
  emit('previewMotion', id)
}
function handleStopPreview() {
  emit('stopPreview')
}
function handleSwitchAppearance(id: string) {
  const switched = appearanceApi.switchAppearance(id)
  if (switched) {
    const appearance = appearanceApi.findAppearance(id)
    if (appearance) {
      emit('modelChanged', appearance.modelId)
    }
  }
}

// ── Close ──
function handleClose() {
  emit('close')
}

// ── Load on visible ──
watch(
  () => props.visible,
  (val) => {
    if (val) {
      loadModels()
    }
  },
)

// ── Keyboard escape ──
function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    handleClose()
  }
}
</script>

<template>
  <Teleport to="body">
    <!-- Backdrop -->
    <div
      v-if="visible"
      class="fixed inset-0 z-50 bg-black/20 backdrop-blur-sm transition-opacity"
      @click.self="handleClose"
      @keydown="onKeydown"
    />

    <!-- Slide-out panel -->
    <Transition name="model-manager-slide">
      <div
        v-if="visible"
        class="fixed right-0 top-0 z-50 h-full w-80 bg-white shadow-2xl flex flex-col"
        @keydown="onKeydown"
      >
        <!-- Header -->
        <div class="flex items-center justify-between px-4 py-3 border-b border-gray-100">
          <div class="flex items-center gap-2">
            <Settings :size="16" class="text-pink-500" />
            <span class="text-sm font-semibold text-gray-800">VRM Models</span>
          </div>
          <button
            class="w-7 h-7 flex items-center justify-center rounded-full text-gray-400 hover:text-gray-600 hover:bg-gray-100 transition-colors"
            @click="handleClose"
          >
            <X :size="16" />
          </button>
        </div>

        <!-- Tab bar -->
        <div class="flex border-b border-gray-100">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            class="flex-1 px-3 py-2 text-xs font-medium text-center border-b-2 transition-colors"
            :class="activeTab === tab.id ? 'border-pink-500 text-pink-600' : 'border-transparent text-gray-400 hover:text-gray-600 hover:border-gray-200'"
            @click="activeTab = tab.id"
          >
            {{ tab.label }}
          </button>
        </div>

        <!-- Models tab content -->
        <template v-if="activeTab === 'models'">
          <!-- Import button -->
        <div class="px-4 py-3 border-b border-gray-50">
          <button
            :disabled="isImporting"
            class="w-full flex items-center justify-center gap-2 px-3 py-2 text-xs font-medium rounded-lg border border-pink-200 bg-pink-50 text-pink-600 hover:bg-pink-100 hover:border-pink-300 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            @click="handleImport"
          >
            <Loader2 v-if="isImporting" :size="14" class="animate-spin" />
            <Upload v-else :size="14" />
            <span v-if="isImporting">Importing...</span>
            <span v-else>Import .vrm Model</span>
          </button>

          <!-- Import feedback -->
          <Transition name="fade">
            <div
              v-if="importError"
              class="mt-2 flex items-start gap-1.5 text-xs text-red-600 bg-red-50 border border-red-100 rounded-md px-2 py-1.5"
            >
              <AlertCircle :size="13" class="shrink-0 mt-px" />
              <span>{{ importError }}</span>
            </div>
          </Transition>

          <Transition name="fade">
            <div
              v-if="importSuccess"
              class="mt-2 flex items-center gap-1.5 text-xs text-green-600 bg-green-50 border border-green-100 rounded-md px-2 py-1.5"
            >
              <Check :size="13" class="shrink-0" />
              <span>{{ importSuccess }}</span>
            </div>
          </Transition>
        </div>

        <!-- Model list -->
        <div class="flex-1 overflow-y-auto">
          <!-- Loading -->
          <div v-if="isLoading" class="flex items-center justify-center py-12 text-gray-400">
            <Loader2 :size="20" class="animate-spin" />
            <span class="ml-2 text-xs">Loading models...</span>
          </div>

          <!-- Error -->
          <div v-else-if="loadError" class="flex flex-col items-center justify-center py-12 px-4">
            <AlertCircle :size="28" class="text-amber-500 mb-2" />
            <p class="text-xs text-gray-500 text-center">{{ loadError }}</p>
          </div>

          <!-- Empty -->
          <div v-else-if="isEmpty" class="flex flex-col items-center justify-center py-12 px-4">
            <PackageOpen :size="28" class="text-gray-300 mb-2" />
            <p class="text-xs text-gray-400 text-center">
              No VRM models found
            </p>
            <p class="text-[10px] text-gray-300 text-center mt-1">
              Click "Import .vrm Model" to add one,<br />
              or place .vrm files in the models directory.
            </p>
          </div>

          <!-- Model entries -->
          <div v-else class="py-1">
            <button
              v-for="model in models"
              :key="model.id"
              class="w-full flex items-center gap-3 px-4 py-2.5 text-left hover:bg-pink-50/50 transition-colors border-b border-gray-50 last:border-b-0"
              :class="activeModelId === model.id ? 'bg-pink-50/70' : ''"
              @click="handleSwitch(model)"
            >
              <!-- Thumbnail / placeholder -->
              <div
                class="w-10 h-10 rounded-lg overflow-hidden shrink-0 border border-gray-100 flex items-center justify-center"
                :class="activeModelId === model.id ? 'border-pink-200 bg-pink-100' : 'bg-gray-50'"
              >
                <img
                  v-if="model.thumbnail"
                  :src="model.thumbnail"
                  :alt="model.name"
                  class="w-full h-full object-cover"
                />
                <FolderOpen v-else :size="18" class="text-gray-400" />
              </div>

              <!-- Model info -->
              <div class="flex-1 min-w-0">
                <div class="text-xs font-medium text-gray-800 truncate">
                  {{ model.name }}
                </div>
                <div class="text-[10px] text-gray-400 truncate mt-0.5">
                  {{ model.path }}
                </div>
              </div>

              <!-- Active indicator / Switch -->
              <div v-if="activeModelId === model.id" class="flex items-center gap-1 text-pink-500 shrink-0">
                <Check :size="14" />
                <span class="text-[10px] font-medium">Active</span>
              </div>
              <div v-else class="text-[10px] text-gray-400 shrink-0">
                Select
              </div>
            </button>
          </div>
        </div>
        </template>

        <!-- Animations tab content -->
        <VrmAnimationPanel
          v-if="activeTab === 'animations'"
          :inline="true"
          :visible="true"
          :motion-list="petConfig.vrmMotionList"
          :selected-motion-ids="petConfig.selectedMotionIds"
          :vrm-motion-enabled="petConfig.vrmMotionEnabled"
          :model-loaded="modelLoaded"
          @update:selected-motion-ids="(ids: string[]) => updateConfig({ selectedMotionIds: ids })"
          @update:vrm-motion-enabled="(val: boolean) => updateConfig({ vrmMotionEnabled: val })"
          @preview-motion="(id: string) => handlePreviewMotion(id)"
          @stop-preview="handleStopPreview"
        />

        <!-- Appearances tab content -->
        <VrmAppearancePanel
          v-if="activeTab === 'appearances'"
          :inline="true"
          :visible="true"
          :appearances="petConfig.vrmAppearances"
          :active-appearance-id="petConfig.activeAppearanceId"
          :models="models"
          :motion-list="petConfig.vrmMotionList"
          @create-appearance="appearanceApi.createAppearance"
          @update-appearance="appearanceApi.updateAppearance"
          @delete-appearance="appearanceApi.deleteAppearance"
          @switch-appearance="handleSwitchAppearance"
        />

        <!-- Footer: Tauri status -->
        <div class="px-4 py-2 border-t border-gray-100">
          <div class="flex items-center gap-1.5 text-[10px] text-gray-400">
            <div
              class="w-1.5 h-1.5 rounded-full"
              :class="isTauri ? 'bg-green-400' : 'bg-gray-300'"
            />
            <span>{{ isTauri ? 'Tauri desktop' : 'Browser mode' }}</span>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.model-manager-slide-enter-active,
.model-manager-slide-leave-active {
  transition: transform 0.2s ease-out, opacity 0.2s ease-out;
}

.model-manager-slide-enter-from {
  transform: translateX(100%);
  opacity: 0;
}

.model-manager-slide-leave-to {
  transform: translateX(100%);
  opacity: 0;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease-out;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>

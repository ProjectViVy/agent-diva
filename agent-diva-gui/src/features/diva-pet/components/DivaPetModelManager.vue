<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { AlertCircle, Check, FolderOpen, Loader2, PackageOpen, Settings, Trash2, Upload, X } from 'lucide-vue-next'
import type { VrmAppearanceConfig, VrmModelInfo } from '../types'
import { usePetConfig } from '../services/pet-config'
import { useAppearanceConfig } from '../services/appearance-config'
import { toVrmModelId } from '../utils/vrm-model'
import { buildKnownMotionInfo, scanVRMAnimations } from '../utils/vrm-animation-scanner'
import {
  DEFAULT_APPEARANCE_ID,
  DEFAULT_VRM_APPEARANCE,
  DEFAULT_VRM_MODEL_PATH,
  resolveAppearance,
} from '../utils/default-appearance'
import VrmAnimationPanel from './VrmAnimationPanel.vue'
import VrmAppearancePanel from './VrmAppearancePanel.vue'

interface Props {
  visible: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'modelChanged', modelId: string): void
  (e: 'previewMotion', id: string): void
  (e: 'stopPreview'): void
}>()

type TabName = 'appearance' | 'model' | 'animation'
const tabs: Array<{ id: TabName; label: string }> = [
  { id: 'appearance', label: '外观' },
  { id: 'model', label: 'VRM 模型' },
  { id: 'animation', label: '动画' },
]

const isTauri =
  typeof window !== 'undefined' &&
  ('__TAURI_INTERNALS__' in window || '__TAURI__' in window)

let invoke: ((cmd: string, args?: Record<string, unknown>) => Promise<unknown>) | null = null
async function getInvoke() {
  if (!invoke && isTauri) {
    const mod = await import('@tauri-apps/api/core')
    invoke = mod.invoke
  }
  return invoke
}

const { config: petConfig, updateConfig } = usePetConfig()
const appearanceApi = useAppearanceConfig(petConfig, updateConfig)

const activeTab = ref<TabName>('appearance')
const models = ref<VrmModelInfo[]>([])
const isLoading = ref(false)
const isImporting = ref(false)
const loadError = ref<string | null>(null)
const importError = ref<string | null>(null)
const fileInput = ref<HTMLInputElement | null>(null)

const activeModelId = computed(() => toVrmModelId(petConfig.value.vrmModel || DEFAULT_VRM_MODEL_PATH))
const modelLoaded = computed(() => !!petConfig.value.vrmModel || models.value.length > 0)
const currentModelName = computed(() => {
  const current = models.value.find(
    (model) => model.path === petConfig.value.vrmModel || model.id === activeModelId.value,
  )
  return current?.name ?? 'Alice'
})

async function loadModels() {
  isLoading.value = true
  loadError.value = null
  try {
    const inv = await getInvoke()
    if (inv) {
      models.value = await inv('pet_list_vrm_models') as VrmModelInfo[]
    } else {
      models.value = [{ id: 'Alice', name: 'Alice', path: DEFAULT_VRM_MODEL_PATH, source: 'builtin' }]
    }
  } catch (error) {
    loadError.value = error instanceof Error ? error.message : String(error)
    models.value = [{ id: 'Alice', name: 'Alice', path: DEFAULT_VRM_MODEL_PATH, source: 'builtin' }]
  } finally {
    isLoading.value = false
    ensureEffectiveAppearance()
  }
}

async function loadMotions() {
  const motions = await scanVRMAnimations().catch(() => [])
  updateConfig({ vrmMotionList: motions.length > 0 ? motions : buildKnownMotionInfo() })
}

function applyAppearance(appearance: VrmAppearanceConfig) {
  updateConfig({
    activeAppearanceId: appearance.id,
    vrmModel: appearance.modelId,
    selectedMotionIds: [...appearance.motionIds],
    vrmMotionEnabled: appearance.motionEnabled,
    vrmExpressionEnabled: appearance.expressionEnabled,
  })
  emit('modelChanged', appearance.modelId)
}

function ensureEffectiveAppearance() {
  const effective = resolveAppearance(
    petConfig.value.vrmAppearances,
    petConfig.value.activeAppearanceId,
    models.value,
  )

  if (
    petConfig.value.activeAppearanceId !== effective.id ||
    petConfig.value.vrmModel !== effective.modelId
  ) {
    applyAppearance(effective)
  }
}

function selectModel(model: VrmModelInfo) {
  updateConfig({ vrmModel: model.path })
  emit('modelChanged', model.path)
}

function openImportPicker() {
  importError.value = null
  fileInput.value?.click()
}

async function importSelectedFile(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  if (!file.name.toLowerCase().endsWith('.vrm')) {
    importError.value = '只能导入 .vrm 模型文件'
    return
  }

  const inv = await getInvoke()
  if (!inv) {
    importError.value = '导入自定义模型需要在 Tauri 桌面端运行。'
    return
  }

  isImporting.value = true
  importError.value = null
  try {
    const base64Data = await readFileAsBase64(file)
    const imported = await inv('pet_import_vrm_model', {
      payload: { fileName: file.name, base64Data },
    }) as VrmModelInfo
    await loadModels()
    selectModel(imported)
  } catch (error) {
    importError.value = error instanceof Error ? error.message : String(error)
  } finally {
    isImporting.value = false
  }
}

async function deleteModel(model: VrmModelInfo) {
  if (model.source !== 'custom') return
  const inv = await getInvoke()
  if (!inv) return
  await inv('pet_delete_vrm_model', { relativePath: model.path })
  if (petConfig.value.vrmModel === model.path) {
    applyAppearance(DEFAULT_VRM_APPEARANCE)
  }
  await loadModels()
}

function createAppearance(appearance: VrmAppearanceConfig) {
  appearanceApi.createAppearance(appearance)
}

function deleteAppearance(id: string) {
  appearanceApi.deleteAppearance(id)
  if (petConfig.value.activeAppearanceId === DEFAULT_APPEARANCE_ID) {
    applyAppearance(DEFAULT_VRM_APPEARANCE)
  }
}

function switchAppearance(id: string) {
  const appearance = resolveAppearance(petConfig.value.vrmAppearances, id, models.value)
  applyAppearance(appearance)
}

function readFileAsBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const result = String(reader.result ?? '')
      resolve(result.includes(',') ? result.split(',').pop() ?? '' : result)
    }
    reader.onerror = () => reject(reader.error ?? new Error('读取模型文件失败'))
    reader.readAsDataURL(file)
  })
}

watch(
  () => petConfig.value.vrmModel,
  (model) => {
    if (!model) {
      applyAppearance(DEFAULT_VRM_APPEARANCE)
    }
  },
)

watch(
  () => [petConfig.value.activeAppearanceId, petConfig.value.vrmAppearances, models.value] as const,
  () => ensureEffectiveAppearance(),
  { deep: true },
)

watch(
  () => petConfig.value.vrmMotionList.length,
  (length) => {
    if (length === 0) {
      void loadMotions()
    }
  },
  { immediate: true },
)

watch(
  () => props.visible,
  (visible) => {
    if (visible) {
      void loadModels()
      void loadMotions()
    }
  },
  { immediate: true },
)
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="fixed inset-0 z-50 bg-black/20 backdrop-blur-sm"
      @click.self="emit('close')"
    />

    <Transition name="model-manager-slide">
      <div
        v-if="visible"
        class="fixed right-0 top-0 z-50 flex h-full w-80 flex-col bg-white shadow-2xl"
      >
        <div class="flex items-center justify-between border-b border-gray-100 px-4 py-3">
          <div class="flex items-center gap-2">
            <Settings :size="16" class="text-pink-500" />
            <span class="text-sm font-semibold text-gray-800">角色设置</span>
          </div>
          <button
            class="flex h-7 w-7 items-center justify-center rounded-full text-gray-400 transition-colors hover:bg-gray-100 hover:text-gray-600"
            @click="emit('close')"
          >
            <X :size="16" />
          </button>
        </div>

        <div class="flex border-b border-gray-100">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            class="flex-1 border-b-2 px-3 py-2 text-center text-xs font-medium transition-colors"
            :class="activeTab === tab.id ? 'border-pink-500 text-pink-600' : 'border-transparent text-gray-400 hover:text-gray-600'"
            @click="activeTab = tab.id"
          >
            {{ tab.label }}
          </button>
        </div>

        <div class="flex-1 min-h-0 overflow-y-auto">
          <VrmAppearancePanel
            v-if="activeTab === 'appearance'"
            :inline="true"
            :visible="true"
            :appearances="petConfig.vrmAppearances"
            :active-appearance-id="petConfig.activeAppearanceId"
            :models="models"
            :motion-list="petConfig.vrmMotionList"
            @create-appearance="createAppearance"
            @update-appearance="appearanceApi.updateAppearance"
            @delete-appearance="deleteAppearance"
            @switch-appearance="switchAppearance"
          />

          <div v-else-if="activeTab === 'model'" class="flex min-h-full flex-col">
            <div class="border-b border-gray-100 px-4 py-3">
              <div class="mb-2 text-xs font-semibold text-gray-800">当前模型：{{ currentModelName }}</div>
              <input ref="fileInput" type="file" accept=".vrm" class="hidden" @change="importSelectedFile" />
              <button
                :disabled="isImporting"
                class="flex w-full items-center justify-center gap-2 rounded-lg border border-pink-200 bg-pink-50 px-3 py-2 text-xs font-medium text-pink-600 transition-colors hover:bg-pink-100 disabled:cursor-not-allowed disabled:opacity-50"
                @click="openImportPicker"
              >
                <Loader2 v-if="isImporting" :size="14" class="animate-spin" />
                <Upload v-else :size="14" />
                <span>导入 .vrm 模型</span>
              </button>
              <div v-if="importError" class="mt-2 flex gap-1.5 rounded-md border border-red-100 bg-red-50 px-2 py-1.5 text-xs text-red-600">
                <AlertCircle :size="13" class="shrink-0" />
                <span>{{ importError }}</span>
              </div>
            </div>

            <div v-if="isLoading" class="flex items-center justify-center py-6 text-xs text-gray-400">
              <Loader2 :size="16" class="mr-2 animate-spin" />
              加载模型中...
            </div>
            <div v-else-if="loadError" class="px-4 py-3 text-xs text-amber-600">{{ loadError }}</div>
            <div v-else-if="models.length === 0" class="flex flex-col items-center justify-center px-4 py-8 text-gray-400">
              <PackageOpen :size="24" class="mb-2 text-gray-300" />
              <p class="text-xs">未找到 VRM 模型</p>
            </div>
            <template v-else>
              <button
                v-for="model in models"
                :key="model.path"
                class="flex w-full items-center gap-3 border-b border-gray-50 px-4 py-2.5 text-left transition-colors last:border-b-0 hover:bg-pink-50/50"
                :class="activeModelId === model.id ? 'bg-pink-50/70' : ''"
                @click="selectModel(model)"
              >
                <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border border-gray-100 bg-gray-50">
                  <FolderOpen :size="17" class="text-gray-400" />
                </div>
                <div class="min-w-0 flex-1">
                  <div class="truncate text-xs font-medium text-gray-800">{{ model.name }}</div>
                  <div class="truncate text-[10px] text-gray-400">{{ model.source === 'custom' ? '自定义' : '内置' }}</div>
                </div>
                <Check v-if="activeModelId === model.id" :size="14" class="text-pink-500" />
                <button
                  v-if="model.source === 'custom'"
                  class="text-gray-400 hover:text-red-500"
                  @click.stop="deleteModel(model)"
                >
                  <Trash2 :size="13" />
                </button>
              </button>
            </template>
          </div>

          <VrmAnimationPanel
            v-else
            :inline="true"
            :visible="visible"
            :motion-list="petConfig.vrmMotionList"
            :selected-motion-ids="petConfig.selectedMotionIds"
            :vrm-motion-enabled="petConfig.vrmMotionEnabled"
            :model-loaded="modelLoaded"
            @update:selected-motion-ids="(ids: string[]) => updateConfig({ selectedMotionIds: ids })"
            @update:vrm-motion-enabled="(value: boolean) => updateConfig({ vrmMotionEnabled: value })"
            @preview-motion="(id: string) => emit('previewMotion', id)"
            @stop-preview="emit('stopPreview')"
          />
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

.model-manager-slide-enter-from,
.model-manager-slide-leave-to {
  transform: translateX(100%);
  opacity: 0;
}
</style>

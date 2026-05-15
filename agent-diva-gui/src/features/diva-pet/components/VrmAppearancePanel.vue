<script setup lang="ts">
import { ref, computed } from 'vue'
import {
  X,
  Palette,
  Plus,
  Pencil,
  Trash2,
  Check,
  Circle,
  ChevronDown,
  AlertCircle,
  PackageOpen,
} from 'lucide-vue-next'
import type { VrmAppearanceConfig, VrmModelInfo, VrmMotionInfo } from '../types'

// ── Props ──
interface Props {
  visible: boolean
  appearances: VrmAppearanceConfig[]
  activeAppearanceId: string
  models: VrmModelInfo[]
  motionList: VrmMotionInfo[]
  inline?: boolean
}

const props = defineProps<Props>()

// ── Emits ──
const emit = defineEmits<{
  (e: 'close'): void
  (e: 'createAppearance', appearance: VrmAppearanceConfig): void
  (e: 'updateAppearance', id: string, patch: Partial<VrmAppearanceConfig>): void
  (e: 'deleteAppearance', id: string): void
  (e: 'switchAppearance', id: string): void
}>()

// ── Tauri detection (for footer indicator only) ──
const isTauri =
  typeof window !== 'undefined' &&
  ('__TAURI_INTERNALS__' in window || '__TAURI__' in window)

// ── UI state ──
const editingAppearanceId = ref<string | null>(null)
const isCreating = ref(false)
const deletingAppearanceId = ref<string | null>(null)

// Form fields
const formName = ref('')
const formModelId = ref('')
const formMotionIds = ref<string[]>([])
const formExpressionEnabled = ref(false)
const formMotionEnabled = ref(false)

// ── Computed ──
const isEmpty = computed(() => props.appearances.length === 0)

// ── Helpers ──
function getModelName(modelId: string): string {
  const model = props.models.find((m) => m.id === modelId)
  return model?.name ?? modelId
}

function toggleMotion(motionId: string) {
  const idx = formMotionIds.value.indexOf(motionId)
  if (idx === -1) {
    formMotionIds.value.push(motionId)
  } else {
    formMotionIds.value.splice(idx, 1)
  }
}

// ── Actions ──
function handleClose() {
  emit('close')
}

function startCreate() {
  isCreating.value = true
  editingAppearanceId.value = null
  formName.value = ''
  formModelId.value = ''
  formMotionIds.value = []
  formExpressionEnabled.value = false
  formMotionEnabled.value = false
}

function startEdit(appearance: VrmAppearanceConfig) {
  isCreating.value = false
  editingAppearanceId.value = appearance.id
  formName.value = appearance.name
  formModelId.value = appearance.modelId
  formMotionIds.value = [...appearance.motionIds]
  formExpressionEnabled.value = appearance.expressionEnabled
  formMotionEnabled.value = appearance.motionEnabled
}

function cancelForm() {
  isCreating.value = false
  editingAppearanceId.value = null
}

function generateId(): string {
  return `appearance_${Date.now()}_${Math.floor(Math.random() * 10000)}`
}

function saveForm() {
  if (isCreating.value) {
    emit('createAppearance', {
      id: generateId(),
      name: formName.value || '未命名外观',
      modelId: formModelId.value || '',
      motionIds: [...formMotionIds.value],
      expressionEnabled: formExpressionEnabled.value,
      motionEnabled: formMotionEnabled.value,
    })
  } else if (editingAppearanceId.value) {
    emit('updateAppearance', editingAppearanceId.value, {
      name: formName.value,
      modelId: formModelId.value,
      motionIds: [...formMotionIds.value],
      expressionEnabled: formExpressionEnabled.value,
      motionEnabled: formMotionEnabled.value,
    })
  }
  cancelForm()
}

function confirmDelete(id: string) {
  deletingAppearanceId.value = id
  editingAppearanceId.value = null
  isCreating.value = false
}

function cancelDelete() {
  deletingAppearanceId.value = null
}

function executeDelete() {
  if (deletingAppearanceId.value) {
    emit('deleteAppearance', deletingAppearanceId.value)
    deletingAppearanceId.value = null
  }
}

function handleSwitchAppearance(id: string) {
  emit('switchAppearance', id)
}

// ── Keyboard ──
function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    if (deletingAppearanceId.value) {
      deletingAppearanceId.value = null
    } else if (editingAppearanceId.value || isCreating.value) {
      cancelForm()
    } else {
      handleClose()
    }
  }
}
</script>

<template>
  <!-- Full slide-out panel (non-inline, default) -->
  <template v-if="!inline">
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
          <!-- ═══ Header ═══ -->
          <div
            class="flex items-center justify-between px-4 py-3 border-b border-gray-100"
          >
            <div class="flex items-center gap-2">
              <Palette :size="16" class="text-pink-500" />
              <span class="text-sm font-semibold text-gray-800">VRM Appearances</span>
            </div>
            <button
              class="w-7 h-7 flex items-center justify-center rounded-full text-gray-400 hover:text-gray-600 hover:bg-gray-100 transition-colors"
              @click="handleClose"
            >
              <X :size="16" />
            </button>
          </div>

          <!-- ═══ Create button ═══ -->
          <div class="px-4 py-3 border-b border-gray-50">
            <button
              class="w-full flex items-center justify-center gap-1.5 px-3 py-2 text-xs font-medium rounded-lg border border-pink-200 bg-pink-50 text-pink-600 hover:bg-pink-100 hover:border-pink-300 transition-colors"
              @click="startCreate"
            >
              <Plus :size="14" />
              <span>新建外观</span>
            </button>
          </div>

          <!-- ═══ Content ═══ -->
          <div class="flex-1 overflow-y-auto">
            <!-- ── Empty state ── -->
            <div
              v-if="isEmpty && !isCreating"
              class="flex flex-col items-center justify-center py-12 px-4"
            >
              <PackageOpen :size="28" class="text-gray-300 mb-2" />
              <p class="text-xs text-gray-400 text-center">
                暂无外观配置
              </p>
              <p class="text-[10px] text-gray-300 text-center mt-1">
                点击"新建外观"创建模型与动作组合
              </p>
            </div>

            <!-- ── Appearance list / forms ── -->
            <div v-else class="py-1">
              <!-- Inline Create Form -->
              <div
                v-if="isCreating"
                class="px-4 py-3 border-b border-gray-50 bg-pink-50/30"
              >
                <div class="flex items-center gap-2 mb-3">
                  <Plus :size="14" class="text-pink-500" />
                  <span class="text-xs font-semibold text-gray-700">新建外观</span>
                </div>

                <!-- Name -->
                <div class="mb-3">
                  <label class="block text-[10px] text-gray-500 mb-1">名称</label>
                  <input
                    v-model="formName"
                    type="text"
                    placeholder="输入外观名称"
                    class="w-full text-xs border border-gray-200 rounded-md px-2.5 py-1.5 text-gray-800 placeholder-gray-400 focus:outline-none focus:border-pink-300 focus:ring-1 focus:ring-pink-200 transition-colors"
                  />
                </div>

                <!-- Model dropdown -->
                <div class="mb-3">
                  <label class="block text-[10px] text-gray-500 mb-1">模型</label>
                  <div class="relative">
                    <select
                      v-model="formModelId"
                      class="w-full text-xs border border-gray-200 rounded-md px-2.5 py-1.5 text-gray-800 appearance-none bg-white focus:outline-none focus:border-pink-300 focus:ring-1 focus:ring-pink-200 transition-colors"
                    >
                      <option value="" disabled>选择模型</option>
                      <option
                        v-for="model in models"
                        :key="model.id"
                        :value="model.id"
                      >
                        {{ model.name }}
                      </option>
                    </select>
                    <ChevronDown
                      :size="13"
                      class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 pointer-events-none"
                    />
                  </div>
                  <p v-if="models.length === 0" class="text-[10px] text-gray-400 mt-1">
                    暂无可用模型
                  </p>
                </div>

                <!-- Motion checkboxes -->
                <div class="mb-3">
                  <label class="block text-[10px] text-gray-500 mb-1">动作</label>
                  <div
                    v-if="motionList.length === 0"
                    class="text-[10px] text-gray-400 py-1"
                  >
                    暂无可用动作
                  </div>
                  <div v-else class="space-y-1 max-h-28 overflow-y-auto">
                    <label
                      v-for="motion in motionList"
                      :key="motion.id"
                      class="flex items-center gap-2 text-xs text-gray-600 cursor-pointer py-0.5 hover:text-gray-800 transition-colors"
                    >
                      <input
                        type="checkbox"
                        :checked="formMotionIds.includes(motion.id)"
                        class="rounded border-gray-300 text-pink-500 focus:ring-pink-400"
                        @change="toggleMotion(motion.id)"
                      />
                      <span class="truncate">{{ motion.name }}</span>
                    </label>
                  </div>
                </div>

                <!-- Expression toggle -->
                <div class="flex items-center justify-between mb-2.5">
                  <span class="text-[10px] text-gray-500">表情</span>
                  <button
                    type="button"
                    class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors duration-200 focus:outline-none"
                    :class="
                      formExpressionEnabled ? 'bg-pink-500' : 'bg-gray-200'
                    "
                    @click="formExpressionEnabled = !formExpressionEnabled"
                  >
                    <span
                      class="inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow-sm transition-transform duration-200"
                      :class="
                        formExpressionEnabled
                          ? 'translate-x-[18px]'
                          : 'translate-x-[3px]'
                      "
                    />
                  </button>
                </div>

                <!-- Motion toggle -->
                <div class="flex items-center justify-between mb-3">
                  <span class="text-[10px] text-gray-500">动作</span>
                  <button
                    type="button"
                    class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors duration-200 focus:outline-none"
                    :class="
                      formMotionEnabled ? 'bg-pink-500' : 'bg-gray-200'
                    "
                    @click="formMotionEnabled = !formMotionEnabled"
                  >
                    <span
                      class="inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow-sm transition-transform duration-200"
                      :class="
                        formMotionEnabled
                          ? 'translate-x-[18px]'
                          : 'translate-x-[3px]'
                      "
                    />
                  </button>
                </div>

                <!-- Form actions -->
                <div class="flex gap-2">
                  <button
                    class="flex-1 text-[10px] px-2 py-1.5 rounded-md border border-gray-200 text-gray-500 hover:bg-gray-100 transition-colors"
                    @click="cancelForm"
                  >
                    取消
                  </button>
                  <button
                    class="flex-1 text-[10px] px-2 py-1.5 rounded-md bg-pink-500 text-white hover:bg-pink-600 transition-colors font-medium"
                    @click="saveForm"
                  >
                    保存
                  </button>
                </div>
              </div>

              <!-- Appearance Cards -->
              <template
                v-for="appearance in appearances"
                :key="appearance.id"
              >
                <!-- ── Card row ── -->
                <div
                  class="w-full border-b border-gray-50 last:border-b-0"
                  :class="
                    activeAppearanceId === appearance.id
                      ? 'bg-pink-50/40'
                      : 'hover:bg-gray-50/50'
                  "
                >
                  <div class="flex items-start gap-2.5 px-4 py-2.5">
                    <!-- Radio indicator -->
                    <div class="shrink-0 pt-px">
                      <div
                        v-if="activeAppearanceId === appearance.id"
                        class="w-4 h-4 rounded-full bg-pink-500 flex items-center justify-center"
                      >
                        <div class="w-1.5 h-1.5 rounded-full bg-white" />
                      </div>
                      <Circle
                        v-else
                        :size="16"
                        class="text-gray-300"
                      />
                    </div>

                    <!-- Content -->
                    <div class="flex-1 min-w-0">
                      <!-- Top row: name + action buttons -->
                      <div class="flex items-center justify-between">
                        <span
                          class="text-xs font-bold text-gray-800 truncate"
                        >
                          {{ appearance.name }}
                        </span>
                        <div class="flex items-center gap-0.5 shrink-0 ml-2">
                          <!-- Edit -->
                          <button
                            class="w-6 h-6 flex items-center justify-center rounded text-gray-400 hover:text-gray-600 hover:bg-gray-100 transition-colors"
                            @click="startEdit(appearance)"
                          >
                            <Pencil :size="13" />
                          </button>
                          <!-- Delete -->
                          <button
                            class="w-6 h-6 flex items-center justify-center rounded text-gray-400 hover:text-red-500 hover:bg-red-50 transition-colors"
                            @click="confirmDelete(appearance.id)"
                          >
                            <Trash2 :size="13" />
                          </button>
                        </div>
                      </div>

                      <!-- Summary line -->
                      <div class="text-[10px] text-gray-400 mt-0.5">
                        {{ getModelName(appearance.modelId) }} + {{ appearance.motionIds.length }} 个动作
                      </div>

                      <!-- Bottom row: activate / active badge -->
                      <div class="mt-1.5">
                        <button
                          v-if="activeAppearanceId !== appearance.id"
                          class="text-[10px] px-2 py-0.5 rounded border border-pink-200 text-pink-500 hover:bg-pink-50 transition-colors"
                          @click="handleSwitchAppearance(appearance.id)"
                        >
                          激活
                        </button>
                        <span
                          v-else
                          class="inline-flex items-center gap-0.5 text-[10px] text-pink-500 font-medium"
                        >
                          <Check :size="11" />
                          当前
                        </span>
                      </div>
                    </div>
                  </div>
                </div>

                <!-- ── Inline Edit Form ── -->
                <div
                  v-if="editingAppearanceId === appearance.id"
                  class="px-4 py-3 border-b border-gray-50 bg-gray-50/60"
                >
                  <div class="flex items-center gap-2 mb-3">
                    <Pencil :size="14" class="text-gray-500" />
                    <span class="text-xs font-semibold text-gray-700"
                      >编辑外观</span
                    >
                  </div>

                  <!-- Name -->
                  <div class="mb-3">
                    <label class="block text-[10px] text-gray-500 mb-1"
                      >名称</label
                    >
                    <input
                      v-model="formName"
                      type="text"
                      placeholder="输入外观名称"
                      class="w-full text-xs border border-gray-200 rounded-md px-2.5 py-1.5 text-gray-800 placeholder-gray-400 focus:outline-none focus:border-pink-300 focus:ring-1 focus:ring-pink-200 transition-colors"
                    />
                  </div>

                  <!-- Model dropdown -->
                  <div class="mb-3">
                    <label class="block text-[10px] text-gray-500 mb-1"
                      >模型</label
                    >
                    <div class="relative">
                      <select
                        v-model="formModelId"
                        class="w-full text-xs border border-gray-200 rounded-md px-2.5 py-1.5 text-gray-800 appearance-none bg-white focus:outline-none focus:border-pink-300 focus:ring-1 focus:ring-pink-200 transition-colors"
                      >
                        <option value="" disabled>选择模型</option>
                        <option
                          v-for="model in models"
                          :key="model.id"
                          :value="model.id"
                        >
                          {{ model.name }}
                        </option>
                      </select>
                      <ChevronDown
                        :size="13"
                        class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 pointer-events-none"
                      />
                    </div>
                    <p
                      v-if="models.length === 0"
                      class="text-[10px] text-gray-400 mt-1"
                    >
                      暂无可用模型
                    </p>
                  </div>

                  <!-- Motion checkboxes -->
                  <div class="mb-3">
                    <label class="block text-[10px] text-gray-500 mb-1"
                      >动作</label
                    >
                    <div
                      v-if="motionList.length === 0"
                      class="text-[10px] text-gray-400 py-1"
                    >
                      暂无可用动作
                    </div>
                    <div v-else class="space-y-1 max-h-28 overflow-y-auto">
                      <label
                        v-for="motion in motionList"
                        :key="motion.id"
                        class="flex items-center gap-2 text-xs text-gray-600 cursor-pointer py-0.5 hover:text-gray-800 transition-colors"
                      >
                        <input
                          type="checkbox"
                          :checked="
                            formMotionIds.includes(motion.id)
                          "
                          class="rounded border-gray-300 text-pink-500 focus:ring-pink-400"
                          @change="toggleMotion(motion.id)"
                        />
                        <span class="truncate">{{ motion.name }}</span>
                      </label>
                    </div>
                  </div>

                  <!-- Expression toggle -->
                  <div class="flex items-center justify-between mb-2.5">
                    <span class="text-[10px] text-gray-500">表情</span>
                    <button
                      type="button"
                      class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors duration-200 focus:outline-none"
                      :class="
                        formExpressionEnabled
                          ? 'bg-pink-500'
                          : 'bg-gray-200'
                      "
                      @click="
                        formExpressionEnabled =
                          !formExpressionEnabled
                      "
                    >
                      <span
                        class="inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow-sm transition-transform duration-200"
                        :class="
                          formExpressionEnabled
                            ? 'translate-x-[18px]'
                            : 'translate-x-[3px]'
                        "
                      />
                    </button>
                  </div>

                  <!-- Motion toggle -->
                  <div class="flex items-center justify-between mb-3">
                    <span class="text-[10px] text-gray-500">动作</span>
                    <button
                      type="button"
                      class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors duration-200 focus:outline-none"
                      :class="
                        formMotionEnabled ? 'bg-pink-500' : 'bg-gray-200'
                      "
                      @click="
                        formMotionEnabled = !formMotionEnabled
                      "
                    >
                      <span
                        class="inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow-sm transition-transform duration-200"
                        :class="
                          formMotionEnabled
                            ? 'translate-x-[18px]'
                            : 'translate-x-[3px]'
                        "
                      />
                    </button>
                  </div>

                  <!-- Form actions -->
                  <div class="flex gap-2">
                    <button
                      class="flex-1 text-[10px] px-2 py-1.5 rounded-md border border-gray-200 text-gray-500 hover:bg-gray-100 transition-colors"
                      @click="cancelForm"
                    >
                      取消
                    </button>
                    <button
                      class="flex-1 text-[10px] px-2 py-1.5 rounded-md bg-pink-500 text-white hover:bg-pink-600 transition-colors font-medium"
                      @click="saveForm"
                    >
                      保存
                    </button>
                  </div>
                </div>

                <!-- ── Delete confirmation ── -->
                <div
                  v-if="deletingAppearanceId === appearance.id"
                  class="px-4 py-2.5 border-b border-gray-50 bg-red-50/60"
                >
                  <div class="flex items-start gap-1.5 mb-2">
                    <AlertCircle
                      :size="14"
                      class="text-red-500 shrink-0 mt-px"
                    />
                    <p class="text-[10px] text-red-600 font-medium">
                      确认删除此外观？
                    </p>
                  </div>
                  <div class="flex gap-2">
                    <button
                      class="flex-1 text-[10px] px-2 py-1 rounded-md border border-gray-200 text-gray-500 hover:bg-gray-100 transition-colors"
                      @click="cancelDelete"
                    >
                      取消
                    </button>
                    <button
                      class="flex-1 text-[10px] px-2 py-1 rounded-md bg-red-500 text-white hover:bg-red-600 transition-colors font-medium"
                      @click="executeDelete"
                    >
                      确认删除
                    </button>
                  </div>
                </div>
              </template>
            </div>
          </div>

          <!-- ═══ Footer: Tauri status indicator ═══ -->
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

  <!-- Inline mode: inner content only, no backdrop/header/footer/fixed positioning -->
  <div v-else class="flex flex-col h-full">
    <!-- ═══ Create button ═══ -->
    <div class="px-4 py-3 border-b border-gray-50">
      <button
        class="w-full flex items-center justify-center gap-1.5 px-3 py-2 text-xs font-medium rounded-lg border border-pink-200 bg-pink-50 text-pink-600 hover:bg-pink-100 hover:border-pink-300 transition-colors"
        @click="startCreate"
      >
        <Plus :size="14" />
        <span>新建外观</span>
      </button>
    </div>

    <!-- ═══ Content ═══ -->
    <div class="flex-1 overflow-y-auto">
      <!-- ── Empty state ── -->
      <div
        v-if="isEmpty && !isCreating"
        class="flex flex-col items-center justify-center py-12 px-4"
      >
        <PackageOpen :size="28" class="text-gray-300 mb-2" />
        <p class="text-xs text-gray-400 text-center">
          暂无外观配置
        </p>
        <p class="text-[10px] text-gray-300 text-center mt-1">
          点击"新建外观"创建模型与动作组合
        </p>
      </div>

      <!-- ── Appearance list / forms ── -->
      <div v-else class="py-1">
        <!-- Inline Create Form -->
        <div
          v-if="isCreating"
          class="px-4 py-3 border-b border-gray-50 bg-pink-50/30"
        >
          <div class="flex items-center gap-2 mb-3">
            <Plus :size="14" class="text-pink-500" />
            <span class="text-xs font-semibold text-gray-700">新建外观</span>
          </div>

          <!-- Name -->
          <div class="mb-3">
            <label class="block text-[10px] text-gray-500 mb-1">名称</label>
            <input
              v-model="formName"
              type="text"
              placeholder="输入外观名称"
              class="w-full text-xs border border-gray-200 rounded-md px-2.5 py-1.5 text-gray-800 placeholder-gray-400 focus:outline-none focus:border-pink-300 focus:ring-1 focus:ring-pink-200 transition-colors"
            />
          </div>

          <!-- Model dropdown -->
          <div class="mb-3">
            <label class="block text-[10px] text-gray-500 mb-1">模型</label>
            <div class="relative">
              <select
                v-model="formModelId"
                class="w-full text-xs border border-gray-200 rounded-md px-2.5 py-1.5 text-gray-800 appearance-none bg-white focus:outline-none focus:border-pink-300 focus:ring-1 focus:ring-pink-200 transition-colors"
              >
                <option value="" disabled>选择模型</option>
                <option
                  v-for="model in models"
                  :key="model.id"
                  :value="model.id"
                >
                  {{ model.name }}
                </option>
              </select>
              <ChevronDown
                :size="13"
                class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 pointer-events-none"
              />
            </div>
            <p v-if="models.length === 0" class="text-[10px] text-gray-400 mt-1">
              暂无可用模型
            </p>
          </div>

          <!-- Motion checkboxes -->
          <div class="mb-3">
            <label class="block text-[10px] text-gray-500 mb-1">动作</label>
            <div
              v-if="motionList.length === 0"
              class="text-[10px] text-gray-400 py-1"
            >
              暂无可用动作
            </div>
            <div v-else class="space-y-1 max-h-28 overflow-y-auto">
              <label
                v-for="motion in motionList"
                :key="motion.id"
                class="flex items-center gap-2 text-xs text-gray-600 cursor-pointer py-0.5 hover:text-gray-800 transition-colors"
              >
                <input
                  type="checkbox"
                  :checked="formMotionIds.includes(motion.id)"
                  class="rounded border-gray-300 text-pink-500 focus:ring-pink-400"
                  @change="toggleMotion(motion.id)"
                />
                <span class="truncate">{{ motion.name }}</span>
              </label>
            </div>
          </div>

          <!-- Expression toggle -->
          <div class="flex items-center justify-between mb-2.5">
            <span class="text-[10px] text-gray-500">表情</span>
            <button
              type="button"
              class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors duration-200 focus:outline-none"
              :class="
                formExpressionEnabled ? 'bg-pink-500' : 'bg-gray-200'
              "
              @click="formExpressionEnabled = !formExpressionEnabled"
            >
              <span
                class="inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow-sm transition-transform duration-200"
                :class="
                  formExpressionEnabled
                    ? 'translate-x-[18px]'
                    : 'translate-x-[3px]'
                "
              />
            </button>
          </div>

          <!-- Motion toggle -->
          <div class="flex items-center justify-between mb-3">
            <span class="text-[10px] text-gray-500">动作</span>
            <button
              type="button"
              class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors duration-200 focus:outline-none"
              :class="
                formMotionEnabled ? 'bg-pink-500' : 'bg-gray-200'
              "
              @click="formMotionEnabled = !formMotionEnabled"
            >
              <span
                class="inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow-sm transition-transform duration-200"
                :class="
                  formMotionEnabled
                    ? 'translate-x-[18px]'
                    : 'translate-x-[3px]'
                "
              />
            </button>
          </div>

          <!-- Form actions -->
          <div class="flex gap-2">
            <button
              class="flex-1 text-[10px] px-2 py-1.5 rounded-md border border-gray-200 text-gray-500 hover:bg-gray-100 transition-colors"
              @click="cancelForm"
            >
              取消
            </button>
            <button
              class="flex-1 text-[10px] px-2 py-1.5 rounded-md bg-pink-500 text-white hover:bg-pink-600 transition-colors font-medium"
              @click="saveForm"
            >
              保存
            </button>
          </div>
        </div>

        <!-- Appearance Cards -->
        <template
          v-for="appearance in appearances"
          :key="appearance.id"
        >
          <!-- ── Card row ── -->
          <div
            class="w-full border-b border-gray-50 last:border-b-0"
            :class="
              activeAppearanceId === appearance.id
                ? 'bg-pink-50/40'
                : 'hover:bg-gray-50/50'
            "
          >
            <div class="flex items-start gap-2.5 px-4 py-2.5">
              <!-- Radio indicator -->
              <div class="shrink-0 pt-px">
                <div
                  v-if="activeAppearanceId === appearance.id"
                  class="w-4 h-4 rounded-full bg-pink-500 flex items-center justify-center"
                >
                  <div class="w-1.5 h-1.5 rounded-full bg-white" />
                </div>
                <Circle
                  v-else
                  :size="16"
                  class="text-gray-300"
                />
              </div>

              <!-- Content -->
              <div class="flex-1 min-w-0">
                <!-- Top row: name + action buttons -->
                <div class="flex items-center justify-between">
                  <span
                    class="text-xs font-bold text-gray-800 truncate"
                  >
                    {{ appearance.name }}
                  </span>
                  <div class="flex items-center gap-0.5 shrink-0 ml-2">
                    <!-- Edit -->
                    <button
                      class="w-6 h-6 flex items-center justify-center rounded text-gray-400 hover:text-gray-600 hover:bg-gray-100 transition-colors"
                      @click="startEdit(appearance)"
                    >
                      <Pencil :size="13" />
                    </button>
                    <!-- Delete -->
                    <button
                      class="w-6 h-6 flex items-center justify-center rounded text-gray-400 hover:text-red-500 hover:bg-red-50 transition-colors"
                      @click="confirmDelete(appearance.id)"
                    >
                      <Trash2 :size="13" />
                    </button>
                  </div>
                </div>

                <!-- Summary line -->
                <div class="text-[10px] text-gray-400 mt-0.5">
                  {{ getModelName(appearance.modelId) }} + {{ appearance.motionIds.length }} 个动作
                </div>

                <!-- Bottom row: activate / active badge -->
                <div class="mt-1.5">
                  <button
                    v-if="activeAppearanceId !== appearance.id"
                    class="text-[10px] px-2 py-0.5 rounded border border-pink-200 text-pink-500 hover:bg-pink-50 transition-colors"
                    @click="handleSwitchAppearance(appearance.id)"
                  >
                    激活
                  </button>
                  <span
                    v-else
                    class="inline-flex items-center gap-0.5 text-[10px] text-pink-500 font-medium"
                  >
                    <Check :size="11" />
                    当前
                  </span>
                </div>
              </div>
            </div>
          </div>

          <!-- ── Inline Edit Form ── -->
          <div
            v-if="editingAppearanceId === appearance.id"
            class="px-4 py-3 border-b border-gray-50 bg-gray-50/60"
          >
            <div class="flex items-center gap-2 mb-3">
              <Pencil :size="14" class="text-gray-500" />
              <span class="text-xs font-semibold text-gray-700"
                >编辑外观</span
              >
            </div>

            <!-- Name -->
            <div class="mb-3">
              <label class="block text-[10px] text-gray-500 mb-1"
                >名称</label
              >
              <input
                v-model="formName"
                type="text"
                placeholder="输入外观名称"
                class="w-full text-xs border border-gray-200 rounded-md px-2.5 py-1.5 text-gray-800 placeholder-gray-400 focus:outline-none focus:border-pink-300 focus:ring-1 focus:ring-pink-200 transition-colors"
              />
            </div>

            <!-- Model dropdown -->
            <div class="mb-3">
              <label class="block text-[10px] text-gray-500 mb-1"
                >模型</label
              >
              <div class="relative">
                <select
                  v-model="formModelId"
                  class="w-full text-xs border border-gray-200 rounded-md px-2.5 py-1.5 text-gray-800 appearance-none bg-white focus:outline-none focus:border-pink-300 focus:ring-1 focus:ring-pink-200 transition-colors"
                >
                  <option value="" disabled>选择模型</option>
                  <option
                    v-for="model in models"
                    :key="model.id"
                    :value="model.id"
                  >
                    {{ model.name }}
                  </option>
                </select>
                <ChevronDown
                  :size="13"
                  class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 pointer-events-none"
                />
              </div>
              <p
                v-if="models.length === 0"
                class="text-[10px] text-gray-400 mt-1"
              >
                暂无可用模型
              </p>
            </div>

            <!-- Motion checkboxes -->
            <div class="mb-3">
              <label class="block text-[10px] text-gray-500 mb-1"
                >动作</label
              >
              <div
                v-if="motionList.length === 0"
                class="text-[10px] text-gray-400 py-1"
              >
                暂无可用动作
              </div>
              <div v-else class="space-y-1 max-h-28 overflow-y-auto">
                <label
                  v-for="motion in motionList"
                  :key="motion.id"
                  class="flex items-center gap-2 text-xs text-gray-600 cursor-pointer py-0.5 hover:text-gray-800 transition-colors"
                >
                  <input
                    type="checkbox"
                    :checked="
                      formMotionIds.includes(motion.id)
                    "
                    class="rounded border-gray-300 text-pink-500 focus:ring-pink-400"
                    @change="toggleMotion(motion.id)"
                  />
                  <span class="truncate">{{ motion.name }}</span>
                </label>
              </div>
            </div>

            <!-- Expression toggle -->
            <div class="flex items-center justify-between mb-2.5">
              <span class="text-[10px] text-gray-500">表情</span>
              <button
                type="button"
                class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors duration-200 focus:outline-none"
                :class="
                  formExpressionEnabled
                    ? 'bg-pink-500'
                    : 'bg-gray-200'
                "
                @click="
                  formExpressionEnabled =
                    !formExpressionEnabled
                "
              >
                <span
                  class="inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow-sm transition-transform duration-200"
                  :class="
                    formExpressionEnabled
                      ? 'translate-x-[18px]'
                      : 'translate-x-[3px]'
                  "
                />
              </button>
            </div>

            <!-- Motion toggle -->
            <div class="flex items-center justify-between mb-3">
              <span class="text-[10px] text-gray-500">动作</span>
              <button
                type="button"
                class="relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full transition-colors duration-200 focus:outline-none"
                :class="
                  formMotionEnabled ? 'bg-pink-500' : 'bg-gray-200'
                "
                @click="
                  formMotionEnabled = !formMotionEnabled
                "
              >
                <span
                  class="inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow-sm transition-transform duration-200"
                  :class="
                    formMotionEnabled
                      ? 'translate-x-[18px]'
                      : 'translate-x-[3px]'
                  "
                />
              </button>
            </div>

            <!-- Form actions -->
            <div class="flex gap-2">
              <button
                class="flex-1 text-[10px] px-2 py-1.5 rounded-md border border-gray-200 text-gray-500 hover:bg-gray-100 transition-colors"
                @click="cancelForm"
              >
                取消
              </button>
              <button
                class="flex-1 text-[10px] px-2 py-1.5 rounded-md bg-pink-500 text-white hover:bg-pink-600 transition-colors font-medium"
                @click="saveForm"
              >
                保存
              </button>
            </div>
          </div>

          <!-- ── Delete confirmation ── -->
          <div
            v-if="deletingAppearanceId === appearance.id"
            class="px-4 py-2.5 border-b border-gray-50 bg-red-50/60"
          >
            <div class="flex items-start gap-1.5 mb-2">
              <AlertCircle
                :size="14"
                class="text-red-500 shrink-0 mt-px"
              />
              <p class="text-[10px] text-red-600 font-medium">
                确认删除此外观？
              </p>
            </div>
            <div class="flex gap-2">
              <button
                class="flex-1 text-[10px] px-2 py-1 rounded-md border border-gray-200 text-gray-500 hover:bg-gray-100 transition-colors"
                @click="cancelDelete"
              >
                取消
              </button>
              <button
                class="flex-1 text-[10px] px-2 py-1 rounded-md bg-red-500 text-white hover:bg-red-600 transition-colors font-medium"
                @click="executeDelete"
              >
                确认删除
              </button>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* ── Slide transition (matches DivaPetModelManager) ── */
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
</style>

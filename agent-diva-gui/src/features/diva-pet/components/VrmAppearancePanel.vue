<script setup lang="ts">
import { computed, ref } from 'vue'
import { Check, Circle, Pencil, Plus, Trash2, X } from 'lucide-vue-next'
import type { VrmAppearanceConfig, VrmModelInfo, VrmMotionInfo } from '../types'
import {
  DEFAULT_APPEARANCE_ID,
  isDefaultAppearanceId,
  withDefaultAppearance,
} from '../utils/default-appearance'

interface Props {
  visible: boolean
  appearances: VrmAppearanceConfig[]
  activeAppearanceId: string
  models: VrmModelInfo[]
  motionList: VrmMotionInfo[]
  inline?: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'createAppearance', appearance: VrmAppearanceConfig): void
  (e: 'updateAppearance', id: string, patch: Partial<VrmAppearanceConfig>): void
  (e: 'deleteAppearance', id: string): void
  (e: 'switchAppearance', id: string): void
}>()

const editingId = ref<string | null>(null)
const isCreating = ref(false)
const formName = ref('')
const formModelId = ref('')
const formMotionIds = ref<string[]>([])
const formStartMotionId = ref('appearing')
const formExpressionEnabled = ref(true)
const formMotionEnabled = ref(true)

const idleMotions = computed(() => props.motionList.filter((motion) => motion.kind === 'idle'))
const startupMotions = computed(() => props.motionList.filter((motion) => motion.kind === 'startup'))
const displayedAppearances = computed(() => withDefaultAppearance(props.appearances))
const effectiveActiveId = computed(() =>
  displayedAppearances.value.some((appearance) => appearance.id === props.activeAppearanceId)
    ? props.activeAppearanceId
    : DEFAULT_APPEARANCE_ID,
)
const selectableModels = computed(() => props.models)

function modelName(modelId: string): string {
  return props.models.find((model) => model.id === modelId || model.path === modelId)?.name ?? modelId
}

function startCreate() {
  isCreating.value = true
  editingId.value = null
  formName.value = ''
  formModelId.value = selectableModels.value[0]?.path ?? ''
  formMotionIds.value = []
  formStartMotionId.value = 'appearing'
  formExpressionEnabled.value = true
  formMotionEnabled.value = true
}

function startEdit(appearance: VrmAppearanceConfig) {
  if (isDefaultAppearanceId(appearance.id)) return
  isCreating.value = false
  editingId.value = appearance.id
  formName.value = appearance.name
  formModelId.value = appearance.modelId
  formMotionIds.value = [...appearance.motionIds]
  formStartMotionId.value = appearance.startMotionId || 'appearing'
  formExpressionEnabled.value = appearance.expressionEnabled
  formMotionEnabled.value = appearance.motionEnabled
}

function cancelForm() {
  isCreating.value = false
  editingId.value = null
}

function toggleMotion(id: string) {
  formMotionIds.value = formMotionIds.value.includes(id)
    ? formMotionIds.value.filter((motionId) => motionId !== id)
    : [...formMotionIds.value, id]
}

function saveForm() {
  if (!formModelId.value) return

  const payload = {
    name: formName.value.trim() || '未命名外观',
    modelId: formModelId.value,
    motionIds: [...formMotionIds.value],
    startMotionId: formStartMotionId.value,
    expressionEnabled: formExpressionEnabled.value,
    motionEnabled: formMotionEnabled.value,
  }

  if (isCreating.value) {
    emit('createAppearance', {
      id: `appearance-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      ...payload,
    })
  } else if (editingId.value) {
    emit('updateAppearance', editingId.value, payload)
  }
  cancelForm()
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="border-b border-gray-100 px-4 py-3">
      <button
        type="button"
        class="flex w-full items-center justify-center gap-1.5 rounded-lg border border-pink-200 bg-pink-50 px-3 py-2 text-xs font-medium text-pink-600 transition-colors hover:bg-pink-100 disabled:cursor-not-allowed disabled:opacity-50"
        :disabled="selectableModels.length === 0"
        @click="startCreate"
      >
        <Plus :size="14" />
        <span>新建外观</span>
      </button>
      <p v-if="selectableModels.length === 0" class="mt-2 text-[10px] text-amber-600">
        请先在“VRM 模型”页加载或导入模型。
      </p>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto">
      <div v-if="isCreating || editingId" class="border-b border-gray-100 bg-pink-50/30 px-4 py-3">
        <div class="mb-3 flex items-center justify-between">
          <span class="text-xs font-semibold text-gray-700">{{ isCreating ? '新建外观' : '编辑外观' }}</span>
          <button class="text-gray-400 hover:text-gray-600" type="button" @click="cancelForm">
            <X :size="14" />
          </button>
        </div>

        <label class="mb-3 block">
          <span class="mb-1 block text-[10px] text-gray-500">名称</span>
          <input
            v-model="formName"
            class="w-full rounded-md border border-gray-200 px-2.5 py-1.5 text-xs text-gray-800 outline-none focus:border-pink-300 focus:ring-1 focus:ring-pink-200"
            placeholder="输入外观名称"
          />
        </label>

        <label class="mb-3 block">
          <span class="mb-1 block text-[10px] text-gray-500">角色模型</span>
          <select
            v-model="formModelId"
            class="w-full rounded-md border border-gray-200 bg-white px-2.5 py-1.5 text-xs text-gray-800 outline-none focus:border-pink-300 focus:ring-1 focus:ring-pink-200"
          >
            <option v-for="model in selectableModels" :key="model.path" :value="model.path">
              {{ model.name }}{{ model.source === 'custom' ? '（自定义）' : '' }}
            </option>
          </select>
        </label>

        <div class="mb-3">
          <div class="mb-1 text-[10px] text-gray-500">待机动作</div>
          <div class="max-h-28 space-y-1 overflow-y-auto">
            <button
              v-for="motion in idleMotions"
              :key="motion.id"
              type="button"
              class="flex w-full items-center gap-2 rounded px-1.5 py-1 text-left text-xs text-gray-600 hover:bg-white/70"
              @click="toggleMotion(motion.id)"
            >
              <span
                class="flex h-4 w-4 items-center justify-center rounded border"
                :class="formMotionIds.includes(motion.id) ? 'border-pink-500 bg-pink-500 text-white' : 'border-gray-300'"
              >
                <Check v-if="formMotionIds.includes(motion.id)" :size="11" />
              </span>
              <span class="truncate">{{ motion.name }}</span>
            </button>
            <p v-if="idleMotions.length === 0" class="text-[10px] text-gray-400">暂无可用待机动作</p>
          </div>
        </div>

        <div class="mb-3">
          <div class="mb-1 text-[10px] text-gray-500">开始动作</div>
          <div class="grid grid-cols-2 gap-1">
            <button
              v-for="motion in startupMotions"
              :key="motion.id"
              type="button"
              class="flex items-center justify-center rounded-md border px-2 py-1.5 text-xs transition-colors"
              :class="formStartMotionId === motion.id ? 'border-pink-400 bg-pink-50 text-pink-600' : 'border-gray-200 bg-white text-gray-600 hover:bg-gray-50'"
              @click="formStartMotionId = motion.id"
            >
              {{ motion.name }}
            </button>
          </div>
          <p v-if="startupMotions.length === 0" class="mt-1 text-[10px] text-gray-400">暂无可用开始动作，保存时默认使用 appearing</p>
        </div>

        <div class="mb-3 grid grid-cols-2 gap-2 text-xs text-gray-600">
          <label class="flex items-center gap-2">
            <input v-model="formMotionEnabled" type="checkbox" class="rounded border-gray-300 text-pink-500" />
            <span>启用动画</span>
          </label>
          <label class="flex items-center gap-2">
            <input v-model="formExpressionEnabled" type="checkbox" class="rounded border-gray-300 text-pink-500" />
            <span>启用表情</span>
          </label>
        </div>

        <button
          type="button"
          class="w-full rounded-md bg-pink-500 px-3 py-1.5 text-xs font-medium text-white hover:bg-pink-600 disabled:cursor-not-allowed disabled:opacity-50"
          :disabled="!formModelId"
          @click="saveForm"
        >
          保存
        </button>
      </div>

      <div class="py-1">
        <div
          v-for="appearance in displayedAppearances"
          :key="appearance.id"
          class="border-b border-gray-50 px-4 py-2.5 last:border-b-0"
          :class="effectiveActiveId === appearance.id ? 'bg-pink-50/50' : 'hover:bg-gray-50'"
        >
          <div class="flex gap-2.5">
            <div class="pt-0.5">
              <div v-if="effectiveActiveId === appearance.id" class="flex h-4 w-4 items-center justify-center rounded-full bg-pink-500">
                <div class="h-1.5 w-1.5 rounded-full bg-white" />
              </div>
              <Circle v-else :size="16" class="text-gray-300" />
            </div>
            <div class="min-w-0 flex-1">
              <div class="flex items-start justify-between gap-2">
                <div class="min-w-0">
                  <div class="truncate text-xs font-semibold text-gray-800">{{ appearance.name }}</div>
                  <div class="mt-0.5 truncate text-[10px] text-gray-400">
                    {{ modelName(appearance.modelId) }} · {{ appearance.motionIds.length }} 个待机动作
                  </div>
                </div>
                <div v-if="!isDefaultAppearanceId(appearance.id)" class="flex shrink-0 gap-1">
                  <button type="button" aria-label="编辑外观" class="text-gray-400 hover:text-gray-600" @click="startEdit(appearance)">
                    <Pencil :size="13" />
                  </button>
                  <button type="button" aria-label="删除外观" class="text-gray-400 hover:text-red-500" @click="emit('deleteAppearance', appearance.id)">
                    <Trash2 :size="13" />
                  </button>
                </div>
              </div>
              <button
                v-if="effectiveActiveId !== appearance.id"
                type="button"
                class="mt-1.5 rounded border border-pink-200 px-2 py-0.5 text-[10px] text-pink-500 hover:bg-pink-50"
                @click="emit('switchAppearance', appearance.id)"
              >
                应用
              </button>
              <span v-else class="mt-1.5 inline-flex items-center gap-1 text-[10px] font-medium text-pink-500">
                <Check :size="11" />
                当前
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

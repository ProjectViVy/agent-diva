<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Check, Clapperboard, Loader2, PackageOpen, Play, Square } from '@lucide/vue'
import type { VrmMotionInfo } from '../types'

interface Props {
  visible: boolean
  motionList: VrmMotionInfo[]
  selectedMotionIds: string[]
  vrmMotionEnabled: boolean
  modelLoaded: boolean
  inline?: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:selectedMotionIds', ids: string[]): void
  (e: 'update:vrmMotionEnabled', val: boolean): void
  (e: 'previewMotion', id: string): void
  (e: 'stopPreview'): void
}>()

const previewingMotionId = ref<string | null>(null)
const idleMotions = computed(() => props.motionList.filter((motion) => motion.kind === 'idle'))
const startupMotions = computed(() => props.motionList.filter((motion) => motion.kind === 'startup'))
const oneShotMotions = computed(() => props.motionList.filter((motion) => motion.kind === 'oneshot'))

function isSelected(id: string): boolean {
  return props.selectedMotionIds.includes(id)
}

function toggleMotion(id: string) {
  const next = isSelected(id)
    ? props.selectedMotionIds.filter((motionId) => motionId !== id)
    : [...props.selectedMotionIds, id]
  emit('update:selectedMotionIds', next)
}

function preview(id: string) {
  if (previewingMotionId.value === id) {
    stopPreview()
    return
  }
  previewingMotionId.value = id
  emit('previewMotion', id)
}

function stopPreview() {
  previewingMotionId.value = null
  emit('stopPreview')
}

watch(
  () => props.visible,
  (visible) => {
    if (!visible) {
      previewingMotionId.value = null
    }
  },
)
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="border-b border-border px-4 py-3">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <Clapperboard :size="15" class="text-primary" />
          <div>
            <div class="text-xs font-semibold text-foreground">待机动画</div>
            <div class="text-[10px] text-muted-foreground">控制当前角色的循环动作</div>
          </div>
        </div>
        <button
          type="button"
          role="switch"
          :aria-checked="vrmMotionEnabled"
          :disabled="!modelLoaded"
          class="relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors"
          :class="[
            vrmMotionEnabled ? 'bg-success' : 'bg-secondary',
            !modelLoaded ? 'cursor-not-allowed opacity-40' : 'cursor-pointer',
          ]"
          @click="emit('update:vrmMotionEnabled', !vrmMotionEnabled)"
        >
          <span
            class="inline-block h-4 w-4 rounded-full bg-card shadow-sm transition-transform"
            :class="vrmMotionEnabled ? 'translate-x-[18px]' : 'translate-x-[2px]'"
          />
        </button>
      </div>
    </div>

    <div v-if="!modelLoaded" class="flex flex-1 flex-col items-center justify-center px-4 py-12 text-muted-foreground">
      <Loader2 :size="28" class="mb-2 animate-spin" />
      <p class="text-xs">模型加载后可管理动画</p>
    </div>

    <div v-else-if="motionList.length === 0" class="flex flex-1 flex-col items-center justify-center px-4 py-12 text-muted-foreground">
      <PackageOpen :size="28" class="mb-2 text-muted-foreground" />
      <p class="text-xs">未找到可播放动作</p>
    </div>

    <div v-else class="flex-1 overflow-y-auto">
      <section class="border-b border-border px-4 py-3">
        <div class="mb-2 text-xs font-semibold text-foreground">待机动作集合</div>
        <div class="space-y-1">
          <button
            v-for="motion in idleMotions"
            :key="motion.id"
            type="button"
            class="ui-button flex w-full items-center justify-start gap-2 text-left transition-colors "
            :class="isSelected(motion.id) ? 'text-primary' : 'text-muted-foreground'"
            @click="toggleMotion(motion.id)"
          >
            <span
              class="flex h-4 w-4 shrink-0 items-center justify-center rounded border"
              :class="isSelected(motion.id) ? 'border-border-strong bg-primary text-primary-foreground' : 'border-border'"
            >
              <Check v-if="isSelected(motion.id)" :size="11" />
            </span>
            <span class="truncate">{{ motion.name }}</span>
          </button>
        </div>
        <p class="mt-2 text-[10px] text-muted-foreground">未选择时会使用全部内置待机动作。</p>
      </section>

      <section class="px-4 py-3">
        <div class="mb-2 text-xs font-semibold text-foreground">动作预览</div>
        <div class="space-y-1">
          <div class="mb-2 text-xs font-semibold text-foreground">开始动作</div>
          <div
            v-for="motion in startupMotions"
            :key="motion.id"
            class="flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-accent"
          >
            <div class="min-w-0 flex-1">
              <div class="truncate text-xs font-medium text-foreground">{{ motion.name }}</div>
              <div class="truncate text-[10px] text-muted-foreground">{{ motion.path }}</div>
            </div>
            <button
              type="button"
              class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full transition-colors"
              :class="previewingMotionId === motion.id ? 'bg-destructive-soft text-destructive' : 'text-muted-foreground hover:bg-accent hover:text-primary'"
              @click="preview(motion.id)"
            >
              <Square v-if="previewingMotionId === motion.id" :size="13" />
              <Play v-else :size="13" />
            </button>
          </div>
          <div class="mt-3 mb-2 text-xs font-semibold text-foreground">单次动作</div>
          <div
            v-for="motion in oneShotMotions"
            :key="motion.id"
            class="flex items-center gap-2 rounded-md px-2 py-1.5 hover:bg-accent"
          >
            <div class="min-w-0 flex-1">
              <div class="truncate text-xs font-medium text-foreground">{{ motion.name }}</div>
              <div class="truncate text-[10px] text-muted-foreground">{{ motion.path }}</div>
            </div>
            <button
              type="button"
              class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full transition-colors"
              :class="previewingMotionId === motion.id ? 'bg-destructive-soft text-destructive' : 'text-muted-foreground hover:bg-accent hover:text-primary'"
              @click="preview(motion.id)"
            >
              <Square v-if="previewingMotionId === motion.id" :size="13" />
              <Play v-else :size="13" />
            </button>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

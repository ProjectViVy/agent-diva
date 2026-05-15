<script setup lang="ts">
import { ref, watch } from 'vue'
import {
  X,
  Clapperboard,
  Play,
  Square,
  Loader2,
  PackageOpen,
} from 'lucide-vue-next'
import type { VrmMotionInfo } from '../types'

// ── Tauri availability (read-only, no Tauri API imports) ──
const isTauri = typeof window !== 'undefined' && ('__TAURI_INTERNALS__' in window || '__TAURI__' in window)

// ── Props ──
interface Props {
  visible: boolean
  motionList: VrmMotionInfo[]
  selectedMotionIds: string[]
  vrmMotionEnabled: boolean
  modelLoaded: boolean
  inline?: boolean
}

const props = defineProps<Props>()

// ── Emits ──
const emit = defineEmits<{
  (e: 'close'): void
  (e: 'update:selectedMotionIds', ids: string[]): void
  (e: 'update:vrmMotionEnabled', val: boolean): void
  (e: 'previewMotion', id: string): void
  (e: 'stopPreview'): void
  (e: 'update:animationMode', mode: string): void
}>()

// ── Animation mode ──
type AnimationMode = 'vrma' | 'procedural' | 'off'
const animationMode = ref<AnimationMode>('off')

type ModeOption = { value: AnimationMode; label: string }

const modeOptions: ModeOption[] = [
  { value: 'vrma', label: 'VRMA 动画' },
  { value: 'procedural', label: '程序化' },
  { value: 'off', label: '关闭' },
]

function onModeChange(mode: AnimationMode) {
  animationMode.value = mode
  emit('update:animationMode', mode)
}

// ── Preview state ──
const previewingMotionId = ref<string | null>(null)

function onPreviewMotion(id: string) {
  previewingMotionId.value = id
  emit('previewMotion', id)
}

function onStopPreview() {
  previewingMotionId.value = null
  emit('stopPreview')
}

// ── Motion selection ──
function isMotionSelected(id: string): boolean {
  return props.selectedMotionIds.includes(id)
}

function toggleMotion(id: string) {
  const current = [...props.selectedMotionIds]
  const index = current.indexOf(id)
  if (index >= 0) {
    current.splice(index, 1)
  } else {
    current.push(id)
  }
  emit('update:selectedMotionIds', current)
}

// ── Close ──
function handleClose() {
  emit('close')
}

// ── Reset preview state on panel close ──
watch(
  () => props.visible,
  (val) => {
    if (!val) {
      previewingMotionId.value = null
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
          <!-- Header -->
          <div class="flex items-center justify-between px-4 py-3 border-b border-gray-100">
            <div class="flex items-center gap-2">
              <Clapperboard :size="16" class="text-pink-500" />
              <span class="text-sm font-semibold text-gray-800">VRM Animations</span>
            </div>
            <button
              class="w-7 h-7 flex items-center justify-center rounded-full text-gray-400 hover:text-gray-600 hover:bg-gray-100 transition-colors"
              @click="handleClose"
            >
              <X :size="16" />
            </button>
          </div>

          <!-- Section 1: Idle Animation Toggle -->
          <div class="px-4 py-3 border-b border-gray-50">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <span
                  class="w-1.5 h-1.5 rounded-full"
                  :class="modelLoaded && vrmMotionEnabled ? 'bg-green-400' : 'bg-gray-300'"
                />
                <span class="text-xs font-medium text-gray-700">闲置动画</span>
                <span
                  v-if="modelLoaded && vrmMotionEnabled"
                  class="text-[10px] text-green-500 font-medium"
                >
                  运行中
                </span>
              </div>
              <!-- Toggle switch -->
              <button
                type="button"
                role="switch"
                :aria-checked="vrmMotionEnabled"
                :disabled="!modelLoaded"
                class="relative inline-flex items-center shrink-0 transition-colors rounded-full"
                :class="[
                  'w-9 h-5',
                  vrmMotionEnabled ? 'bg-green-500' : 'bg-gray-200',
                  !modelLoaded ? 'opacity-40 cursor-not-allowed' : 'cursor-pointer',
                ]"
                @click="emit('update:vrmMotionEnabled', !vrmMotionEnabled)"
              >
                <span
                  class="inline-block w-4 h-4 bg-white rounded-full shadow-sm transition-transform"
                  :class="vrmMotionEnabled ? 'translate-x-[18px]' : 'translate-x-[2px]'"
                />
              </button>
            </div>
          </div>

          <!-- Section 2: Animation Mode -->
          <div class="px-4 py-3 border-b border-gray-50">
            <span class="text-xs font-medium text-gray-700 block mb-2">动画模式</span>
            <div class="flex rounded-lg border border-gray-200 overflow-hidden">
              <button
                v-for="mode in modeOptions"
                :key="mode.value"
                class="flex-1 px-2 py-1.5 text-xs font-medium border-r border-gray-200 last:border-r-0 transition-colors"
                :class="
                  animationMode === mode.value
                    ? 'bg-pink-50 text-pink-600'
                    : 'text-gray-500 hover:bg-gray-50'
                "
                @click="onModeChange(mode.value)"
              >
                {{ mode.label }}
              </button>
            </div>
          </div>

          <!-- Section 3: Motion List -->
          <div class="flex-1 overflow-y-auto">
            <!-- Not loaded state -->
            <div v-if="!modelLoaded" class="flex flex-col items-center justify-center py-12 px-4">
              <Loader2 :size="28" class="text-gray-300 mb-2 animate-spin" />
              <p class="text-xs text-gray-400 text-center">
                模型未加载，动画不可用
              </p>
            </div>

            <!-- Empty state -->
            <div
              v-else-if="motionList.length === 0"
              class="flex flex-col items-center justify-center py-12 px-4"
            >
              <PackageOpen :size="28" class="text-gray-300 mb-2" />
              <p class="text-xs text-gray-400 text-center">
                No VRMA animations found
              </p>
              <p class="text-[10px] text-gray-300 text-center mt-1">
                Place .vrma files in the animations directory.
              </p>
            </div>

            <!-- Motion entries -->
            <div v-else class="py-1">
              <div
                v-for="motion in motionList"
                :key="motion.id"
                class="flex items-center gap-2.5 px-4 py-2.5 border-b border-gray-50 last:border-b-0 hover:bg-pink-50/30 transition-colors"
              >
                <!-- Checkbox -->
                <button
                  class="w-4 h-4 rounded border flex items-center justify-center shrink-0 transition-colors"
                  :class="
                    isMotionSelected(motion.id)
                      ? 'bg-pink-500 border-pink-500 text-white'
                      : 'border-gray-300 hover:border-pink-300'
                  "
                  @click="toggleMotion(motion.id)"
                >
                  <svg
                    v-if="isMotionSelected(motion.id)"
                    xmlns="http://www.w3.org/2000/svg"
                    class="w-2.5 h-2.5"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="3"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                </button>

                <!-- Motion info -->
                <div class="flex-1 min-w-0">
                  <div class="text-xs font-medium text-gray-800 truncate">
                    {{ motion.name }}
                  </div>
                  <div class="text-[10px] text-gray-400 truncate mt-0.5">
                    {{ motion.path }}
                  </div>
                </div>

                <!-- Preview button -->
                <button
                  class="w-7 h-7 flex items-center justify-center rounded-full shrink-0 transition-colors"
                  :class="
                    previewingMotionId === motion.id
                      ? 'bg-red-50 text-red-500 hover:bg-red-100'
                      : 'text-gray-400 hover:text-pink-500 hover:bg-pink-50'
                  "
                  @click="
                    previewingMotionId === motion.id ? onStopPreview() : onPreviewMotion(motion.id)
                  "
                >
                  <Square v-if="previewingMotionId === motion.id" :size="13" />
                  <Play v-else :size="13" />
                </button>
              </div>
            </div>
          </div>

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

  <!-- Inline mode: inner content only, no backdrop/header/footer/fixed positioning -->
  <div v-else class="flex flex-col h-full">
    <!-- Section 1: Idle Animation Toggle -->
    <div class="px-4 py-3 border-b border-gray-50">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <span
            class="w-1.5 h-1.5 rounded-full"
            :class="modelLoaded && vrmMotionEnabled ? 'bg-green-400' : 'bg-gray-300'"
          />
          <span class="text-xs font-medium text-gray-700">闲置动画</span>
          <span
            v-if="modelLoaded && vrmMotionEnabled"
            class="text-[10px] text-green-500 font-medium"
          >
            运行中
          </span>
        </div>
        <!-- Toggle switch -->
        <button
          type="button"
          role="switch"
          :aria-checked="vrmMotionEnabled"
          :disabled="!modelLoaded"
          class="relative inline-flex items-center shrink-0 transition-colors rounded-full"
          :class="[
            'w-9 h-5',
            vrmMotionEnabled ? 'bg-green-500' : 'bg-gray-200',
            !modelLoaded ? 'opacity-40 cursor-not-allowed' : 'cursor-pointer',
          ]"
          @click="emit('update:vrmMotionEnabled', !vrmMotionEnabled)"
        >
          <span
            class="inline-block w-4 h-4 bg-white rounded-full shadow-sm transition-transform"
            :class="vrmMotionEnabled ? 'translate-x-[18px]' : 'translate-x-[2px]'"
          />
        </button>
      </div>
    </div>

    <!-- Section 2: Animation Mode -->
    <div class="px-4 py-3 border-b border-gray-50">
      <span class="text-xs font-medium text-gray-700 block mb-2">动画模式</span>
      <div class="flex rounded-lg border border-gray-200 overflow-hidden">
        <button
          v-for="mode in modeOptions"
          :key="mode.value"
          class="flex-1 px-2 py-1.5 text-xs font-medium border-r border-gray-200 last:border-r-0 transition-colors"
          :class="
            animationMode === mode.value
              ? 'bg-pink-50 text-pink-600'
              : 'text-gray-500 hover:bg-gray-50'
          "
          @click="onModeChange(mode.value)"
        >
          {{ mode.label }}
        </button>
      </div>
    </div>

    <!-- Section 3: Motion List -->
    <div class="flex-1 overflow-y-auto">
      <!-- Not loaded state -->
      <div v-if="!modelLoaded" class="flex flex-col items-center justify-center py-12 px-4">
        <Loader2 :size="28" class="text-gray-300 mb-2 animate-spin" />
        <p class="text-xs text-gray-400 text-center">
          模型未加载，动画不可用
        </p>
      </div>

      <!-- Empty state -->
      <div
        v-else-if="motionList.length === 0"
        class="flex flex-col items-center justify-center py-12 px-4"
      >
        <PackageOpen :size="28" class="text-gray-300 mb-2" />
        <p class="text-xs text-gray-400 text-center">
          No VRMA animations found
        </p>
        <p class="text-[10px] text-gray-300 text-center mt-1">
          Place .vrma files in the animations directory.
        </p>
      </div>

      <!-- Motion entries -->
      <div v-else class="py-1">
        <div
          v-for="motion in motionList"
          :key="motion.id"
          class="flex items-center gap-2.5 px-4 py-2.5 border-b border-gray-50 last:border-b-0 hover:bg-pink-50/30 transition-colors"
        >
          <!-- Checkbox -->
          <button
            class="w-4 h-4 rounded border flex items-center justify-center shrink-0 transition-colors"
            :class="
              isMotionSelected(motion.id)
                ? 'bg-pink-500 border-pink-500 text-white'
                : 'border-gray-300 hover:border-pink-300'
            "
            @click="toggleMotion(motion.id)"
          >
            <svg
              v-if="isMotionSelected(motion.id)"
              xmlns="http://www.w3.org/2000/svg"
              class="w-2.5 h-2.5"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="3"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="20 6 9 17 4 12" />
            </svg>
          </button>

          <!-- Motion info -->
          <div class="flex-1 min-w-0">
            <div class="text-xs font-medium text-gray-800 truncate">
              {{ motion.name }}
            </div>
            <div class="text-[10px] text-gray-400 truncate mt-0.5">
              {{ motion.path }}
            </div>
          </div>

          <!-- Preview button -->
          <button
            class="w-7 h-7 flex items-center justify-center rounded-full shrink-0 transition-colors"
            :class="
              previewingMotionId === motion.id
                ? 'bg-red-50 text-red-500 hover:bg-red-100'
                : 'text-gray-400 hover:text-pink-500 hover:bg-pink-50'
            "
            @click="
              previewingMotionId === motion.id ? onStopPreview() : onPreviewMotion(motion.id)
            "
          >
            <Square v-if="previewingMotionId === motion.id" :size="13" />
            <Play v-else :size="13" />
          </button>
        </div>
      </div>
    </div>
  </div>
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

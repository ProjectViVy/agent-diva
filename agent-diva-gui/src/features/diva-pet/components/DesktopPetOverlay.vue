<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { Mic } from 'lucide-vue-next'
import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import DivaVrmAvatar from '../vrm/components/DivaVrmAvatar.vue'
import { usePetConfig } from '../services/pet-config'
import { useVoiceInput } from '../voice/composables/useVoiceInput'
import { ttsService, type TTSVoiceConfig } from '../voice/services/tts-service'
import { filterPunctuation, splitIntoSentences, stripMarkdown } from '../voice/utils/text-preprocessor'
import type { GaussSceneId, VrmMood } from '../types'
import { normalizeMood } from '../utils/mood'
import { resolveVrmModelPath } from '../utils/vrm-model'
import {
  DEFAULT_VRM_MODEL_PATH,
  resolveAppearance,
  withDefaultAppearance,
} from '../utils/default-appearance'
import { useSubtitleOverlay } from './subtitle-overlay'
import { getTtsApiKey } from '../types'

interface DivaVrmAvatarHandle {
  setScale(scale: number): void
  getScale(): number
}

const { config: petConfig } = usePetConfig()
const { subtitle, startDrag: startSubtitleDrag, onDrag: onSubtitleDrag, endDrag: endSubtitleDrag } = useSubtitleOverlay()

/** Build a TTSVoiceConfig from the reactive pet config (same pattern as DivaPetView.vue). */
const voiceConfig = computed<TTSVoiceConfig>(() => ({
  enabled: petConfig.value.ttsEnabled,
  provider: petConfig.value.ttsProvider,
  apiKey: getTtsApiKey(petConfig.value),
  baseUrl: petConfig.value.ttsBaseUrl,
  model: petConfig.value.ttsModel,
  voiceId: petConfig.value.ttsVoiceId,
  referenceVoice: petConfig.value.ttsReferenceVoice,
  referenceText: petConfig.value.ttsReferenceText,
  speed: petConfig.value.ttsSpeed,
  volume: petConfig.value.ttsVolume,
}))

const vrmModelPath = ref(resolveVrmModelPath(petConfig.value.vrmModel))
let resolveModelRequestId = 0

async function refreshVrmModelPath(model: string) {
  const requestId = ++resolveModelRequestId
  const resolved = resolveVrmModelPath(model)
  if (!resolved.startsWith('vrm/models/custom/')) {
    vrmModelPath.value = resolved
    return
  }

  try {
    const data = await invoke<{
      base64Data: string
      contentType: string
    }>('pet_read_vrm_model', { relativePath: resolved })
    if (requestId === resolveModelRequestId) {
      vrmModelPath.value = `data:${data.contentType};base64,${data.base64Data}`
    }
  } catch (error) {
    console.warn('[DesktopPetOverlay] Failed to read custom VRM model:', error)
    if (requestId === resolveModelRequestId) {
      vrmModelPath.value = DEFAULT_VRM_MODEL_PATH
    }
  }
}

// ── 3D Gaussian Splatting: 背景场景 ────────────────────────────
// Desktop-pet window should always stay transparent. Embedded main-page view
// owns the selectable gaussian background scene.
const backgroundSceneId = computed<GaussSceneId>(() => 'transparent')
const backgroundSceneUrl = computed(() => undefined)

// ── State ──────────────────────────────────────────────────────

const isDragMode = ref(false)
const isMousePassThrough = ref(false)
const contextMenu = ref<{ x: number; y: number } | null>(null)
const isRenderActive = ref(true)
const vrmAvatarRef = ref<DivaVrmAvatarHandle | null>(null)
const subtitleRef = ref<HTMLDivElement | null>(null)
const activeMood = ref<VrmMood>('neutral')
let moodResetTimeoutId: ReturnType<typeof setTimeout> | null = null

// ── PTT (Push-To-Talk) ──────────────────────────────────────────

const voiceInput = useVoiceInput({
  config: computed(() => ({
    provider: petConfig.value.asrProvider,
    language: petConfig.value.asrLanguage,
    apiKey: petConfig.value.asrApiKey,
    baseUrl: petConfig.value.asrBaseUrl,
    model: petConfig.value.asrModel,
  })),
  onRecognizedText: async (text: string) => {
    const trimmed = text.trim()
    if (!trimmed) return
    await emitTo('main', 'desktop-pet-voice-message', trimmed)
  },
})

const isPttDisabled = computed(() => !petConfig.value.asrEnabled || !voiceInput.isSupported)

async function startRecording(event: PointerEvent): Promise<void> {
  event.preventDefault()
  if (isPttDisabled.value || voiceInput.isEnabled.value) return
  await voiceInput.setEnabled(true)
}

function stopRecording(_event?: Event): void {
  if (!voiceInput.isEnabled.value) return
  void voiceInput.setEnabled(false)
}

// ── Wheel zoom ──────────────────────────────────────────────────

const SCALE_MIN = 0.75
const SCALE_MAX = 1.6
const WHEEL_DELTA_STEP = 0.05
const desktopPetScale = ref(petConfig.value.desktopPetScale ?? 1.0)

function handleWheel(event: WheelEvent): void {
  // Block wheel zoom when click-through or drag mode is active
  if (isMousePassThrough.value || isDragMode.value) return

  event.preventDefault()

  const delta = event.deltaY > 0 ? -WHEEL_DELTA_STEP : WHEEL_DELTA_STEP
  const newScale = Math.max(SCALE_MIN, Math.min(SCALE_MAX, desktopPetScale.value + delta))

  desktopPetScale.value = newScale
  vrmAvatarRef.value?.setScale(newScale)
  petConfig.value.desktopPetScale = newScale  // 持久化
}

// ── Mood reset ─────────────────────────────────────────────────

function scheduleMoodReset(mood: VrmMood) {
  if (moodResetTimeoutId !== null) {
    clearTimeout(moodResetTimeoutId)
    moodResetTimeoutId = null
  }
  if (mood === 'neutral') {
    return
  }
  moodResetTimeoutId = setTimeout(() => {
    moodResetTimeoutId = null
    activeMood.value = 'neutral'
  }, 4000)
}

// ── Context menu ───────────────────────────────────────────────

let menuHideTimer: ReturnType<typeof setTimeout> | null = null
const MENU_AUTO_HIDE_MS = 3000

function resetMenuHideTimer(): void {
  if (menuHideTimer !== null) {
    clearTimeout(menuHideTimer)
    menuHideTimer = null
  }
  if (!contextMenu.value) return

  menuHideTimer = setTimeout(() => {
    menuHideTimer = null
    hideContextMenu()
  }, MENU_AUTO_HIDE_MS)
}

function showContextMenu(event: MouseEvent) {
  if (isDragMode.value) {
    exitDragMode()
  }
  contextMenu.value = { x: event.clientX, y: event.clientY }
  resetMenuHideTimer()
}

function hideContextMenu() {
  if (menuHideTimer !== null) {
    clearTimeout(menuHideTimer)
    menuHideTimer = null
  }
  contextMenu.value = null
}

const menuStyle = computed(() => {
  if (!contextMenu.value) return {}
  const menuWidth = 180
  const menuHeight = 420
  return {
    left: Math.min(contextMenu.value.x, window.innerWidth - menuWidth) + 'px',
    top: Math.min(contextMenu.value.y, window.innerHeight - menuHeight) + 'px',
  }
})

// ── Drag mode ──────────────────────────────────────────────────

function enterDragMode() {
  isDragMode.value = true
  contextMenu.value = null
}

function exitDragMode() {
  isDragMode.value = false
}

async function onDragPointerDown(event: PointerEvent) {
  if (!isDragMode.value) return
  if (event.button !== 0) return
  await getCurrentWindow().startDragging()
  exitDragMode()
}

// ── Pass-through toggle ────────────────────────────────────────

async function togglePassThrough() {
  isMousePassThrough.value = !isMousePassThrough.value
  try {
    await invoke('set_desktop_pet_ignore_mouse', { ignore: isMousePassThrough.value })
  } catch (_) {
    isMousePassThrough.value = !isMousePassThrough.value
  }
  contextMenu.value = null
}

// ── Close ──────────────────────────────────────────────────────

async function closePet() {
  try {
    await invoke('close_desktop_pet')
  } catch (_) {}
  contextMenu.value = null
}

// ── Menu: 子菜单 ────────────────────────────────────────────────

type SubmenuKey = 'appearance' | 'animation' | 'voice' | null
const activeSubmenu = ref<SubmenuKey>(null)

function openSubmenu(key: SubmenuKey) {
  activeSubmenu.value = key
  resetMenuHideTimer()
}

function closeSubmenu(_key: SubmenuKey) {
  // 延迟关闭由子菜单的 @mouseleave 处理
}

// ── Menu: 缩放持久化 ────────────────────────────────────────────

function handleScaleInput(event: Event) {
  const target = event.target as HTMLInputElement
  const value = parseFloat(target.value)
  desktopPetScale.value = value
  vrmAvatarRef.value?.setScale(value)
  petConfig.value.desktopPetScale = value
  resetMenuHideTimer()
}

// ── Menu: 置顶切换 ──────────────────────────────────────────────

const isAlwaysOnTop = ref(petConfig.value.desktopPetAlwaysOnTop ?? true)

async function toggleAlwaysOnTop() {
  const previous = isAlwaysOnTop.value
  isAlwaysOnTop.value = !isAlwaysOnTop.value
  try {
    await invoke('set_desktop_pet_always_on_top', { alwaysOnTop: isAlwaysOnTop.value })
    petConfig.value.desktopPetAlwaysOnTop = isAlwaysOnTop.value
  } catch (_) {
    isAlwaysOnTop.value = previous
  }
  resetMenuHideTimer()
}

// ── Menu: 窗口操作 ──────────────────────────────────────────────

async function showMainWindow() {
  try {
    await invoke('open_desktop_pet')
  } catch (_) {}
  contextMenu.value = null
}

async function minimizePet() {
  try {
    await invoke('minimize_desktop_pet')
  } catch (_) {
    // fallback: 使用 window API
    try {
      await getCurrentWindow().minimize()
    } catch (_) {}
  }
  contextMenu.value = null
}

// ── Menu: 外观/动画/语音 快速设置 ─────────────────────────────────

const vrmAppearances = computed(() => withDefaultAppearance(petConfig.value.vrmAppearances ?? []))
const activeAppearanceId = computed(() =>
  resolveAppearance(petConfig.value.vrmAppearances ?? [], petConfig.value.activeAppearanceId).id,
)
const isVrmMotionEnabled = computed(() => petConfig.value.vrmMotionEnabled)
const isVrmExpressionEnabled = computed(() => petConfig.value.vrmExpressionEnabled)
const isTtsEnabled = computed(() => petConfig.value.ttsEnabled)
const isAsrEnabled = computed(() => petConfig.value.asrEnabled)
const isSubtitleEnabled = computed(() => petConfig.value.subtitleEnabled)
const effectiveMood = computed<VrmMood>(() =>
  isVrmExpressionEnabled.value ? activeMood.value : 'neutral',
)
const activeAppearanceStartMotionId = computed(() =>
  resolveAppearance(petConfig.value.vrmAppearances ?? [], petConfig.value.activeAppearanceId).startMotionId || 'appearing',
)

function selectAppearance(id: string) {
  const appearance = resolveAppearance(petConfig.value.vrmAppearances ?? [], id)
  petConfig.value.activeAppearanceId = appearance.id
  petConfig.value.vrmModel = appearance.modelId
  petConfig.value.selectedMotionIds = [...appearance.motionIds]
  petConfig.value.vrmMotionEnabled = appearance.motionEnabled
  petConfig.value.vrmExpressionEnabled = appearance.expressionEnabled
  activeSubmenu.value = null
  contextMenu.value = null
}

function toggleVrmMotion() {
  petConfig.value.vrmMotionEnabled = !petConfig.value.vrmMotionEnabled
  resetMenuHideTimer()
}

function toggleVrmExpression() {
  petConfig.value.vrmExpressionEnabled = !petConfig.value.vrmExpressionEnabled
  resetMenuHideTimer()
}

function toggleTts() {
  petConfig.value.ttsEnabled = !petConfig.value.ttsEnabled
  resetMenuHideTimer()
}

function toggleAsr() {
  petConfig.value.asrEnabled = !petConfig.value.asrEnabled
  resetMenuHideTimer()
}

function toggleSubtitle() {
  petConfig.value.subtitleEnabled = !petConfig.value.subtitleEnabled
  resetMenuHideTimer()
}

/** Test TTS: read last subtitle text (if any) or use default test phrase. */
let testTtsCancelId = 0
async function testTts() {
  const rawText = subtitle.value.text?.trim()
  const displayText = rawText || '你好，我是 Diva，这是一条语音测试消息。'
  console.log('[DesktopPet] Test TTS triggered. Subtitle text:', JSON.stringify(subtitle.value.text))

  // Force-enable TTS for the test if currently disabled
  const savedEnabled = petConfig.value.ttsEnabled
  if (!savedEnabled) {
    console.log('[DesktopPet] TTS was disabled, temporarily enabling for test.')
    petConfig.value.ttsEnabled = true
  }

  const cancelId = ++testTtsCancelId
  const segments = splitIntoSentences(filterPunctuation(stripMarkdown(displayText)))

  console.log('[DesktopPet] Test TTS segments:', segments.length, segments.map(s => s.text.slice(0, 30)))

  for (const segment of segments) {
    if (cancelId !== testTtsCancelId) return
    try {
      await ttsService.speakText(segment.text, voiceConfig.value)
    } catch (err) {
      console.error('[DesktopPet] Test TTS segment failed:', err)
    }
  }

  // Restore original TTS enabled state
  if (!savedEnabled) {
    petConfig.value.ttsEnabled = false
  }

  activeSubmenu.value = null
  contextMenu.value = null
}

// ── Watchers ───────────────────────────────────────────────────

watch(isMousePassThrough, (ignore) => {
  if (ignore) {
    isDragMode.value = false
    contextMenu.value = null
  }
})

watch(isVrmExpressionEnabled, (enabled) => {
  if (enabled) {
    return
  }
  if (moodResetTimeoutId !== null) {
    clearTimeout(moodResetTimeoutId)
    moodResetTimeoutId = null
  }
  activeMood.value = 'neutral'
})

watch(
  () => petConfig.value.vrmModel,
  (model) => {
    void refreshVrmModelPath(model)
  },
  { immediate: true },
)

watch(
  () => [petConfig.value.activeAppearanceId, petConfig.value.vrmAppearances] as const,
  () => {
    const appearance = resolveAppearance(petConfig.value.vrmAppearances ?? [], petConfig.value.activeAppearanceId)
    if (
      petConfig.value.activeAppearanceId !== appearance.id ||
      petConfig.value.vrmModel !== appearance.modelId
    ) {
      petConfig.value.activeAppearanceId = appearance.id
      petConfig.value.vrmModel = appearance.modelId
      petConfig.value.selectedMotionIds = [...appearance.motionIds]
      petConfig.value.vrmMotionEnabled = appearance.motionEnabled
      petConfig.value.vrmExpressionEnabled = appearance.expressionEnabled
    }
  },
  { deep: true, immediate: true },
)

/**
 * TTS auto-play for desktop pet subtitles.
 *
 * When a non-empty subtitle appears, preprocess the text (strip markdown,
 * split into sentences) and speak each sentence sequentially.  A newer
 * subtitle cancels any in-progress speech from the previous one.
 * TTS errors are silently ignored so subtitle display is never blocked.
 */
let subtitleTtsId = 0
watch(
  () => ({ visible: subtitle.value.visible, text: subtitle.value.text }),
  ({ visible, text }) => {
    console.log('[DesktopPet] Subtitle watcher fired. visible:', visible, 'text length:', text?.length ?? 0, 'ttsEnabled:', voiceConfig.value.enabled)
    if (!visible || !text || !voiceConfig.value.enabled) {
      return
    }

    const cancelId = ++subtitleTtsId
    const segments = splitIntoSentences(filterPunctuation(stripMarkdown(text)))
    console.log('[DesktopPet] TTS auto-play: speaking', segments.length, 'segments')

    void (async () => {
      for (const segment of segments) {
        // Bail out if a newer subtitle arrived while we were speaking
        if (cancelId !== subtitleTtsId) return
        try {
          await ttsService.speakText(segment.text, voiceConfig.value)
        } catch (err) {
          console.error('[DesktopPet] TTS segment failed:', err)
        }
      }
    })()
  },
)

// ── Lifecycle ──────────────────────────────────────────────────

const unlisteners: UnlistenFn[] = []

onMounted(async () => {
  isMousePassThrough.value = false

  // 应用持久化的缩放和置顶状态
  if (desktopPetScale.value !== 1.0) {
    vrmAvatarRef.value?.setScale(desktopPetScale.value)
  }
  isAlwaysOnTop.value = petConfig.value.desktopPetAlwaysOnTop ?? true

  unlisteners.push(await listen<string>('desktop-pet-emotion', (event) => {
    if (!isVrmExpressionEnabled.value) {
      activeMood.value = 'neutral'
      scheduleMoodReset('neutral')
      return
    }
    const mood = normalizeMood(event.payload)
    activeMood.value = mood
    scheduleMoodReset(mood)
  }))
  unlisteners.push(await listen('desktop-pet-close-request', closePet))
  unlisteners.push(await listen('desktop-pet-render-pause', () => {
    isRenderActive.value = false
  }))
  unlisteners.push(await listen('desktop-pet-render-resume', () => {
    isMousePassThrough.value = false
    isRenderActive.value = true
  }))

  window.addEventListener('pointerup', exitDragMode)
  window.addEventListener('pointerup', stopRecording)
  window.addEventListener('pointercancel', stopRecording)
  window.addEventListener('blur', exitDragMode)
  window.addEventListener('blur', stopRecording)
  window.addEventListener('wheel', handleWheel, { passive: false })
  window.addEventListener('pointermove', onSubtitleDrag)
  window.addEventListener('pointerup', endSubtitleDrag)
})

onUnmounted(() => {
  if (moodResetTimeoutId !== null) {
    clearTimeout(moodResetTimeoutId)
    moodResetTimeoutId = null
  }
  stopRecording()
  unlisteners.forEach((fn) => fn())
  window.removeEventListener('pointerup', exitDragMode)
  window.removeEventListener('pointerup', stopRecording)
  window.removeEventListener('pointercancel', stopRecording)
  window.removeEventListener('blur', exitDragMode)
  window.removeEventListener('blur', stopRecording)
  window.removeEventListener('wheel', handleWheel)
  window.removeEventListener('pointermove', onSubtitleDrag)
  window.removeEventListener('pointerup', endSubtitleDrag)
})
</script>

<template>
  <div
    class="desktop-pet-overlay"
    :class="{ 'drag-mode': isDragMode }"
    @contextmenu.prevent="showContextMenu"
    @click="hideContextMenu"
    @pointerdown="onDragPointerDown"
  >
    <DivaVrmAvatar
      ref="vrmAvatarRef"
      :model-path="vrmModelPath"
      :mood="effectiveMood"
      :is-speaking="false"
      :desktop-pet="true"
      :active="isRenderActive"
      :background-scene="backgroundSceneId"
      :background-scene-url="backgroundSceneUrl"
      :idle-motion-enabled="petConfig.vrmMotionEnabled"
      :selected-motion-ids="petConfig.selectedMotionIds"
      :start-motion-id="activeAppearanceStartMotionId"
      :start-motion-token="petConfig.activeAppearanceId"
    />

    <!-- Subtitle overlay -->
    <Transition name="subtitle-fade">
      <div
        v-if="subtitle.visible && isSubtitleEnabled"
        ref="subtitleRef"
        class="subtitle-overlay"
        :style="{
          left: subtitle.position.x + 'px',
          top: subtitle.position.y + 'px',
          cursor: subtitle.isDragging ? 'grabbing' : 'grab',
        }"
        @pointerdown="startSubtitleDrag"
      >
        {{ subtitle.text }}
      </div>
    </Transition>

    <!-- PTT Floating Button -->
    <div
      v-if="!isDragMode && !isMousePassThrough"
      class="ptt-floating-wrapper"
    >
      <button
        class="ptt-btn"
        :class="{ recording: voiceInput.isEnabled.value }"
        :disabled="isPttDisabled"
        @pointerdown.prevent="startRecording"
        @pointerup="stopRecording"
        @pointerleave="stopRecording"
        @pointercancel="stopRecording"
      >
        <div v-if="voiceInput.isEnabled.value" class="ptt-pulse-ring" />
        <Mic :size="20" />
      </button>
      <span class="ptt-hint" :class="{ recording: voiceInput.isEnabled.value }">
        {{ voiceInput.isEnabled.value ? '松开发送' : '按住说话' }}
      </span>
      <Transition name="tooltip-fade">
        <span v-if="voiceInput.error.value" class="ptt-error">
          {{ voiceInput.error.value }}
        </span>
      </Transition>
    </div>

    <Transition name="menu-fade">
      <div
        v-if="contextMenu"
        class="context-menu"
        :style="menuStyle"
        @click.stop
        @contextmenu.prevent
        @mousemove="resetMenuHideTimer"
      >
        <!-- 层级 1: 子菜单 -->
        <div class="menu-item menu-item-has-sub"
             @mouseenter="openSubmenu('appearance')"
             @mouseleave="closeSubmenu('appearance')">
          <span class="menu-label">切换外观</span>
          <span class="menu-arrow">›</span>
          <Transition name="submenu-slide">
            <div v-if="activeSubmenu === 'appearance'" class="submenu">
              <div v-for="app in vrmAppearances" :key="app.id"
                   class="submenu-item"
                   :class="{ active: app.id === activeAppearanceId }"
                   @click="selectAppearance(app.id)">
                {{ app.name }}
              </div>
            </div>
          </Transition>
        </div>

        <div class="menu-item menu-item-has-sub"
             @mouseenter="openSubmenu('animation')"
             @mouseleave="closeSubmenu('animation')">
          <span class="menu-label">动画设置</span>
          <span class="menu-arrow">›</span>
          <Transition name="submenu-slide">
            <div v-if="activeSubmenu === 'animation'" class="submenu">
              <div class="submenu-item submenu-toggle"
                   :class="{ active: isVrmMotionEnabled }"
                   @click="toggleVrmMotion">
                待机动画: {{ isVrmMotionEnabled ? 'ON' : 'OFF' }}
              </div>
              <div class="submenu-item submenu-toggle"
                   :class="{ active: isVrmExpressionEnabled }"
                   @click="toggleVrmExpression">
                表情映射: {{ isVrmExpressionEnabled ? 'ON' : 'OFF' }}
              </div>
            </div>
          </Transition>
        </div>

        <div class="menu-item menu-item-has-sub"
             @mouseenter="openSubmenu('voice')"
             @mouseleave="closeSubmenu('voice')">
          <span class="menu-label">语音设置</span>
          <span class="menu-arrow">›</span>
          <Transition name="submenu-slide">
            <div v-if="activeSubmenu === 'voice'" class="submenu">
              <div class="submenu-item submenu-toggle"
                   :class="{ active: isTtsEnabled }"
                   @click="toggleTts">
                TTS: {{ isTtsEnabled ? 'ON' : 'OFF' }}
              </div>
              <div class="submenu-item submenu-toggle"
                   :class="{ active: isAsrEnabled }"
                   @click="toggleAsr">
                ASR: {{ isAsrEnabled ? 'ON' : 'OFF' }}
              </div>
              <div class="submenu-item submenu-toggle"
                    :class="{ active: isSubtitleEnabled }"
                    @click="toggleSubtitle">
                 字幕显示: {{ isSubtitleEnabled ? 'ON' : 'OFF' }}
               </div>
               <div class="submenu-separator" />
               <div class="submenu-item submenu-item-action"
                    @click="testTts">
                 测试语音
               </div>
            </div>
          </Transition>
        </div>

        <div class="menu-separator" />

        <!-- 层级 2: 开关项 -->
        <div class="menu-item menu-item-toggle" @click="togglePassThrough">
          <span class="menu-label">穿透切换</span>
          <span class="menu-toggle-state" :class="{ on: isMousePassThrough }">
            {{ isMousePassThrough ? 'ON' : 'OFF' }}
          </span>
        </div>

        <div class="menu-item menu-item-toggle" @click="toggleAlwaysOnTop">
          <span class="menu-label">窗口置顶</span>
          <span class="menu-toggle-state" :class="{ on: isAlwaysOnTop }">
            {{ isAlwaysOnTop ? 'ON' : 'OFF' }}
          </span>
        </div>

        <div class="menu-separator" />

        <!-- 层级 3: 缩放滑块 -->
        <div class="menu-item menu-item-slider">
          <span class="menu-label">缩放</span>
          <div class="menu-slider-row">
            <input type="range" class="menu-slider" :min="SCALE_MIN" :max="SCALE_MAX"
                   :step="WHEEL_DELTA_STEP" :value="desktopPetScale"
                   @input="handleScaleInput" @pointerdown.stop />
            <span class="menu-scale-value">{{ Math.round(desktopPetScale * 100) }}%</span>
          </div>
        </div>

        <div class="menu-separator" />

        <!-- 层级 4: 窗口操作 -->
        <div class="menu-item" @click="enterDragMode">
          <span class="menu-label">移动</span>
        </div>

        <div class="menu-separator" />

        <div class="menu-item" @click="showMainWindow">
          <span class="menu-label">显示主窗口</span>
        </div>
        <div class="menu-item" @click="minimizePet">
          <span class="menu-label">最小化</span>
        </div>
        <div class="menu-item menu-item-danger" @click="closePet">
          <span class="menu-label">关闭</span>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.desktop-pet-overlay {
  width: 100%;
  height: 100%;
  position: relative;
  user-select: none;
  overflow: hidden;
  background: transparent;
}

.desktop-pet-overlay.drag-mode {
  cursor: grab;
}

/* ── Context menu ────────────────────────────────────────────── */

.context-menu {
  position: fixed;
  z-index: 50;
  min-width: 140px;
  border-radius: 10px;
  background: rgba(30, 30, 30, 0.88);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  padding: 4px 0;
}

.menu-item {
  padding: 10px 16px;
  font-size: 13px;
  color: rgba(255, 255, 255, 0.85);
  cursor: pointer;
  transition: background 0.12s ease;
  user-select: none;
}

.menu-item:hover {
  background: rgba(255, 255, 255, 0.1);
}

.menu-item + .menu-item {
  border-top: 1px solid rgba(255, 255, 255, 0.06);
}

.menu-item-danger {
  color: #f87171;
}

.menu-item-danger:hover {
  background: rgba(239, 68, 68, 0.15);
}

/* ── Menu: 子菜单增强 ────────────────────────────────────────── */

.menu-label {
  flex: 1;
}

.menu-arrow {
  font-size: 10px;
  opacity: 0.5;
  transition: transform 0.15s ease;
}

.menu-item-has-sub:hover .menu-arrow {
  opacity: 1;
}

/* ── Separator ───────────────────────────────────────────────── */

.menu-separator {
  height: 1px;
  background: rgba(255, 255, 255, 0.08);
  margin: 4px 0;
}

/* ── Submenu ─────────────────────────────────────────────────── */

.menu-item-has-sub {
  position: relative;
  display: flex;
  align-items: center;
}

.submenu {
  position: absolute;
  left: 100%;
  top: -4px;
  min-width: 160px;
  border-radius: 10px;
  background: rgba(30, 30, 30, 0.92);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  padding: 4px 0;
  z-index: 51;
}

.submenu-item {
  padding: 8px 16px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.8);
  cursor: pointer;
  transition: background 0.12s ease;
  user-select: none;
  white-space: nowrap;
}

.submenu-item:hover {
  background: rgba(255, 255, 255, 0.1);
}

.submenu-item.active {
  color: #60a5fa;
  background: rgba(59, 130, 246, 0.1);
}

.submenu-item-disabled {
  opacity: 0.4;
  cursor: default;
}

.submenu-item-disabled:hover {
  background: transparent;
}

.submenu-separator {
  height: 1px;
  background: rgba(255, 255, 255, 0.08);
  margin: 2px 8px;
}

.submenu-item-action {
  color: #60a5fa;
  font-weight: 500;
}

.submenu-item-action:hover {
  background: rgba(59, 130, 246, 0.1);
}

.submenu-toggle {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

/* ── Toggle indicator ────────────────────────────────────────── */

.menu-item-toggle {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.menu-toggle-state {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.4);
  transition: color 0.15s ease;
}

.menu-toggle-state.on {
  color: #34d399;
}

/* ── Scale slider ────────────────────────────────────────────── */

.menu-item-slider {
  display: flex;
  flex-direction: column;
  gap: 6px;
  cursor: default;
  padding: 10px 16px;
}

.menu-slider-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.menu-slider {
  flex: 1;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: rgba(255, 255, 255, 0.15);
  border-radius: 2px;
  outline: none;
  cursor: pointer;
}

.menu-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #60a5fa;
  cursor: pointer;
  border: 2px solid rgba(255, 255, 255, 0.3);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
}

.menu-scale-value {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.6);
  min-width: 36px;
  text-align: right;
  font-variant-numeric: tabular-nums;
}

/* ── Submenu transitions ─────────────────────────────────────── */

.submenu-slide-enter-active {
  transition: opacity 0.12s ease, transform 0.12s ease;
}

.submenu-slide-leave-active {
  transition: opacity 0.08s ease, transform 0.08s ease;
}

.submenu-slide-enter-from {
  opacity: 0;
  transform: translateX(-4px);
}

.submenu-slide-leave-to {
  opacity: 0;
  transform: translateX(-4px);
}

/* ── Subtitle overlay ────────────────────────────────────────── */

.subtitle-overlay {
  position: fixed;
  z-index: 40;
  transform: translate(-50%, -50%);
  padding: 8px 20px;
  border-radius: 20px;
  background: rgba(0, 0, 0, 0.65);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border: 1px solid rgba(255, 255, 255, 0.1);
  color: rgba(255, 255, 255, 0.9);
  font-size: 14px;
  line-height: 1.5;
  max-width: 320px;
  text-align: center;
  user-select: none;
  pointer-events: auto;
  transition: opacity 0.2s ease;
}

.subtitle-overlay:hover {
  background: rgba(0, 0, 0, 0.75);
}

.subtitle-fade-enter-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.subtitle-fade-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.subtitle-fade-enter-from {
  opacity: 0;
  transform: translate(-50%, -50%) translateY(8px);
}

.subtitle-fade-leave-to {
  opacity: 0;
  transform: translate(-50%, -50%) translateY(-4px);
}

/* ── Visual enhancement: matching SAP quality ────────────────── */

.diva-vrm-avatar :deep(canvas) {
  filter: contrast(1.05) brightness(1.02);
}

/* ── Transitions ─────────────────────────────────────────────── */

.menu-fade-enter-active,
.menu-fade-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.menu-fade-enter-from,
.menu-fade-leave-to {
  opacity: 0;
  transform: scale(0.95);
}

/* ── PTT Floating Button ──────────────────────────────────────── */

.ptt-floating-wrapper {
  position: absolute;
  bottom: 20px;
  right: 20px;
  z-index: 30;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}

.ptt-btn {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.8);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
  border: 1px solid rgba(255, 255, 255, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  opacity: 0.7;
  position: relative;
  transition: opacity 0.15s ease, background 0.2s ease, transform 0.15s ease;
  user-select: none;
  color: #ec4899;
}

.ptt-btn:hover {
  opacity: 1;
}

.ptt-btn:disabled {
  cursor: not-allowed;
  opacity: 0.38;
}

.ptt-btn:active {
  transform: scale(0.92);
}

.ptt-btn.recording {
  opacity: 1;
  background: #ec4899;
  color: #ffffff;
  border-color: #ec4899;
  box-shadow: 0 4px 24px rgba(236, 72, 153, 0.5);
}

.ptt-pulse-ring {
  position: absolute;
  inset: -4px;
  border-radius: 50%;
  border: 2px solid rgba(236, 72, 153, 0.5);
  animation: pt-pulse 1.2s ease-out infinite;
  pointer-events: none;
}

@keyframes pt-pulse {
  0% {
    transform: scale(1);
    opacity: 0.6;
  }
  100% {
    transform: scale(1.6);
    opacity: 0;
  }
}

.ptt-hint {
  font-size: 10px;
  color: #9ca3af;
  user-select: none;
  transition: color 0.15s ease;
}

.ptt-hint.recording {
  color: #ec4899;
}

.ptt-error {
  font-size: 10px;
  color: #ef4444;
  white-space: nowrap;
  user-select: none;
  margin-top: 2px;
}

.tooltip-fade-enter-active,
.tooltip-fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.tooltip-fade-enter-from,
.tooltip-fade-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>

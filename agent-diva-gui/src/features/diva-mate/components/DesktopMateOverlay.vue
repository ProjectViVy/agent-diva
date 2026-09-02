<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { Mic } from '@lucide/vue'
import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import DivaVrmAvatar from '../vrm/components/DivaVrmAvatar.vue'
import { useMateConfig } from '../services/mate-config'
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

interface NeuroLinkPresentationPayload {
  event: string
  body?: Record<string, unknown>
  source?: 'live' | 'replay'
}

const { config: mateConfig } = useMateConfig()
const { subtitle, startDrag: startSubtitleDrag, onDrag: onSubtitleDrag, endDrag: endSubtitleDrag } = useSubtitleOverlay()

/** Build a TTSVoiceConfig from the reactive mate config (same pattern as DivaMateView.vue). */
const voiceConfig = computed<TTSVoiceConfig>(() => ({
  enabled: mateConfig.value.ttsEnabled,
  provider: mateConfig.value.ttsProvider,
  apiKey: getTtsApiKey(mateConfig.value),
  baseUrl: mateConfig.value.ttsBaseUrl,
  model: mateConfig.value.ttsModel,
  voiceId: mateConfig.value.ttsVoiceId,
  referenceVoice: mateConfig.value.ttsReferenceVoice,
  referenceText: mateConfig.value.ttsReferenceText,
  speed: mateConfig.value.ttsSpeed,
  volume: mateConfig.value.ttsVolume,
}))

const vrmModelPath = ref(resolveVrmModelPath(mateConfig.value.vrmModel))
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
    }>('mate_read_vrm_model', { relativePath: resolved })
    if (requestId === resolveModelRequestId) {
      vrmModelPath.value = `data:${data.contentType};base64,${data.base64Data}`
    }
  } catch (error) {
    console.warn('[DesktopMateOverlay] Failed to read custom VRM model:', error)
    if (requestId === resolveModelRequestId) {
      vrmModelPath.value = DEFAULT_VRM_MODEL_PATH
    }
  }
}

// ── 3D Gaussian Splatting: 背景场景 ────────────────────────────
// Desktop-mate window should always stay transparent. Embedded main-page view
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
const isSpeaking = ref(false)
let moodResetTimeoutId: ReturnType<typeof setTimeout> | null = null

// ── PTT (Push-To-Talk) ──────────────────────────────────────────

const voiceInput = useVoiceInput({
  config: computed(() => ({
    provider: mateConfig.value.asrProvider,
    language: mateConfig.value.asrLanguage,
    apiKey: mateConfig.value.asrApiKey,
    baseUrl: mateConfig.value.asrBaseUrl,
    model: mateConfig.value.asrModel,
  })),
  onRecognizedText: async (text: string) => {
    const trimmed = text.trim()
    if (!trimmed) return
    await emitTo('main', 'desktop-mate-voice-message', trimmed)
  },
})

const isPttDisabled = computed(() => !mateConfig.value.asrEnabled || !voiceInput.isSupported)

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
const desktopMateScale = ref(mateConfig.value.desktopMateScale ?? 1.0)

const scaleSliderPercent = computed(() => {
  const range = SCALE_MAX - SCALE_MIN
  if (range <= 0) return 0
  const percent = ((desktopMateScale.value - SCALE_MIN) / range) * 100
  return Math.max(0, Math.min(100, percent))
})

const scaleSliderStyle = computed(() => ({
  '--slider-progress': `${scaleSliderPercent.value}%`,
}))

function handleWheel(event: WheelEvent): void {
  // Block wheel zoom when click-through or drag mode is active
  if (isMousePassThrough.value || isDragMode.value) return

  event.preventDefault()

  const delta = event.deltaY > 0 ? -WHEEL_DELTA_STEP : WHEEL_DELTA_STEP
  const newScale = Math.max(SCALE_MIN, Math.min(SCALE_MAX, desktopMateScale.value + delta))

  desktopMateScale.value = newScale
  vrmAvatarRef.value?.setScale(newScale)
  mateConfig.value.desktopMateScale = newScale  // 持久化
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
    await invoke('set_desktop_mate_ignore_mouse', { ignore: isMousePassThrough.value })
  } catch (_) {
    isMousePassThrough.value = !isMousePassThrough.value
  }
  contextMenu.value = null
}

// ── Close ──────────────────────────────────────────────────────

async function closeMate() {
  try {
    await invoke('close_desktop_mate')
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
  desktopMateScale.value = value
  vrmAvatarRef.value?.setScale(value)
  mateConfig.value.desktopMateScale = value
  resetMenuHideTimer()
}

// ── Menu: 置顶切换 ──────────────────────────────────────────────

const isAlwaysOnTop = ref(mateConfig.value.desktopMateAlwaysOnTop ?? true)

async function toggleAlwaysOnTop() {
  const previous = isAlwaysOnTop.value
  isAlwaysOnTop.value = !isAlwaysOnTop.value
  try {
    await invoke('set_desktop_mate_always_on_top', { alwaysOnTop: isAlwaysOnTop.value })
    mateConfig.value.desktopMateAlwaysOnTop = isAlwaysOnTop.value
  } catch (_) {
    isAlwaysOnTop.value = previous
  }
  resetMenuHideTimer()
}

// ── Menu: 窗口操作 ──────────────────────────────────────────────

async function showMainWindow() {
  try {
    await invoke('open_desktop_mate')
  } catch (_) {}
  contextMenu.value = null
}

async function minimizeMate() {
  try {
    await invoke('minimize_desktop_mate')
  } catch (_) {
    // fallback: 使用 window API
    try {
      await getCurrentWindow().minimize()
    } catch (_) {}
  }
  contextMenu.value = null
}

// ── Menu: 外观/动画/语音 快速设置 ─────────────────────────────────

const vrmAppearances = computed(() => withDefaultAppearance(mateConfig.value.vrmAppearances ?? []))
const activeAppearanceId = computed(() =>
  resolveAppearance(mateConfig.value.vrmAppearances ?? [], mateConfig.value.activeAppearanceId).id,
)
const isVrmMotionEnabled = computed(() => mateConfig.value.vrmMotionEnabled)
const isVrmExpressionEnabled = computed(() => mateConfig.value.vrmExpressionEnabled)
const isTtsEnabled = computed(() => mateConfig.value.ttsEnabled)
const isAsrEnabled = computed(() => mateConfig.value.asrEnabled)
const isSubtitleEnabled = computed(() => mateConfig.value.subtitleEnabled)
const effectiveMood = computed<VrmMood>(() =>
  isVrmExpressionEnabled.value ? activeMood.value : 'neutral',
)
const activeAppearanceStartMotionId = computed(() =>
  resolveAppearance(mateConfig.value.vrmAppearances ?? [], mateConfig.value.activeAppearanceId).startMotionId || 'appearing',
)

function selectAppearance(id: string) {
  const appearance = resolveAppearance(mateConfig.value.vrmAppearances ?? [], id)
  mateConfig.value.activeAppearanceId = appearance.id
  mateConfig.value.vrmModel = appearance.modelId
  mateConfig.value.selectedMotionIds = [...appearance.motionIds]
  mateConfig.value.vrmMotionEnabled = appearance.motionEnabled
  mateConfig.value.vrmExpressionEnabled = appearance.expressionEnabled
  activeSubmenu.value = null
  contextMenu.value = null
}

function toggleVrmMotion() {
  mateConfig.value.vrmMotionEnabled = !mateConfig.value.vrmMotionEnabled
  resetMenuHideTimer()
}

function toggleVrmExpression() {
  mateConfig.value.vrmExpressionEnabled = !mateConfig.value.vrmExpressionEnabled
  resetMenuHideTimer()
}

function toggleTts() {
  mateConfig.value.ttsEnabled = !mateConfig.value.ttsEnabled
  resetMenuHideTimer()
}

function toggleAsr() {
  mateConfig.value.asrEnabled = !mateConfig.value.asrEnabled
  resetMenuHideTimer()
}

function toggleSubtitle() {
  mateConfig.value.subtitleEnabled = !mateConfig.value.subtitleEnabled
  resetMenuHideTimer()
}

/** Test TTS: read last subtitle text (if any) or use default test phrase. */
let testTtsCancelId = 0
async function testTts() {
  const rawText = subtitle.value.text?.trim()
  const displayText = rawText || '你好，我是 Diva，这是一条语音测试消息。'
  console.log('[DesktopMate] Test TTS triggered. Subtitle text:', JSON.stringify(subtitle.value.text))

  // Force-enable TTS for the test if currently disabled
  const savedEnabled = mateConfig.value.ttsEnabled
  if (!savedEnabled) {
    console.log('[DesktopMate] TTS was disabled, temporarily enabling for test.')
    mateConfig.value.ttsEnabled = true
  }

  const cancelId = ++testTtsCancelId
  const segments = splitIntoSentences(filterPunctuation(stripMarkdown(displayText)))

  console.log('[DesktopMate] Test TTS segments:', segments.length, segments.map(s => s.text.slice(0, 30)))

  for (const segment of segments) {
    if (cancelId !== testTtsCancelId) return
    try {
      await ttsService.speakText(segment.text, voiceConfig.value)
    } catch (err) {
      console.error('[DesktopMate] Test TTS segment failed:', err)
    }
  }

  // Restore original TTS enabled state
  if (!savedEnabled) {
    mateConfig.value.ttsEnabled = false
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
  () => mateConfig.value.vrmModel,
  (model) => {
    void refreshVrmModelPath(model)
  },
  { immediate: true },
)

watch(
  () => [mateConfig.value.activeAppearanceId, mateConfig.value.vrmAppearances] as const,
  () => {
    const appearance = resolveAppearance(mateConfig.value.vrmAppearances ?? [], mateConfig.value.activeAppearanceId)
    if (
      mateConfig.value.activeAppearanceId !== appearance.id ||
      mateConfig.value.vrmModel !== appearance.modelId
    ) {
      mateConfig.value.activeAppearanceId = appearance.id
      mateConfig.value.vrmModel = appearance.modelId
      mateConfig.value.selectedMotionIds = [...appearance.motionIds]
      mateConfig.value.vrmMotionEnabled = appearance.motionEnabled
      mateConfig.value.vrmExpressionEnabled = appearance.expressionEnabled
    }
  },
  { deep: true, immediate: true },
)

let subtitleTtsId = 0
function playLiveSubtitle(text: string): void {
  if (!text.trim() || !voiceConfig.value.enabled) return
  const cancelId = ++subtitleTtsId
  const segments = splitIntoSentences(filterPunctuation(stripMarkdown(text)))
  if (segments.length === 0) return
  isSpeaking.value = true
  void (async () => {
    try {
      for (const segment of segments) {
        if (cancelId !== subtitleTtsId) return
        try {
          await ttsService.speakText(segment.text, voiceConfig.value)
        } catch (err) {
          console.error('[DesktopMate] TTS segment failed:', err)
        }
      }
    } finally {
      if (cancelId === subtitleTtsId) isSpeaking.value = false
    }
  })()
}

// ── Lifecycle ──────────────────────────────────────────────────

const unlisteners: UnlistenFn[] = []

onMounted(async () => {
  isMousePassThrough.value = false

  // 应用持久化的缩放和置顶状态
  if (desktopMateScale.value !== 1.0) {
    vrmAvatarRef.value?.setScale(desktopMateScale.value)
  }
  isAlwaysOnTop.value = mateConfig.value.desktopMateAlwaysOnTop ?? true

  unlisteners.push(await listen<NeuroLinkPresentationPayload>('neuro-link-presentation', (event) => {
    const payload = event.payload
    if (!payload) return
    if (payload.event === 'subtitle.updated' && typeof payload.body?.text === 'string') {
      if (payload.source === 'live') playLiveSubtitle(payload.body.text)
    } else if (payload.event === 'subtitle.cleared') {
      subtitleTtsId += 1
      isSpeaking.value = false
      ttsService.stopPlayback()
    } else if (payload.event === 'persona.expression_hint') {
      const mood = isVrmExpressionEnabled.value ? normalizeMood(
        typeof payload.body?.expression === 'string' ? payload.body.expression : null,
      ) : 'neutral'
      activeMood.value = mood
      scheduleMoodReset(mood)
    }
  }))
  unlisteners.push(await listen('desktop-mate-close-request', closeMate))
  unlisteners.push(await listen('desktop-mate-render-pause', () => {
    isRenderActive.value = false
  }))
  unlisteners.push(await listen('desktop-mate-render-resume', () => {
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
    class="desktop-mate-overlay"
    :class="{ 'drag-mode': isDragMode }"
    @contextmenu.prevent="showContextMenu"
    @click="hideContextMenu"
    @pointerdown="onDragPointerDown"
  >
    <DivaVrmAvatar
      ref="vrmAvatarRef"
      :model-path="vrmModelPath"
      :mood="effectiveMood"
      :is-speaking="isSpeaking"
      :desktop-mate="true"
      :active="isRenderActive"
      :background-scene="backgroundSceneId"
      :background-scene-url="backgroundSceneUrl"
      :idle-motion-enabled="mateConfig.vrmMotionEnabled"
      :selected-motion-ids="mateConfig.selectedMotionIds"
      :start-motion-id="activeAppearanceStartMotionId"
      :start-motion-token="mateConfig.activeAppearanceId"
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
            <input
              type="range"
              class="menu-slider"
              :min="SCALE_MIN"
              :max="SCALE_MAX"
              :step="WHEEL_DELTA_STEP"
              :value="desktopMateScale"
              :style="scaleSliderStyle"
              aria-label="缩放"
              @input="handleScaleInput"
              @pointerdown.stop
            />
            <span class="menu-scale-value">{{ Math.round(desktopMateScale * 100) }}%</span>
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
        <div class="menu-item" @click="minimizeMate">
          <span class="menu-label">最小化</span>
        </div>
        <div class="menu-item menu-item-danger" @click="closeMate">
          <span class="menu-label">关闭</span>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.desktop-mate-overlay {
  width: 100%;
  height: 100%;
  position: relative;
  user-select: none;
  overflow: hidden;
  background: transparent;
}

.desktop-mate-overlay.drag-mode {
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
  border: 1px solid var(--mate-glass-border-subtle, rgba(255, 255, 255, 0.08));
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
  background: var(--mate-glass-bg, rgba(255, 255, 255, 0.1));
}

.menu-item + .menu-item {
  border-top: 1px solid var(--mate-glass-inset, rgba(255, 255, 255, 0.06));
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
  background: var(--mate-glass-border-subtle, rgba(255, 255, 255, 0.08));
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
  border: 1px solid var(--mate-glass-border-subtle, rgba(255, 255, 255, 0.08));
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
  background: var(--mate-glass-bg, rgba(255, 255, 255, 0.1));
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
  background: var(--mate-glass-border-subtle, rgba(255, 255, 255, 0.08));
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
  color: var(--mate-text-faint, rgba(255, 255, 255, 0.4));
  transition: color 0.15s ease;
}

.menu-toggle-state.on {
  color: #34d399;
}

/* ── Scale slider ────────────────────────────────────────────── */

.menu-item-slider {
  display: flex;
  flex-direction: column;
  gap: 8px;
  cursor: default;
  padding: 10px 16px;
}

.menu-slider-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.menu-slider {
  flex: 1;
  height: 6px;
  -webkit-appearance: none;
  appearance: none;
  background: linear-gradient(
    to right,
    #60a5fa 0%,
    #60a5fa var(--slider-progress, 35%),
    var(--mate-glass-bg-hover, rgba(255, 255, 255, 0.18)) var(--slider-progress, 35%),
    var(--mate-glass-bg-hover, rgba(255, 255, 255, 0.18)) 100%
  );
  border-radius: 999px;
  outline: none;
  cursor: pointer;
  transition: box-shadow 0.15s ease, filter 0.15s ease;
  box-shadow: inset 0 0 0 1px var(--mate-glass-inset, rgba(255, 255, 255, 0.06));
}

.menu-slider:hover {
  filter: brightness(1.05);
}

.menu-slider:focus-visible {
  box-shadow:
    0 0 0 3px rgba(96, 165, 250, 0.18),
    inset 0 0 0 1px rgba(255, 255, 255, 0.12);
}

.menu-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background:
    radial-gradient(circle at 30% 30%, rgba(255, 255, 255, 0.95), rgba(255, 255, 255, 0.4) 35%, rgba(96, 165, 250, 0.95) 36%, #60a5fa 100%);
  cursor: pointer;
  border: 2px solid rgba(255, 255, 255, 0.55);
  box-shadow: 0 3px 10px rgba(0, 0, 0, 0.32);
  transition: transform 0.12s ease, box-shadow 0.12s ease;
}

.menu-slider::-webkit-slider-thumb:hover {
  transform: scale(1.08);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.38);
}

.menu-slider::-webkit-slider-thumb:active {
  transform: scale(0.98);
}

.menu-slider::-moz-range-track {
  height: 6px;
  border-radius: 999px;
  background: var(--mate-glass-bg-hover, rgba(255, 255, 255, 0.18));
  box-shadow: inset 0 0 0 1px var(--mate-glass-inset, rgba(255, 255, 255, 0.06));
}

.menu-slider::-moz-range-progress {
  height: 6px;
  border-radius: 999px;
  background: #60a5fa;
}

.menu-slider::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background:
    radial-gradient(circle at 30% 30%, rgba(255, 255, 255, 0.95), rgba(255, 255, 255, 0.4) 35%, rgba(96, 165, 250, 0.95) 36%, #60a5fa 100%);
  cursor: pointer;
  border: 2px solid rgba(255, 255, 255, 0.55);
  box-shadow: 0 3px 10px rgba(0, 0, 0, 0.32);
  transition: transform 0.12s ease, box-shadow 0.12s ease;
}

.menu-slider::-moz-range-thumb:hover {
  transform: scale(1.08);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.38);
}

.menu-scale-value {
  font-size: 11px;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.88);
  min-width: 42px;
  text-align: right;
  font-variant-numeric: tabular-nums;
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--mate-glass-border-subtle, rgba(255, 255, 255, 0.08));
  border: 1px solid var(--mate-glass-border-subtle, rgba(255, 255, 255, 0.08));
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
  color: var(--danger, #ef4444);
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

<script setup lang="ts">
import { ref, watch, nextTick, computed, onUnmounted } from 'vue'
import { Send, Loader2, Settings, Monitor, Image, Menu, Plus, ChevronDown } from '@lucide/vue'
import MarkdownIt from 'markdown-it'
import { useI18n } from 'vue-i18n'
import EmbeddedPetFrame from './EmbeddedPetFrame.vue'
import DivaPetVoicePanel from '../voice/components/DivaPetVoicePanel.vue'
import DivaPetModelManager from './DivaPetModelManager.vue'
import { useVoicePlayer } from '../voice/composables/useVoicePlayer'
import { useVoiceInput } from '../voice/composables/useVoiceInput'
import { usePetConfig } from '../services/pet-config'
import { getTtsApiKey, type PetMessage, type VrmMood, type GaussSceneId } from '../types'
import { getDesktopPetEmotionSignal } from '../../../utils/desktop-pet-emotion'
import { resolveVrmModelPath } from '../utils/vrm-model'
import { resolveAppearance } from '../utils/default-appearance'
import { resolveGaussSceneUrl } from '../utils/gauss-scene'
import { ttsService, type TTSVoiceConfig } from '../voice/services/tts-service'
import { tauriVoiceFileReader } from '../voice/services/voice-api'

const { t } = useI18n()
const { config: petConfig, updateConfig } = usePetConfig()
ttsService.setVoiceFileReader(tauriVoiceFileReader)

const md = new MarkdownIt({ html: false, linkify: true, breaks: true })

const vrmModelPath = ref(resolveVrmModelPath(petConfig.value.vrmModel))
let resolveModelRequestId = 0
let invoke: ((cmd: string, args?: Record<string, unknown>) => Promise<unknown>) | null = null

async function getInvoke() {
  const isDesktop =
    typeof window !== 'undefined' && (window as any)._wails?.environment != null
  if (!invoke && isDesktop) {
    const mod = await import('../../../platform/desktop-host')
    invoke = mod.nativeCall
  }
  return invoke
}

async function refreshVrmModelPath(model: string) {
  const requestId = ++resolveModelRequestId
  const resolved = resolveVrmModelPath(model)
  if (!resolved.startsWith('vrm/models/custom/')) {
    vrmModelPath.value = resolved
    return
  }

  const inv = await getInvoke()
  if (!inv) {
    vrmModelPath.value = resolved
    return
  }
  try {
    const data = await inv('pet_read_vrm_model', { relativePath: resolved }) as {
      base64Data: string
      contentType: string
    }
    if (requestId === resolveModelRequestId) {
      vrmModelPath.value = `data:${data.contentType};base64,${data.base64Data}`
    }
  } catch (error) {
    console.warn('[DivaPetView] Failed to read custom VRM model:', error)
    if (requestId === resolveModelRequestId) {
      vrmModelPath.value = '/vrm/models/Alice.vrm'
    }
  }
}

interface SavedModel {
  id: string;
  provider: string;
  model: string;
  apiBase: string;
  apiKey: string;
  displayName: string;
}

interface Props {
  messages?: PetMessage[]
  isTyping?: boolean
  currentEmotion?: string
  desktopPetActive?: boolean
  savedModels?: SavedModel[]
  currentModel?: string
  currentProvider?: string
  connectionStatus?: 'connected' | 'error' | 'connecting'
}
const props = withDefaults(defineProps<Props>(), {
  messages: () => [],
  isTyping: false,
  currentEmotion: 'normal',
  desktopPetActive: false,
  savedModels: () => [],
  currentModel: '',
  currentProvider: '',
  connectionStatus: 'connecting',
})

const emit = defineEmits<{
  (e: 'send', content: string): void
  (e: 'toggle-sidebar'): void
  (e: 'new-topic', greeting: string): void
  (e: 'select-model', model: SavedModel): void
}>()

const NEW_TOPIC_GREETING = '让我们换个话题聊聊吧'

const currentMood = ref<VrmMood>('neutral')
const lastMoodSignature = ref<string | null>(null)
let moodResetTimer: ReturnType<typeof setTimeout> | null = null

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

const { isSpeaking, speakText, stopSpeaking } = useVoicePlayer({
  messages: computed(() => props.messages),
  isTyping: computed(() => props.isTyping),
  ttsConfig: voiceConfig,
})

const voiceInput = useVoiceInput({
  isSuspended: computed(() => isSpeaking.value),
  config: computed(() => ({
    provider: petConfig.value.asrProvider,
    language: petConfig.value.asrLanguage,
    apiKey: petConfig.value.asrApiKey,
    baseUrl: petConfig.value.asrBaseUrl,
    model: petConfig.value.asrModel,
  })),
  onRecognizedText: async (text: string) => {
    if (props.isTyping) return
    emit('send', text)
  },
})

watch(
  () => petConfig.value.asrEnabled,
  async (enabled) => {
    if (enabled === voiceInput.isEnabled.value) return
    const applied = await voiceInput.setEnabled(enabled)
    if (enabled && !applied) updateConfig({ asrEnabled: false })
  },
  { immediate: true },
)

function clearMoodResetTimer() {
  if (moodResetTimer !== null) {
    window.clearTimeout(moodResetTimer)
    moodResetTimer = null
  }
}

function scheduleMoodReset(mood: VrmMood) {
  clearMoodResetTimer()
  if (mood === 'neutral') {
    currentMood.value = 'neutral'
    return
  }
  moodResetTimer = window.setTimeout(() => {
    moodResetTimer = null
    currentMood.value = 'neutral'
  }, 4000)
}

watch(
  () => getDesktopPetEmotionSignal(props.messages),
  (signal) => {
    if (!signal) return
    if (signal.signature === lastMoodSignature.value) return

    lastMoodSignature.value = signal.signature
    currentMood.value = signal.mood as VrmMood
    scheduleMoodReset(currentMood.value)
  },
)

onUnmounted(() => {
  clearMoodResetTimer()
})

function onTtsToggle(val: boolean) {
  updateConfig({ ttsEnabled: val })
}

async function onVoiceToggle() {
  const nextEnabled = !voiceInput.isEnabled.value
  const applied = await voiceInput.setEnabled(nextEnabled)
  updateConfig({ asrEnabled: nextEnabled && applied })
}

const isHeaderPttDisabled = computed(() => props.isTyping || !voiceInput.isSupported)

async function startHeaderVoiceInput(event: PointerEvent) {
  event.preventDefault()
  if (isHeaderPttDisabled.value || voiceInput.isEnabled.value) return
  await voiceInput.setEnabled(true)
}

function stopHeaderVoiceInput() {
  if (!voiceInput.isEnabled.value) return
  void voiceInput.setEnabled(false)
}

function onTestSpeak() {
  speakText('Diva pet mode preview.')
}

function onNewTopic() {
  if (props.isTyping) return
  emit('new-topic', NEW_TOPIC_GREETING)
}

const showModelManager = ref(false)
const showModelMenu = ref(false)
const previewMotionId = ref<string | null>(null)
const stopPreviewToken = ref(0)

const emotionConfig = computed(() => ({
  happy: { emoji: '😊', label: t('emotion.happy') },
  sad: { emoji: '😢', label: t('emotion.sad') },
  angry: { emoji: '😠', label: t('emotion.angry') },
  surprised: { emoji: '😲', label: t('emotion.surprised') },
  normal: { emoji: '😐', label: t('emotion.normal') },
  excited: { emoji: '🤩', label: t('emotion.excited') },
  confused: { emoji: '😕', label: t('emotion.confused') },
  worried: { emoji: '😟', label: t('emotion.worried') },
  love: { emoji: '😍', label: t('emotion.love') },
  sleepy: { emoji: '😴', label: t('emotion.sleepy') },
}))

function selectModel(model: SavedModel) {
  showModelMenu.value = false
  emit('select-model', model)
}

function onModelChanged(modelId: string) {
  updateConfig({ vrmModel: modelId })
}

function onPreviewMotion(id: string) {
  previewMotionId.value = id
}

function onStopPreview() {
  previewMotionId.value = null
  stopPreviewToken.value += 1
}

const showScenePicker = ref(false)
const isWhisperNearby = ref(false)
const isTransparentScene = computed(() => petConfig.value.selectedGaussSceneId === 'transparent')
const runtimeBackgroundScene = computed(() => (
  isTransparentScene.value ? undefined : petConfig.value.selectedGaussSceneId
))
const activeAppearanceStartMotionId = computed(() =>
  resolveAppearance(petConfig.value.vrmAppearances, petConfig.value.activeAppearanceId).startMotionId || 'appearing',
)

const SCENE_ICONS: Record<string, string> = {
  transparent: 'T', home: 'H', sea: 'S', space: '*',
}
function getSceneIcon(id: string) { return SCENE_ICONS[id] ?? '-' }
function selectScene(id: string) {
  updateConfig({ selectedGaussSceneId: id as GaussSceneId })
  showScenePicker.value = false
}

const backgroundSceneUrl = computed(() => {
  if (isTransparentScene.value) return undefined
  const scene = petConfig.value.gaussSceneList?.find(
    (s) => s.id === petConfig.value.selectedGaussSceneId,
  )
  return resolveGaussSceneUrl(scene?.path)
})

function onClickOutside() { showScenePicker.value = false }
function onWhisperZoneEnter() { isWhisperNearby.value = true }
function onWhisperZoneLeave() { isWhisperNearby.value = false }

const inputText = ref('')
const messagesContainer = ref<HTMLElement | null>(null)

function handleSend() {
  const trimmed = inputText.value.trim()
  if (!trimmed || props.isTyping) return
  emit('send', trimmed)
  inputText.value = ''
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    handleSend()
  }
}

function formatTime(ts?: number): string {
  if (!ts) return ''
  const d = new Date(ts)
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function scrollToBottom() {
  nextTick(() => {
    if (messagesContainer.value) {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
    }
  })
}

watch(() => props.messages.length, scrollToBottom)
watch(() => props.messages, scrollToBottom, { deep: true })
watch(
  () => petConfig.value.vrmModel,
  (model) => {
    void refreshVrmModelPath(model)
  },
  { immediate: true },
)

</script>

<template>
  <div data-ui-surface="overlay" class="diva-pet-view h-full relative overflow-hidden">
    <div class="diva-pet-backdrop absolute inset-0 pointer-events-none" />

    <div class="avatar-section relative h-full min-h-0">
      <button
        class="ui-button ui-button--ghost ui-button--compact pet-edge-button absolute top-4 left-4 z-20"
        :title="t('nav.openSidebar')"
        @click="emit('toggle-sidebar')"
      >
        <Menu :size="16" />
      </button>

      <EmbeddedPetFrame
        v-show="!desktopPetActive"
        :model-path="vrmModelPath"
        :mood="currentMood"
        :is-speaking="isSpeaking"
        :active="!desktopPetActive"
        :lip-sync-enabled="petConfig.vrmExpressionEnabled"
        :idle-motion-enabled="petConfig.vrmMotionEnabled"
        :selected-motion-ids="petConfig.selectedMotionIds"
        :start-motion-id="activeAppearanceStartMotionId"
        :start-motion-token="petConfig.activeAppearanceId"
        :preview-motion-id="previewMotionId"
        :stop-preview-token="stopPreviewToken"
        :background-scene="runtimeBackgroundScene"
        :background-scene-url="backgroundSceneUrl"
        :transparent-background="isTransparentScene"
      />

      <div
        v-if="desktopPetActive"
        class="w-full h-full flex flex-col items-center justify-center text-foreground gap-3"
      >
        <Monitor :size="36" class="text-primary animate-pulse" />
        <p class="text-sm">{{ t('pet.desktopPetActiveHint') }}</p>
      </div>

      <!-- 迷你状态栏 (Mini Status Bar) -->
      <div
        class="pet-glass absolute top-4 left-16 z-20 flex items-center gap-2 px-3 py-1.5 rounded-full text-foreground border-border"
      >
        <!-- 情绪 -->
        <span class="text-sm">{{ emotionConfig[(props.currentEmotion || 'happy') as keyof typeof emotionConfig]?.emoji || '😊' }}</span>
        <span class="text-[11px]">{{ emotionConfig[(props.currentEmotion || 'happy') as keyof typeof emotionConfig]?.label || t('emotion.happy') }}</span>
        <!-- 分隔线 -->
        <span class="w-px h-3 bg-card" />
        <!-- 连接状态 -->
        <div class="flex items-center gap-1">
          <div
            class="w-1.5 h-1.5 rounded-full"
            :class="{
              'bg-success': props.connectionStatus === 'connected',
              'bg-destructive': props.connectionStatus === 'error',
              'bg-warning animate-pulse': props.connectionStatus === 'connecting',
            }"
          />
          <span class="text-[10px]">
            {{ props.connectionStatus === 'connected' ? t('app.online') : props.connectionStatus === 'error' ? t('app.offline') : t('app.connecting') }}
          </span>
        </div>
        <!-- 分隔线 -->
        <span class="w-px h-3 bg-card" />
        <!-- 模型选择 -->
        <div class="relative">
          <button
            class="text-[10px] hover:text-primary transition-colors flex items-center gap-1"
            @click="showModelMenu = !showModelMenu"
          >
            <span class="max-w-[80px] truncate">{{ props.currentModel || t('app.switchModel') }}</span>
            <ChevronDown :size="10" />
          </button>
          <!-- 模型下拉菜单 -->
          <div
            v-if="showModelMenu"
            class="absolute top-full left-0 mt-1 w-48 bg-card   rounded-lg shadow-xl border border-border overflow-hidden z-30"
          >
            <div class="py-1 max-h-60 overflow-y-auto">
              <div
                v-for="model in props.savedModels"
                :key="model.id"
                class="w-full cursor-pointer px-3 py-2 text-left text-xs hover:bg-accent  flex items-center justify-between"
                @click="selectModel(model)"
              >
                <span class="truncate">{{ model.displayName }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <button
        class="ui-button ui-button--ghost ui-button--compact pet-edge-button absolute top-4 right-16 z-20"
        title="外观设置"
        @click="showModelManager = !showModelManager"
      >
        <Settings :size="14" />
      </button>

      <button
        class="ui-button ui-button--ghost ui-button--compact pet-edge-button absolute top-4 right-4 z-20"
        title="Switch Scene"
        @click.stop="showScenePicker = !showScenePicker"
      >
        <Image :size="14" />
      </button>

      <Transition name="menu-fade">
        <div
          v-if="showScenePicker"
          class="pet-glass absolute top-16 right-4 z-20 min-w-[160px] py-1 rounded-2xl border-border text-foreground shadow-2xl"
          @click.stop
        >
          <div
            v-for="s in petConfig.gaussSceneList"
            :key="s.id"
            class="flex items-center gap-2 px-3 py-2 text-xs cursor-pointer transition-colors rounded-xl mx-1 my-0.5 hover:bg-card"
            :class="s.id === petConfig.selectedGaussSceneId ? 'text-primary bg-card' : 'text-foreground'"
            @click="selectScene(s.id)"
          >
            <span>{{ getSceneIcon(s.id) }}</span>
            <span>{{ s.name }}</span>
          </div>
        </div>
      </Transition>

      <div v-if="showScenePicker" class="fixed inset-0 z-10" @click="onClickOutside" />

      <div
        class="pet-whisper-zone absolute left-4 top-1/2 z-20 w-[min(316px,calc(100%-1.5rem))] -translate-y-1/2"
        @pointerenter="onWhisperZoneEnter"
        @pointerleave="onWhisperZoneLeave"
      >
        <div
          class="pet-chat-panel transition-all duration-200"
          :class="isWhisperNearby ? 'pet-glass pet-chat-panel--active' : 'pet-chat-panel--idle'"
        >
          <div class="pet-chat-header">
            <span class="text-[11px] tracking-[0.24em] uppercase text-foreground">Whispers</span>
            <button
              class="ui-button ui-button--ghost pet-chat-new-topic-button ui-button--compact ui-button--icon"
              :disabled="isTyping"
              title="新建聊天"
              @click.stop="onNewTopic"
            >
              <Plus :size="12" />
            </button>
          </div>

          <div ref="messagesContainer" class="pet-chat-scroll space-y-2">
            <div
              v-for="(msg, idx) in messages"
              :key="idx"
              class="flex"
              :class="msg.role === 'user' ? 'justify-end' : 'justify-start'"
            >
              <div
                v-if="msg.role === 'user' || msg.role === 'agent'"
                class="chat-bubble max-w-[92%] px-3 py-2 rounded-[18px] text-xs leading-relaxed break-words"
                :class="{
                  'pet-user-bubble rounded-br-md': msg.role === 'user',
                  'pet-agent-bubble rounded-bl-md': msg.role === 'agent',
                }"
              >
                <div v-if="msg.content" class="whitespace-pre-wrap markdown-body pet-markdown" v-html="md.render(msg.content)"></div>
                <div v-else-if="msg.role === 'agent' && msg.isStreaming" class="flex items-center gap-1.5 text-foreground">
                  <Loader2 :size="12" class="animate-spin" />
                  <span class="text-[10px]">{{ t('chat.thinking') }}</span>
                </div>
                <div
                  v-if="msg.timestamp"
                  class="text-[9px] mt-0.5 opacity-50"
                  :class="msg.role === 'user' ? 'text-right' : 'text-left'"
                >
                  {{ formatTime(msg.timestamp) }}
                </div>
              </div>
            </div>

          </div>
        </div>
      </div>

      <div class="absolute left-4 right-4 bottom-4 z-20 flex flex-col items-center gap-3">
        <DivaPetVoicePanel
          :is-speaking="isSpeaking"
          :is-voice-supported="voiceInput.isSupported"
          :is-voice-enabled="voiceInput.isEnabled.value"
          :is-listening="voiceInput.isListening.value"
          :is-processing="voiceInput.isProcessing.value"
          :voice-error="voiceInput.error.value"
          :tts-enabled="petConfig.ttsEnabled"
          :is-push-to-talk-disabled="isHeaderPttDisabled"
          @toggle-voice="onVoiceToggle"
          @update:tts-enabled="onTtsToggle"
          @test-speak="onTestSpeak"
          @start-voice-hold="startHeaderVoiceInput"
          @stop-voice-hold="stopHeaderVoiceInput"
          @stop-speaking="stopSpeaking"
        />

        <div class="pet-input-dock pet-glass">
          <div class="flex w-full items-center gap-2">
            <input
              v-model="inputText"
              type="text"
              :placeholder="t('chat.placeholder')"
              :disabled="isTyping"
              class="ui-input ui-input--embedded pet-input flex-1"
              @keydown="handleKeydown"
            />
            <button
              :disabled="!inputText.trim() || isTyping"
              class="ui-button ui-button--primary pet-send-button ui-button--compact ui-button--icon"
              @click="handleSend"
            >
              <Send :size="14" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <DivaPetModelManager
      :visible="showModelManager"
      @close="showModelManager = false"
      @model-changed="onModelChanged"
      @preview-motion="onPreviewMotion"
      @stop-preview="onStopPreview"
    />
  </div>
</template>

<style scoped>
.diva-pet-view {
  font-family: "Segoe UI", "Microsoft YaHei", "PingFang SC", sans-serif;
  --pet-bg-top: rgba(17, 24, 39, 0.18);
  --pet-bg-mid: rgba(12, 74, 110, 0.12);
  --pet-bg-bottom: rgba(15, 23, 42, 0.42);
}

.avatar-section {
  min-height: 0;
}

.diva-pet-backdrop {
  background:
    var(--card);
}

.pet-glass {
  background: var(--card);
  border: 1px solid var(--border);
  box-shadow: var(--shadow-sm);
}

.pet-edge-button {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.18s ease;
}

.pet-chat-panel {
  border-radius: 26px;
  padding: 12px;
}

.pet-chat-panel--idle {
  background: transparent;
  border: 1px solid transparent;
  box-shadow: var(--shadow-sm);
}

.pet-chat-panel--active {
  background: var(--card);
  border-color: var(--border);
  box-shadow: var(--shadow-sm);
}

.pet-whisper-zone {
  padding: 10px;
  margin: -10px;
}

.pet-chat-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 2px 10px;
}

.pet-chat-new-topic-button {
  width: 22px;
  height: 22px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: color 0.14s ease, background 0.14s ease, border-color 0.14s ease;
}

.pet-chat-scroll {
  max-height: min(42vh, 360px);
  overflow-y: auto;
  padding-right: 6px;
  scrollbar-gutter: stable;
  scrollbar-width: thin;
  scrollbar-color: var(--muted-foreground) transparent;
}

.pet-chat-scroll::-webkit-scrollbar {
  width: 6px;
}

.pet-chat-scroll::-webkit-scrollbar-track {
  background: transparent;
}

.pet-chat-scroll::-webkit-scrollbar-thumb {
  min-height: 32px;
  border-radius: 9999px;
  background: var(--card);
  border: 2px solid transparent;
  background-clip: content-box;
}

.pet-chat-scroll::-webkit-scrollbar-thumb:hover {
  background: var(--card);
  background-clip: content-box;
}

.pet-chat-scroll::-webkit-scrollbar-corner {
  background: transparent;
}

.pet-agent-bubble {
  color: var(--foreground);
  background: var(--card);
  border: 1px solid var(--border);
  box-shadow: var(--shadow-sm);
}

.pet-user-bubble {
  color: var(--message-user-foreground);
  background: var(--message-user);
  border: 1px solid var(--border-strong);
  box-shadow: var(--shadow-sm);
}

.pet-input-dock {
  width: min(760px, 100%);
  max-width: 100%;
  align-self: center;
  border-radius: 9999px;
  padding: 8px 8px 8px 16px;
}

.pet-input-dock:focus-within {
  border-color: var(--ring);
  box-shadow: 0 0 0 2px var(--ring-soft), var(--shadow-sm);
}

.pet-input {
  min-width: 0;
}

.pet-input:disabled {
  opacity: 0.55;
}

.pet-send-button {
  width: 38px;
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.18s ease;
}

.chat-bubble {
  animation: fade-in 0.2s ease-out;
}

@keyframes fade-in {
  from {
    opacity: 0;
    transform: translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.menu-fade-enter-active,
.menu-fade-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.menu-fade-enter-from,
.menu-fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

@media (max-width: 768px) {
  .pet-chat-panel {
    border-radius: 22px;
  }

  .pet-chat-scroll {
    max-height: min(34vh, 260px);
  }

  .pet-input-dock {
    width: 100%;
  }
}

.pet-markdown {
  font-size: 0.75rem;
  line-height: 1.5;
}

.pet-markdown :deep(p) {
  margin-bottom: 0.3em;
}

.pet-markdown :deep(p:last-child) {
  margin-bottom: 0;
}

.pet-markdown :deep(strong) {
  font-weight: 600;
  color: inherit;
}

.pet-markdown :deep(em) {
  font-style: italic;
  color: inherit;
}

.pet-markdown :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 0.7rem;
  background-color: var(--muted);
  padding: 0.15em 0.35em;
  border-radius: 0.25rem;
  color: var(--primary);
}

.pet-markdown :deep(pre) {
  background-color: var(--code-background);
  border-radius: 0.375rem;
  padding: 0.5rem 0.65rem;
  margin: 0.4rem 0;
  overflow-x: auto;
}

.pet-markdown :deep(pre code) {
  background-color: transparent;
  padding: 0;
  color: var(--code-foreground);
}

.pet-markdown :deep(ul), .pet-markdown :deep(ol) {
  padding-left: 1.25em;
  margin-bottom: 0.3em;
}

.pet-markdown :deep(ul) {
  list-style-type: disc;
}

.pet-markdown :deep(ol) {
  list-style-type: decimal;
}

.pet-markdown :deep(li) {
  margin-bottom: 0.15em;
}

.pet-markdown :deep(blockquote) {
  border-left: 2px solid var(--border-strong);
  padding-left: 0.6rem;
  color: inherit;
  margin: 0.3rem 0;
}

.pet-markdown :deep(a) {
  color: var(--primary);
  text-decoration: underline;
}

.pet-markdown :deep(a:hover) {
  color: var(--info);
}

.pet-markdown :deep(h1), .pet-markdown :deep(h2), .pet-markdown :deep(h3),
.pet-markdown :deep(h4), .pet-markdown :deep(h5), .pet-markdown :deep(h6) {
  font-weight: 600;
  margin-bottom: 0.25em;
  color: inherit;
}

.pet-user-bubble .pet-markdown :deep(a) {
  color: inherit;
}

.pet-markdown :deep(h1) { font-size: 0.85rem; }
.pet-markdown :deep(h2) { font-size: 0.8rem; }
.pet-markdown :deep(h3) { font-size: 0.75rem; }
.pet-markdown :deep(h4), .pet-markdown :deep(h5), .pet-markdown :deep(h6) { font-size: 0.72rem; }

.pet-markdown :deep(hr) {
  border: none;
  border-top: 1px solid var(--border);
  margin: 0.5rem 0;
}

.pet-markdown :deep(table) {
  border-collapse: collapse;
  font-size: 0.7rem;
  margin: 0.3rem 0;
}

.pet-markdown :deep(table td), .pet-markdown :deep(table th) {
  border: 1px solid var(--border);
  padding: 0.2em 0.5em;
}
</style>

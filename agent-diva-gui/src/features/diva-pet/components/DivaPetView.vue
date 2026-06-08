<script setup lang="ts">
import { ref, watch, nextTick, computed, onUnmounted } from 'vue'
import { Send, Loader2, Settings, Monitor, Image, Menu, Plus } from 'lucide-vue-next'
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
  const isTauri =
    typeof window !== 'undefined' &&
    ('__TAURI_INTERNALS__' in window || '__TAURI__' in window)
  if (!invoke && isTauri) {
    const mod = await import('@tauri-apps/api/core')
    invoke = mod.invoke
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

interface Props {
  messages?: PetMessage[]
  isTyping?: boolean
  currentEmotion?: string
  desktopPetActive?: boolean
}
const props = withDefaults(defineProps<Props>(), {
  messages: () => [],
  isTyping: false,
  currentEmotion: 'normal',
  desktopPetActive: false,
})

const emit = defineEmits<{
  (e: 'send', content: string): void
  (e: 'toggle-sidebar'): void
  (e: 'new-topic', greeting: string): void
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
const previewMotionId = ref<string | null>(null)
const stopPreviewToken = ref(0)

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

const moodLabels: Record<VrmMood, string> = {
  neutral: '',
  happy: ':)',
  sad: ':(',
  angry: '!!',
  surprised: '?!',
}
</script>

<template>
  <div class="diva-pet-view h-full relative overflow-hidden">
    <div class="diva-pet-backdrop absolute inset-0 pointer-events-none" />

    <div class="avatar-section relative h-full min-h-0">
      <button
        class="pet-edge-button absolute top-4 left-4 z-20"
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
        class="w-full h-full flex flex-col items-center justify-center text-white/70 gap-3"
      >
        <Monitor :size="36" class="text-cyan-100/80 animate-pulse" />
        <p class="text-sm">{{ t('pet.desktopPetActiveHint') }}</p>
      </div>

      <div
        v-if="currentMood !== 'neutral'"
        data-testid="pet-mood-badge"
        class="pet-glass absolute top-4 left-16 px-3 py-1.5 text-[11px] rounded-full text-white/90 border-white/15 z-20"
      >
        {{ moodLabels[currentMood] }} {{ currentMood }}
      </div>

      <button
        class="pet-edge-button absolute top-4 right-16 z-20"
        title="外观设置"
        @click="showModelManager = !showModelManager"
      >
        <Settings :size="14" />
      </button>

      <button
        class="pet-edge-button absolute top-4 right-4 z-20"
        title="Switch Scene"
        @click.stop="showScenePicker = !showScenePicker"
      >
        <Image :size="14" />
      </button>

      <Transition name="menu-fade">
        <div
          v-if="showScenePicker"
          class="pet-glass absolute top-16 right-4 z-20 min-w-[160px] py-1 rounded-2xl border-white/15 text-white/90 shadow-2xl"
          @click.stop
        >
          <div
            v-for="s in petConfig.gaussSceneList"
            :key="s.id"
            class="flex items-center gap-2 px-3 py-2 text-xs cursor-pointer transition-colors rounded-xl mx-1 my-0.5 hover:bg-white/10"
            :class="s.id === petConfig.selectedGaussSceneId ? 'text-cyan-100 bg-white/14' : 'text-white/75'"
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
            <span class="text-[11px] tracking-[0.24em] uppercase text-white/55">Whispers</span>
            <button
              class="pet-chat-new-topic-button"
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
                <div v-else-if="msg.role === 'agent' && msg.isStreaming" class="flex items-center gap-1.5 text-white/60">
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
              class="pet-input flex-1"
              @keydown="handleKeydown"
            />
            <button
              :disabled="!inputText.trim() || isTyping"
              class="pet-send-button"
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
  --pet-panel-bg: linear-gradient(145deg, rgba(19, 28, 45, 0.30), rgba(255, 255, 255, 0.10));
  --pet-panel-border: rgba(255, 255, 255, 0.16);
  --pet-panel-shadow: 0 24px 80px rgba(15, 23, 42, 0.28);
}

.avatar-section {
  min-height: 0;
}

.diva-pet-backdrop {
  background:
    radial-gradient(circle at 22% 24%, rgba(125, 211, 252, 0.20), transparent 24%),
    radial-gradient(circle at 78% 18%, rgba(244, 114, 182, 0.14), transparent 18%),
    linear-gradient(180deg, var(--pet-bg-top), var(--pet-bg-mid) 35%, var(--pet-bg-bottom));
}

.pet-glass {
  background: var(--pet-panel-bg);
  backdrop-filter: blur(22px);
  border: 1px solid var(--pet-panel-border);
  box-shadow: var(--pet-panel-shadow);
}

.pet-edge-button {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 9999px;
  color: rgba(255, 255, 255, 0.82);
  background: linear-gradient(145deg, rgba(15, 23, 42, 0.38), rgba(255, 255, 255, 0.10));
  border: 1px solid rgba(255, 255, 255, 0.14);
  box-shadow: 0 16px 40px rgba(15, 23, 42, 0.24);
  backdrop-filter: blur(18px);
  transition: all 0.18s ease;
}

.pet-edge-button:hover {
  transform: translateY(-1px);
  color: rgba(255, 255, 255, 1);
  background: linear-gradient(145deg, rgba(15, 23, 42, 0.48), rgba(255, 255, 255, 0.13));
}

.pet-chat-panel {
  border-radius: 26px;
  padding: 12px;
}

.pet-chat-panel--idle {
  background: transparent;
  border: 1px solid transparent;
  box-shadow: none;
  backdrop-filter: blur(0px);
}

.pet-chat-panel--active {
  background: var(--pet-panel-bg);
  border-color: var(--pet-panel-border);
  box-shadow: var(--pet-panel-shadow);
  backdrop-filter: blur(22px);
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
  border-radius: 9999px;
  color: rgba(255, 255, 255, 0.62);
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.12);
  transition: color 0.14s ease, background 0.14s ease, border-color 0.14s ease;
}

.pet-chat-new-topic-button:hover:not(:disabled) {
  color: rgba(255, 255, 255, 0.94);
  background: rgba(255, 255, 255, 0.16);
  border-color: rgba(255, 255, 255, 0.22);
}

.pet-chat-new-topic-button:disabled {
  opacity: 0.38;
  cursor: not-allowed;
}

.pet-chat-scroll {
  max-height: min(42vh, 360px);
  overflow-y: auto;
  padding-right: 6px;
  scrollbar-gutter: stable;
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.24) transparent;
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
  background: rgba(255, 255, 255, 0.22);
  border: 2px solid transparent;
  background-clip: content-box;
}

.pet-chat-scroll::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.34);
  background-clip: content-box;
}

.pet-chat-scroll::-webkit-scrollbar-corner {
  background: transparent;
}

.pet-agent-bubble {
  color: rgba(255, 255, 255, 0.92);
  background: linear-gradient(145deg, rgba(255, 255, 255, 0.14), rgba(255, 255, 255, 0.07));
  border: 1px solid rgba(255, 255, 255, 0.12);
  box-shadow: 0 12px 30px rgba(15, 23, 42, 0.18);
  backdrop-filter: blur(18px);
}

.pet-user-bubble {
  color: rgba(255, 255, 255, 0.96);
  background: linear-gradient(145deg, rgba(34, 211, 238, 0.42), rgba(59, 130, 246, 0.22));
  border: 1px solid rgba(165, 243, 252, 0.28);
  box-shadow: 0 12px 30px rgba(8, 47, 73, 0.22);
  backdrop-filter: blur(18px);
}

.pet-input-dock {
  width: min(760px, 100%);
  max-width: 100%;
  align-self: center;
  border-radius: 9999px;
  padding: 8px 8px 8px 16px;
}

.pet-input {
  min-width: 0;
  background: transparent;
  border: 0;
  color: rgba(255, 255, 255, 0.92);
  font-size: 12px;
  line-height: 1.4;
  outline: none;
}

.pet-input::placeholder {
  color: rgba(255, 255, 255, 0.48);
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
  border-radius: 9999px;
  color: rgba(255, 255, 255, 0.95);
  background: linear-gradient(145deg, rgba(56, 189, 248, 0.72), rgba(244, 114, 182, 0.42));
  box-shadow: 0 12px 28px rgba(14, 116, 144, 0.24);
  transition: all 0.18s ease;
}

.pet-send-button:hover:not(:disabled) {
  transform: translateY(-1px);
  filter: brightness(1.06);
}

.pet-send-button:disabled {
  opacity: 0.4;
  cursor: not-allowed;
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
  color: rgba(255, 255, 255, 0.95);
}

.pet-markdown :deep(em) {
  font-style: italic;
  color: rgba(255, 255, 255, 0.85);
}

.pet-markdown :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 0.7rem;
  background-color: rgba(0, 0, 0, 0.25);
  padding: 0.15em 0.35em;
  border-radius: 0.25rem;
  color: rgba(165, 243, 252, 0.9);
}

.pet-markdown :deep(pre) {
  background-color: rgba(0, 0, 0, 0.35);
  border-radius: 0.375rem;
  padding: 0.5rem 0.65rem;
  margin: 0.4rem 0;
  overflow-x: auto;
}

.pet-markdown :deep(pre code) {
  background-color: transparent;
  padding: 0;
  color: rgba(255, 255, 255, 0.85);
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
  border-left: 2px solid rgba(165, 243, 252, 0.4);
  padding-left: 0.6rem;
  color: rgba(255, 255, 255, 0.65);
  margin: 0.3rem 0;
}

.pet-markdown :deep(a) {
  color: rgba(165, 243, 252, 0.9);
  text-decoration: underline;
}

.pet-markdown :deep(a:hover) {
  color: rgba(103, 232, 249, 1);
}

.pet-markdown :deep(h1), .pet-markdown :deep(h2), .pet-markdown :deep(h3),
.pet-markdown :deep(h4), .pet-markdown :deep(h5), .pet-markdown :deep(h6) {
  font-weight: 600;
  margin-bottom: 0.25em;
  color: rgba(255, 255, 255, 0.92);
}

.pet-markdown :deep(h1) { font-size: 0.85rem; }
.pet-markdown :deep(h2) { font-size: 0.8rem; }
.pet-markdown :deep(h3) { font-size: 0.75rem; }
.pet-markdown :deep(h4), .pet-markdown :deep(h5), .pet-markdown :deep(h6) { font-size: 0.72rem; }

.pet-markdown :deep(hr) {
  border: none;
  border-top: 1px solid rgba(255, 255, 255, 0.15);
  margin: 0.5rem 0;
}

.pet-markdown :deep(table) {
  border-collapse: collapse;
  font-size: 0.7rem;
  margin: 0.3rem 0;
}

.pet-markdown :deep(table td), .pet-markdown :deep(table th) {
  border: 1px solid rgba(255, 255, 255, 0.15);
  padding: 0.2em 0.5em;
}
</style>

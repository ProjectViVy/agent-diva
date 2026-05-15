<script setup lang="ts">
import { ref, watch, nextTick, computed } from 'vue'
import { Send, Loader2, Settings, Monitor, Image, Menu } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import EmbeddedPetFrame from './EmbeddedPetFrame.vue'
import DivaPetVoicePanel from '../voice/components/DivaPetVoicePanel.vue'
import DivaPetModelManager from './DivaPetModelManager.vue'
import { useVoicePlayer } from '../voice/composables/useVoicePlayer'
import { useVoiceInput } from '../voice/composables/useVoiceInput'
import { usePetConfig } from '../services/pet-config'
import type { PetMessage, VrmMood, GaussSceneId } from '../types'
import { deriveMoodFromMessages } from '../utils/mood'
import { resolveVrmModelPath } from '../utils/vrm-model'
import { resolveGaussSceneUrl } from '../utils/gauss-scene'
import { ttsService, type TTSVoiceConfig } from '../voice/services/tts-service'
import { tauriVoiceFileReader } from '../voice/services/voice-api'

const { t } = useI18n()
const { config: petConfig, updateConfig } = usePetConfig()
ttsService.setVoiceFileReader(tauriVoiceFileReader)

const vrmModelPath = computed(() => resolveVrmModelPath(petConfig.value.vrmModel))

interface Props {
  messages: PetMessage[]
  isTyping: boolean
  currentEmotion?: string
  desktopPetActive?: boolean
}
const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'send', content: string): void
  (e: 'toggle-sidebar'): void
}>()

const currentMood = computed<VrmMood>(() =>
  deriveMoodFromMessages(props.messages, props.currentEmotion),
)

const voiceConfig = computed<TTSVoiceConfig>(() => ({
  enabled: petConfig.value.ttsEnabled,
  provider: petConfig.value.ttsProvider,
  apiKey: petConfig.value.ttsApiKey,
  baseUrl: petConfig.value.ttsBaseUrl,
  model: petConfig.value.ttsModel,
  referenceVoice: petConfig.value.ttsReferenceVoice,
  referenceText: petConfig.value.ttsReferenceText,
  speed: petConfig.value.ttsSpeed,
  volume: petConfig.value.ttsVolume,
}))

const { isSpeaking, speakText, stopSpeaking } = useVoicePlayer({
  messages: computed(() => props.messages),
  ttsConfig: voiceConfig,
})

const voiceInput = useVoiceInput({
  isSuspended: computed(() => isSpeaking.value),
  language: computed(() => petConfig.value.asrLanguage),
  onRecognizedText: async (text: string) => {
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

function onTtsToggle(val: boolean) {
  updateConfig({ ttsEnabled: val })
}

async function onVoiceToggle() {
  const nextEnabled = !voiceInput.isEnabled.value
  const applied = await voiceInput.setEnabled(nextEnabled)
  updateConfig({ asrEnabled: nextEnabled && applied })
}

function onTestSpeak() {
  speakText('Diva pet mode preview.')
}

const showModelManager = ref(false)

function onModelChanged(modelId: string) {
  updateConfig({ vrmModel: modelId })
}

const showScenePicker = ref(false)
const isWhisperNearby = ref(false)

const SCENE_ICONS: Record<string, string> = {
  transparent: 'T', home: 'H', sea: 'S', space: '*',
}
function getSceneIcon(id: string) { return SCENE_ICONS[id] ?? '-' }
function selectScene(id: string) {
  updateConfig({ selectedGaussSceneId: id as GaussSceneId })
  showScenePicker.value = false
}

const backgroundSceneUrl = computed(() => {
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
        title="Open Menu"
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
        :background-scene="petConfig.selectedGaussSceneId"
        :background-scene-url="backgroundSceneUrl"
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
        class="pet-glass absolute top-4 left-16 px-3 py-1.5 text-[11px] rounded-full text-white/90 border-white/15 z-20"
      >
        {{ moodLabels[currentMood] }} {{ currentMood }}
      </div>

      <button
        class="pet-edge-button absolute top-4 right-16 z-20"
        title="Manage VRM Models"
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
                <div v-if="msg.content" class="whitespace-pre-wrap">{{ msg.content }}</div>
                <div v-else-if="msg.role === 'agent'" class="flex items-center gap-1.5 text-white/60">
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

            <div v-if="isTyping" class="flex justify-start">
              <div class="pet-agent-bubble rounded-[18px] rounded-bl-md px-3 py-2">
                <div class="flex items-center gap-1.5 text-white/60 text-[10px]">
                  <Loader2 :size="12" class="animate-spin" />
                  {{ currentMood !== 'neutral' ? `Thinking ${moodLabels[currentMood]}` : t('chat.thinking') }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="absolute left-4 right-4 bottom-4 z-20 flex flex-col items-start gap-3">
        <DivaPetVoicePanel
          :is-speaking="isSpeaking"
          :is-voice-supported="voiceInput.isSupported"
          :is-voice-enabled="voiceInput.isEnabled.value"
          :is-listening="voiceInput.isListening.value"
          :is-processing="voiceInput.isProcessing.value"
          :voice-error="voiceInput.error.value"
          :tts-enabled="petConfig.ttsEnabled"
          @toggle-voice="onVoiceToggle"
          @update:tts-enabled="onTtsToggle"
          @test-speak="onTestSpeak"
          @stop-speaking="stopSpeaking"
        />

        <div class="pet-input-dock pet-glass">
          <div class="flex items-center gap-2">
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

.pet-chat-scroll {
  max-height: min(42vh, 360px);
  overflow-y: auto;
  padding-right: 4px;
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
  width: min(680px, 100%);
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
</style>

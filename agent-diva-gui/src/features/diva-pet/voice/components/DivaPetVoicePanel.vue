<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  Mic,
  MicOff,
  Volume2,
  VolumeX,
  Loader2,
  AlertCircle,
  Square,
} from 'lucide-vue-next'

const { t } = useI18n()

function tt(key: string, fallback: string): string {
  const result = t(key)
  return result === key ? fallback : result
}

interface Props {
  isSpeaking: boolean
  isVoiceSupported: boolean
  isVoiceEnabled: boolean
  isListening: boolean
  isProcessing: boolean
  voiceError: string | null
  ttsEnabled: boolean
  isPushToTalkDisabled?: boolean
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'toggleVoice'): void
  (e: 'update:ttsEnabled', val: boolean): void
  (e: 'testSpeak'): void
  (e: 'stopSpeaking'): void
  (e: 'startVoiceHold', event: PointerEvent): void
  (e: 'stopVoiceHold', event: PointerEvent): void
}>()

const ttsLabel = computed(() => tt('pet.voice.tts', '播报'))
const micLabel = computed(() => tt('pet.voice.mic', '麦克风'))
const speakingLabel = computed(() => tt('pet.voice.speaking', '播报中'))
const pushToTalkLabel = computed(() => tt('pet.voice.pushToTalk', '按住说话'))

const ttsTitle = computed(() =>
  props.ttsEnabled ? tt('pet.voice.ttsOff', '关闭播报') : tt('pet.voice.ttsOn', '开启播报'),
)

const micTitle = computed(() => {
  if (!props.isVoiceSupported) return tt('pet.voice.notSupported', '当前环境不支持语音识别')
  if (props.voiceError) return props.voiceError
  if (props.isProcessing) return tt('pet.voice.processing', '识别中...')
  if (props.isListening) return tt('pet.voice.listening', '正在聆听...')
  if (props.isVoiceEnabled) return tt('pet.voice.enabled', '点击关闭语音')
  return tt('pet.voice.disabled', '点击开启语音')
})

const stopTitle = computed(() => tt('pet.voice.stop', '停止播报'))
const pushToTalkTitle = computed(() => (
  props.isPushToTalkDisabled
    ? props.voiceError || tt('pet.voice.notSupported', '当前不可用')
    : pushToTalkLabel.value
))

function onToggleTts() {
  emit('update:ttsEnabled', !props.ttsEnabled)
}

function onToggleVoice() {
  emit('toggleVoice')
}

function onStartVoiceHold(event: PointerEvent) {
  emit('startVoiceHold', event)
}

function onStopVoiceHold(event: PointerEvent) {
  emit('stopVoiceHold', event)
}

function onStopSpeaking() {
  emit('stopSpeaking')
}
</script>

<template>
  <div class="diva-voice-panel flex items-center gap-2 select-none">
    <button
      :title="ttsTitle"
      class="voice-btn voice-btn--glass"
      :class="ttsEnabled ? 'text-cyan-100 ring-1 ring-cyan-200/35' : 'text-white/55'"
      @click="onToggleTts"
    >
      <Volume2 v-if="ttsEnabled" :size="16" />
      <VolumeX v-else :size="16" />
      <span class="voice-label">{{ ttsLabel }}</span>
    </button>

    <div class="relative">
      <button
        :title="micTitle"
        :disabled="!isVoiceSupported"
        class="voice-btn voice-btn--glass"
        :class="[
          isListening
            ? 'text-emerald-100 ring-1 ring-emerald-200/35 animate-pulse'
            : isProcessing
              ? 'text-amber-100 ring-1 ring-amber-200/35'
              : isVoiceEnabled
                ? 'text-white/90'
                : isVoiceSupported
                  ? 'text-white/55'
                  : 'text-white/25 cursor-not-allowed',
        ]"
        @click="onToggleVoice"
      >
        <MicOff v-if="!isVoiceSupported || !isVoiceEnabled" :size="16" />
        <Loader2 v-else-if="isProcessing" :size="16" class="animate-spin" />
        <Mic v-else :size="16" />
        <span class="voice-label">{{ micLabel }}</span>
      </button>

      <AlertCircle
        v-if="voiceError"
        :size="10"
        class="absolute -top-1 -right-1 text-amber-300 drop-shadow-sm"
        :title="voiceError"
      />
    </div>

    <!-- Test voice button is temporarily hidden.
    <button class="voice-btn voice-btn--glass text-fuchsia-100 ring-1 ring-fuchsia-200/30">
      <span class="voice-label">测试</span>
    </button>
    -->

    <button
      :title="pushToTalkTitle"
      :disabled="isPushToTalkDisabled"
      class="voice-btn voice-btn--glass voice-btn--ptt"
      :class="[
        isVoiceEnabled
          ? 'text-white ring-1 ring-red-200/45 voice-btn--ptt-active'
          : isPushToTalkDisabled
            ? 'text-white/25 cursor-not-allowed'
            : 'text-white/70',
      ]"
      @pointerdown.prevent="onStartVoiceHold"
      @pointerup="onStopVoiceHold"
      @pointerleave="onStopVoiceHold"
      @pointercancel="onStopVoiceHold"
    >
      <Mic :size="15" />
      <span class="voice-label">{{ pushToTalkLabel }}</span>
    </button>

    <button
      v-if="isSpeaking"
      :title="stopTitle"
      class="voice-btn voice-btn--glass text-rose-100 ring-1 ring-rose-200/35"
      @click="onStopSpeaking"
    >
      <Square :size="14" />
      <span class="voice-label">{{ speakingLabel }}</span>
    </button>
  </div>
</template>

<style scoped>
.diva-voice-panel {
  font-family: "Segoe UI", "Microsoft YaHei", "PingFang SC", sans-serif;
  width: min(760px, 100%);
}

.voice-btn {
  @apply relative h-10 rounded-full flex items-center justify-center gap-2 px-3 transition-all cursor-pointer;
}

.voice-btn--glass {
  background: linear-gradient(135deg, rgba(255, 255, 255, 0.18), rgba(255, 255, 255, 0.08));
  border: 1px solid rgba(255, 255, 255, 0.18);
  box-shadow: 0 10px 30px rgba(9, 14, 28, 0.18);
  backdrop-filter: blur(18px);
}

.voice-btn--glass:hover {
  background: linear-gradient(135deg, rgba(255, 255, 255, 0.24), rgba(255, 255, 255, 0.12));
  transform: translateY(-1px);
}

.voice-btn--ptt {
  margin-left: auto;
}

.voice-btn--ptt-active {
  background: linear-gradient(135deg, rgba(239, 68, 68, 0.86), rgba(185, 28, 28, 0.70));
  border-color: rgba(252, 165, 165, 0.62);
  box-shadow: 0 10px 30px rgba(185, 28, 28, 0.30);
}

.voice-label {
  font-size: 10px;
  line-height: 1;
  letter-spacing: 0.04em;
}

.voice-btn:disabled {
  @apply cursor-not-allowed;
}
</style>

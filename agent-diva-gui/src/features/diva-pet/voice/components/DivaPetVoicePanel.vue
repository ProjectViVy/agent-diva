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
} from '@lucide/vue'

const { t, te } = useI18n()

function tt(key: string, fallback: string): string {
  return te(key) ? t(key) : fallback
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
      class="ui-button voice-btn voice-btn--glass"
      :class="ttsEnabled ? 'text-primary ring-1 ring-ring' : 'text-foreground'"
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
        class="ui-button ui-button--ghost voice-btn voice-btn--glass"
        :class="[
          isListening
            ? 'text-success ring-1 ring-ring animate-pulse'
            : isProcessing
              ? 'text-warning ring-1 ring-ring'
              : isVoiceEnabled
                ? 'text-foreground'
                : isVoiceSupported
                  ? 'text-foreground'
                  : 'text-foreground cursor-not-allowed',
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
        class="absolute -top-1 -right-1 text-warning drop-shadow-sm"
        :title="voiceError"
      />
    </div>

    <!-- Test voice button is temporarily hidden.
    <button class="ui-button ui-button--ghost voice-btn voice-btn--glass ring-1 ring-ring">
      <span class="voice-label">测试</span>
    </button>
    -->

    <button
      :title="pushToTalkTitle"
      :disabled="isPushToTalkDisabled"
      class="ui-button ui-button--ghost voice-btn voice-btn--glass voice-btn--ptt"
      :class="[
        isVoiceEnabled
          ? 'text-foreground ring-1 ring-ring voice-btn--ptt-active'
          : isPushToTalkDisabled
            ? 'text-foreground cursor-not-allowed'
            : 'text-foreground',
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
      class="ui-button ui-button--ghost voice-btn voice-btn--glass ring-1 ring-ring"
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

.voice-btn--ptt {
  margin-left: auto;
}

.voice-btn--ptt-active {
  background: var(--card);
  border-color: var(--destructive);
  box-shadow: var(--shadow-sm);
}

.voice-label {
  font-size: 10px;
  line-height: 1;
  letter-spacing: 0.04em;
}
</style>

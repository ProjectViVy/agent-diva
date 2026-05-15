import { ref, watch, type Ref } from 'vue'
import { ttsService, type TTSVoiceConfig } from '../services/tts-service'
import type { PetMessage } from '../../types'

/**
 * VoicePlayer Vue 3 composable for Diva Pet.
 *
 * Watches the agent message history and auto-plays TTS audio for new
 * agent messages, exposing `isSpeaking` state that drives VRM
 * mouth-sync via the `:is-speaking` prop.
 *
 * @example
 * ```ts
 * const { isSpeaking, stopSpeaking } = useVoicePlayer({
 *   messages: computed(() => props.messages),
 *   ttsConfig: voiceConfig,
 * })
 * ```
 */
export function useVoicePlayer(options: {
  messages: Ref<PetMessage[]>
  ttsConfig: Ref<TTSVoiceConfig>
}) {
  const { messages, ttsConfig } = options

  const isSpeaking = ref(false)

  /** Monotonically-increasing token used to cancel stale speech operations. */
  let speechId = 0

  /** Speak a single message via the TTS service. */
  async function speakMessage(text: string, config: TTSVoiceConfig): Promise<void> {
    const id = ++speechId
    isSpeaking.value = true
    try {
      await ttsService.speakText(text, config)
    } finally {
      // Only deactivate speaking when no newer speech has been started.
      if (speechId === id) {
        isSpeaking.value = false
      }
    }
  }

  /** Halt any in-progress speech and reset speaking state. */
  function stopSpeaking(): void {
    speechId++
    ttsService.stopPlayback()
    isSpeaking.value = false
  }

  /** Manually speak arbitrary text with the current TTS configuration. */
  function speakText(text: string): void {
    const trimmed = text.trim()
    if (!trimmed) return

    stopSpeaking()
    void speakMessage(trimmed, ttsConfig.value)
  }

  watch(
    ttsConfig,
    (config) => {
      void ttsService.prepareVoiceClone(config).catch((error: unknown) => {
        console.warn('[VoicePlayer] Failed to prepare voice clone.', error)
      })
    },
    { deep: true, immediate: true },
  )

  // ── Message watcher ────────────────────────────────────────────
  let lastMessageCount = 0

  watch(
    messages,
    (currentMessages: PetMessage[]) => {
      if (currentMessages.length <= lastMessageCount) {
        lastMessageCount = currentMessages.length
        return
      }

      const newMessages = currentMessages.slice(lastMessageCount)
      lastMessageCount = currentMessages.length

      const config = ttsConfig.value
      if (!config?.enabled) {
        return
      }

      for (const msg of newMessages) {
        if (msg.role !== 'agent') {
          continue
        }
        if (!msg.content || msg.content.trim().length === 0) {
          continue
        }

        // Stop any previously-playing speech before starting the new one.
        stopSpeaking()
        void speakMessage(msg.content, config)
      }
    },
    { deep: false },
  )

  // ── Public API ─────────────────────────────────────────────────
  return {
    /** Ref<boolean> for driving VRM mouth-sync. */
    isSpeaking,
    /** Manually speak arbitrary text with the current TTS configuration. */
    speakText,
    /** Manually halt speech and reset the speaking state. */
    stopSpeaking,
  }
}

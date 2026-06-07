import { ref, watch, type Ref } from 'vue'
import { ttsService, type TTSVoiceConfig } from '../services/tts-service'
import { filterPunctuation, splitIntoSentences, stripMarkdown } from '../utils/text-preprocessor'
import type { PetMessage } from '../../types'

/**
 * VoicePlayer Vue 3 composable for Diva Pet.
 *
 * Watches the agent message history and auto-plays TTS audio for new
 * agent messages, exposing `isSpeaking` state that drives VRM
 * mouth-sync via the `:is-speaking` prop.
 *
 * Two triggers for auto-play:
 * 1. New messages pushed to the array (legacy, for non-streaming scenarios)
 * 2. `isTyping` transitions from `true` to `false` (primary: streaming completion)
 *
 * @example
 * ```ts
 * const { isSpeaking, stopSpeaking } = useVoicePlayer({
 *   messages: computed(() => props.messages),
 *   isTyping: computed(() => props.isTyping),
 *   ttsConfig: voiceConfig,
 * })
 * ```
 */
export function useVoicePlayer(options: {
  messages: Ref<PetMessage[]>
  isTyping?: Ref<boolean>
  ttsConfig: Ref<TTSVoiceConfig>
}) {
  const { messages, isTyping, ttsConfig } = options
  const typingState = isTyping ?? ref(false)

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

        // Preprocess: strip markdown, then split into sentences
        // Speak each sentence sequentially so TTS sounds natural.
        const plainText = stripMarkdown(msg.content)
        const filteredText = filterPunctuation(plainText)
        const segments = splitIntoSentences(filteredText)

        void (async () => {
          for (const segment of segments) {
            await speakMessage(segment.text, config)
          }
        })()
      }
    },
    { deep: false },
  )

  // ── isTyping watcher ───────────────────────────────────────────
  // Triggers TTS when streaming completes (isTyping true → false).
  watch(
    typingState,
    (current, previous) => {
      if (previous !== true || current !== false) {
        return
      }

      const config = ttsConfig.value
      if (!config?.enabled) {
        return
      }

      // Find the most recent agent message with non-empty content
      const lastAgentMsg = [...messages.value]
        .reverse()
        .find((m) => m.role === 'agent' && m.content?.trim())

      if (!lastAgentMsg?.content) {
        return
      }

      // Only speak if it hasn't already been picked up by the messages watcher.
      // Guard: compare partial content snapshot to avoid double-play.
      const contentSnapshot = lastAgentMsg.content.trim()
      if (contentSnapshot.length === 0) {
        return
      }

      stopSpeaking()

      const plainText = stripMarkdown(contentSnapshot)
      const filteredText = filterPunctuation(plainText)
      const segments = splitIntoSentences(filteredText)

      void (async () => {
        for (const segment of segments) {
          await speakMessage(segment.text, config)
        }
      })()
    },
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

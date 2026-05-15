import {
  computed,
  onUnmounted,
  ref,
  toValue,
  watch,
  type ComputedRef,
  type MaybeRef,
  type Ref,
} from 'vue'
import { addVoiceLogEvent } from '../services/voice-log'

const VOICE_INPUT_RESTART_DELAY_MS = 420

interface SpeechRecognitionAlternativeLike {
  transcript: string
}

interface SpeechRecognitionResultLike {
  readonly isFinal: boolean
  readonly length: number
  [index: number]: SpeechRecognitionAlternativeLike
}

interface SpeechRecognitionResultListLike {
  readonly length: number
  [index: number]: SpeechRecognitionResultLike
}

interface SpeechRecognitionEventLike extends Event {
  readonly resultIndex: number
  readonly results: SpeechRecognitionResultListLike
}

interface SpeechRecognitionErrorEventLike extends Event {
  readonly error: string
}

interface SpeechRecognitionLike extends EventTarget {
  continuous: boolean
  interimResults: boolean
  lang: string
  maxAlternatives: number
  onend: ((event: Event) => void) | null
  onerror: ((event: SpeechRecognitionErrorEventLike) => void) | null
  onresult: ((event: SpeechRecognitionEventLike) => void) | null
  onstart: ((event: Event) => void) | null
  abort: () => void
  start: () => void
  stop: () => void
}

interface SpeechRecognitionConstructor {
  new (): SpeechRecognitionLike
}

declare global {
  interface Window {
    SpeechRecognition?: SpeechRecognitionConstructor
    webkitSpeechRecognition?: SpeechRecognitionConstructor
  }
}

export interface UseVoiceInputOptions {
  isSuspended?: MaybeRef<boolean>
  language?: MaybeRef<string>
  onPreviewText?: (text: string) => void
  onRecognizedText: (text: string) => Promise<void> | void
}

export interface UseVoiceInputResult {
  error: Ref<string | null>
  isEnabled: Ref<boolean>
  isListening: Ref<boolean>
  isProcessing: Ref<boolean>
  isSupported: boolean
  note: ComputedRef<string | null>
  pauseFor: (durationMs: number) => void
  setEnabled: (enabled: boolean) => Promise<boolean>
  toggle: () => Promise<boolean>
}

function getSpeechRecognitionConstructor(): SpeechRecognitionConstructor | null {
  if (typeof window === 'undefined') return null
  return window.SpeechRecognition ?? window.webkitSpeechRecognition ?? null
}

function normalizeTranscript(text: string): string {
  return text.replace(/\s+/g, ' ').trim()
}

function describeVoiceInputError(error: string): string {
  switch (error) {
    case 'audio-capture':
      return '没有检测到可用麦克风。'
    case 'network':
      return '语音识别暂时不可用，请检查网络后重试。'
    case 'not-allowed':
    case 'service-not-allowed':
      return '没有获得麦克风权限，请在系统或浏览器设置中允许 Agent Diva 使用麦克风。'
    default:
      return '语音输入暂时不可用，请稍后重试。'
  }
}

function isMicrophonePermissionError(error: unknown): boolean {
  if (!(error instanceof DOMException)) return false
  return error.name === 'NotAllowedError' || error.name === 'SecurityError' || error.name === 'PermissionDeniedError'
}

export function useVoiceInput(options: UseVoiceInputOptions): UseVoiceInputResult {
  const recognitionConstructor = getSpeechRecognitionConstructor()
  const isSupported = recognitionConstructor !== null

  const isEnabled = ref(false)
  const isListening = ref(false)
  const isProcessing = ref(false)
  const error = ref<string | null>(null)

  const recognitionRef = ref<SpeechRecognitionLike | null>(null)
  const restartTimeoutRef = ref<number | null>(null)
  const resumeAtRef = ref(0)
  const suspendedRef = ref(toValue(options.isSuspended) ?? false)
  const isRunningRef = ref(false)
  const stoppedForDisableRef = ref(false)
  const hasMicrophoneGrantRef = ref(false)
  const onPreviewTextRef = ref(options.onPreviewText)
  const onRecognizedTextRef = ref(options.onRecognizedText)
  const languageRef = ref(toValue(options.language) ?? 'zh-CN')

  if (!isSupported) {
    addVoiceLogEvent({
      level: 'warn',
      source: 'asr',
      message: '当前环境不支持 Web Speech API',
      detail: { provider: 'web_speech' },
    })
  }

  watch(
    () => toValue(options.isSuspended),
    (val) => {
      suspendedRef.value = val ?? false
    },
  )

  watch(
    () => options.onPreviewText,
    (val) => {
      onPreviewTextRef.value = val
    },
  )

  watch(
    () => options.onRecognizedText,
    (val) => {
      onRecognizedTextRef.value = val
    },
  )

  watch(
    () => toValue(options.language),
    (val) => {
      languageRef.value = val ?? 'zh-CN'
    },
  )

  function clearRestartTimeout(): void {
    if (restartTimeoutRef.value !== null) {
      window.clearTimeout(restartTimeoutRef.value)
      restartTimeoutRef.value = null
    }
  }

  async function requestMicrophonePermission(): Promise<boolean> {
    if (hasMicrophoneGrantRef.value) return true
    if (typeof navigator === 'undefined' || !navigator.mediaDevices?.getUserMedia) {
      return true
    }

    let stream: MediaStream | null = null
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true })
      hasMicrophoneGrantRef.value = true
      addVoiceLogEvent({
        level: 'info',
        source: 'asr',
        message: '麦克风权限已确认',
        detail: { provider: 'web_speech' },
      })
      return true
    } catch (permissionError) {
      const nextError = isMicrophonePermissionError(permissionError)
        ? describeVoiceInputError('not-allowed')
        : describeVoiceInputError('audio-capture')
      error.value = nextError
      addVoiceLogEvent({
        level: 'error',
        source: 'asr',
        message: '麦克风权限请求失败',
        detail: {
          code: permissionError instanceof DOMException ? permissionError.name : 'unknown',
          error: nextError,
        },
      })
      return false
    } finally {
      stream?.getTracks().forEach((track) => track.stop())
    }
  }

  function scheduleRecognitionStart(): void {
    clearRestartTimeout()
    if (!isEnabled.value || suspendedRef.value || isProcessing.value || recognitionConstructor === null) {
      return
    }

    const remainingPauseMs = Math.max(0, resumeAtRef.value - Date.now())
    const startDelayMs = Math.max(VOICE_INPUT_RESTART_DELAY_MS, remainingPauseMs)

    restartTimeoutRef.value = window.setTimeout(() => {
      restartTimeoutRef.value = null
      if (!isEnabled.value || suspendedRef.value || isProcessing.value) return
      if (Date.now() < resumeAtRef.value) {
        scheduleRecognitionStart()
        return
      }

      const recognition = recognitionRef.value ?? new recognitionConstructor()
      recognition.continuous = false
      recognition.interimResults = true
      recognition.lang = languageRef.value
      recognition.maxAlternatives = 1

      recognition.onstart = () => {
        stoppedForDisableRef.value = false
        isRunningRef.value = true
        isListening.value = true
        error.value = null
        addVoiceLogEvent({
          level: 'info',
          source: 'asr',
          message: '开始语音识别',
          detail: { provider: 'web_speech', language: languageRef.value },
        })
      }

      recognition.onresult = (event: SpeechRecognitionEventLike) => {
        if (isProcessing.value) return

        const finalParts: string[] = []
        for (let index = event.resultIndex; index < event.results.length; index += 1) {
          const result = event.results[index]
          const transcript = normalizeTranscript(result?.[0]?.transcript ?? '')
          if (transcript && result?.isFinal) finalParts.push(transcript)
        }

        const finalTranscript = normalizeTranscript(finalParts.join(' '))
        if (!finalTranscript) return

        isProcessing.value = true
        onPreviewTextRef.value?.(finalTranscript)
        addVoiceLogEvent({
          level: 'info',
          source: 'asr',
          message: '识别到语音输入',
          detail: { text: finalTranscript.slice(0, 80) },
        })

        try {
          recognition.stop()
        } catch (stopError) {
          console.warn('[VoiceInput] Failed to stop recognition after result.', stopError)
        }

        void Promise.resolve(onRecognizedTextRef.value(finalTranscript))
          .catch((submitError: unknown) => {
            console.error('[VoiceInput] Failed to submit recognized text.', submitError)
            error.value = submitError instanceof Error ? submitError.message : '语音输入发送失败，请重试。'
            addVoiceLogEvent({
              level: 'error',
              source: 'asr',
              message: '语音输入发送失败',
              detail: { error: error.value },
            })
          })
          .finally(() => {
            isProcessing.value = false
            if (isEnabled.value && !isRunningRef.value) scheduleRecognitionStart()
          })
      }

      recognition.onerror = (event: SpeechRecognitionErrorEventLike) => {
        if (event.error === 'aborted') return
        if (event.error === 'no-speech') {
          error.value = null
          return
        }

        const nextError = describeVoiceInputError(event.error)
        error.value = nextError
        addVoiceLogEvent({
          level: 'error',
          source: 'asr',
          message: '语音识别错误',
          detail: { code: event.error, error: nextError },
        })

        if (event.error === 'audio-capture' || event.error === 'not-allowed' || event.error === 'service-not-allowed') {
          stoppedForDisableRef.value = true
          isEnabled.value = false
        }
      }

      recognition.onend = () => {
        isRunningRef.value = false
        isListening.value = false
        addVoiceLogEvent({
          level: 'info',
          source: 'asr',
          message: '语音识别结束',
          detail: { enabled: isEnabled.value },
        })

        if (stoppedForDisableRef.value) {
          stoppedForDisableRef.value = false
          return
        }
        if (isEnabled.value && !isProcessing.value) scheduleRecognitionStart()
      }

      recognitionRef.value = recognition
      try {
        recognition.start()
      } catch (startError) {
        const message = startError instanceof Error ? startError.message : String(startError)
        if (!message.toLowerCase().includes('already started')) {
          console.warn('[VoiceInput] Failed to start recognition.', startError)
          error.value = '语音输入暂时无法启动，请稍后再试。'
          isEnabled.value = false
          addVoiceLogEvent({
            level: 'error',
            source: 'asr',
            message: '语音识别启动失败',
            detail: { error: message },
          })
        }
      }
    }, startDelayMs)
  }

  watch([isEnabled, suspendedRef, languageRef], ([enabled, suspended]) => {
    if (!isSupported) return

    if (!enabled || suspended) {
      clearRestartTimeout()
      stoppedForDisableRef.value = true
      if (recognitionRef.value && isRunningRef.value) {
        try {
          recognitionRef.value.stop()
        } catch (stopError) {
          console.warn('[VoiceInput] Failed to stop recognition.', stopError)
        }
      }
      isListening.value = false
      if (!enabled) isProcessing.value = false
      return
    }

    scheduleRecognitionStart()
  })

  onUnmounted(() => {
    if (!isSupported) return
    isEnabled.value = false
    clearRestartTimeout()
    if (!recognitionRef.value) return
    try {
      recognitionRef.value.abort()
    } catch (abortError) {
      console.warn('[VoiceInput] Failed to abort recognition during cleanup.', abortError)
    }
  })

  function pauseFor(durationMs: number): void {
    if (!Number.isFinite(durationMs) || durationMs <= 0) return
    resumeAtRef.value = Math.max(resumeAtRef.value, Date.now() + durationMs)
    clearRestartTimeout()

    if (recognitionRef.value && isRunningRef.value) {
      try {
        recognitionRef.value.stop()
      } catch (stopError) {
        console.warn('[VoiceInput] Failed to pause recognition.', stopError)
      }
      return
    }

    if (isEnabled.value && !isProcessing.value) scheduleRecognitionStart()
  }

  async function setEnabled(enabled: boolean): Promise<boolean> {
    if (enabled && !isSupported) {
      error.value = '当前环境暂不支持语音输入。'
      addVoiceLogEvent({
        level: 'warn',
        source: 'asr',
        message: '无法开启语音输入：当前环境不支持 Web Speech API',
        detail: { provider: 'web_speech' },
      })
      return false
    }

    if (enabled) {
      const hasPermission = await requestMicrophonePermission()
      if (!hasPermission) {
        isEnabled.value = false
        isListening.value = false
        isProcessing.value = false
        return false
      }
    }

    error.value = null
    isEnabled.value = enabled
    addVoiceLogEvent({
      level: 'info',
      source: 'asr',
      message: enabled ? '语音输入已开启' : '语音输入已关闭',
      detail: { provider: 'web_speech', language: languageRef.value },
    })
    return true
  }

  function toggle(): Promise<boolean> {
    return setEnabled(!isEnabled.value)
  }

  return {
    error,
    isEnabled,
    isListening,
    isProcessing,
    isSupported,
    note: computed(() => error.value),
    pauseFor,
    setEnabled,
    toggle,
  }
}

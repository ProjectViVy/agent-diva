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
import type { ASRProviderConfig } from '../services/asr-service'
import { isCloudAsrProvider, transcribeWithAsrProvider } from '../services/asr-service'
import { addVoiceLogEvent } from '../services/voice-log'

const VOICE_INPUT_RESTART_DELAY_MS = 420
const CLOUD_ASR_SILENCE_MS = 1200
const CLOUD_ASR_MAX_RECORDING_MS = 12000
const CLOUD_ASR_IDLE_TIMEOUT_MS = 6000
const CLOUD_ASR_MONITOR_INTERVAL_MS = 180
const CLOUD_ASR_ACTIVITY_THRESHOLD = 10

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
  config?: MaybeRef<ASRProviderConfig>
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
  const isWebSpeechSupported = recognitionConstructor !== null
  const isCloudSupported = typeof window !== 'undefined'
    && typeof navigator !== 'undefined'
    && !!navigator.mediaDevices?.getUserMedia
    && typeof MediaRecorder !== 'undefined'
  const isSupported = isWebSpeechSupported || isCloudSupported

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
  const mediaStreamRef = ref<MediaStream | null>(null)
  const mediaRecorderRef = ref<MediaRecorder | null>(null)
  const monitorIntervalRef = ref<number | null>(null)
  const recordingTimeoutRef = ref<number | null>(null)
  const audioContextRef = ref<AudioContext | null>(null)
  const analyserRef = ref<AnalyserNode | null>(null)
  const chunksRef = ref<Blob[]>([])
  const speechDetectedRef = ref(false)
  const silenceStartedAtRef = ref<number | null>(null)
  const recordingStartedAtRef = ref<number | null>(null)
  const onPreviewTextRef = ref(options.onPreviewText)
  const onRecognizedTextRef = ref(options.onRecognizedText)
  const configRef = ref<ASRProviderConfig>(toValue(options.config) ?? {
    provider: 'web_speech',
    language: 'zh-CN',
    apiKey: null,
    baseUrl: '',
    model: null,
  })

  if (!isWebSpeechSupported) {
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
    () => toValue(options.config),
    (val) => {
      configRef.value = val ?? {
        provider: 'web_speech',
        language: 'zh-CN',
        apiKey: null,
        baseUrl: '',
        model: null,
      }
    },
    { deep: true },
  )

  function clearRestartTimeout(): void {
    if (restartTimeoutRef.value !== null) {
      window.clearTimeout(restartTimeoutRef.value)
      restartTimeoutRef.value = null
    }
  }


  function clearCloudTimers(): void {
    if (monitorIntervalRef.value !== null) {
      window.clearInterval(monitorIntervalRef.value)
      monitorIntervalRef.value = null
    }
    if (recordingTimeoutRef.value !== null) {
      window.clearTimeout(recordingTimeoutRef.value)
      recordingTimeoutRef.value = null
    }
    silenceStartedAtRef.value = null
    recordingStartedAtRef.value = null
  }

  function stopCloudStream(): void {
    mediaStreamRef.value?.getTracks().forEach((track) => track.stop())
    mediaStreamRef.value = null
  }

  async function ensureCloudAudioGraph(stream: MediaStream): Promise<void> {
    if (!audioContextRef.value) {
      audioContextRef.value = new AudioContext()
    }
    if (audioContextRef.value.state === 'suspended') {
      await audioContextRef.value.resume()
    }

    const source = audioContextRef.value.createMediaStreamSource(stream)
    const analyser = audioContextRef.value.createAnalyser()
    analyser.fftSize = 1024
    analyser.smoothingTimeConstant = 0.85
    source.connect(analyser)
    analyserRef.value = analyser
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
      if (isCloudAsrProvider(configRef.value.provider)) {
        mediaStreamRef.value = stream
        stream = null
      }
      addVoiceLogEvent({
        level: 'info',
        source: 'asr',
        message: '麦克风权限已确认',
        detail: { provider: configRef.value.provider },
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

  function describeUnsupportedProvider(provider: ASRProviderConfig['provider']): string {
    if (provider === 'web_speech') {
      return '当前环境暂不支持 Web Speech API。'
    }
    return '当前环境暂不支持云端 ASR 录音能力。'
  }

  function cleanupCloudRecorder(): void {
    clearCloudTimers()
    chunksRef.value = []
    speechDetectedRef.value = false
    if (mediaRecorderRef.value) {
      mediaRecorderRef.value.ondataavailable = null
      mediaRecorderRef.value.onerror = null
      mediaRecorderRef.value.onstart = null
      mediaRecorderRef.value.onstop = null
      mediaRecorderRef.value = null
    }
    analyserRef.value = null
    isRunningRef.value = false
    isListening.value = false
  }

  async function finishCloudCapture(): Promise<void> {
    const blob = new Blob(chunksRef.value, {
      type: mediaRecorderRef.value?.mimeType || chunksRef.value[0]?.type || 'audio/webm',
    })
    const hadSpeech = speechDetectedRef.value
    cleanupCloudRecorder()

    if (!hadSpeech || blob.size === 0) {
      if (isEnabled.value && !isProcessing.value) scheduleRecognitionStart()
      return
    }

    isProcessing.value = true
    try {
      const text = await transcribeWithAsrProvider({
        audioBlob: blob,
        config: configRef.value,
      })
      const finalTranscript = normalizeTranscript(text)
      if (finalTranscript) {
        onPreviewTextRef.value?.(finalTranscript)
        addVoiceLogEvent({
          level: 'info',
          source: 'asr',
          message: '识别到语音输入',
          detail: {
            provider: configRef.value.provider,
            text: finalTranscript.slice(0, 80),
          },
        })
        await Promise.resolve(onRecognizedTextRef.value(finalTranscript))
      }
    } catch (submitError) {
      error.value = submitError instanceof Error ? submitError.message : '语音输入发送失败，请重试。'
      addVoiceLogEvent({
        level: 'error',
        source: 'asr',
        message: '云端 ASR 处理失败',
        detail: { provider: configRef.value.provider, error: error.value },
      })
    } finally {
      isProcessing.value = false
      if (isEnabled.value && !isRunningRef.value) scheduleRecognitionStart()
    }
  }

  function stopCloudRecording(): void {
    const recorder = mediaRecorderRef.value
    if (!recorder || recorder.state === 'inactive') {
      cleanupCloudRecorder()
      return
    }
    try {
      recorder.stop()
    } catch (stopError) {
      console.warn('[VoiceInput] Failed to stop cloud recorder.', stopError)
      cleanupCloudRecorder()
    }
  }

  async function scheduleCloudRecognitionStart(): Promise<void> {
    if (!mediaStreamRef.value) {
      mediaStreamRef.value = await navigator.mediaDevices.getUserMedia({ audio: true })
    }
    const stream = mediaStreamRef.value
    if (!stream) {
      throw new Error('云端 ASR 无法获取麦克风输入。')
    }

    await ensureCloudAudioGraph(stream)
    const recorder = new MediaRecorder(stream)
    chunksRef.value = []
    speechDetectedRef.value = false
    silenceStartedAtRef.value = null
    recordingStartedAtRef.value = Date.now()

    recorder.onstart = () => {
      stoppedForDisableRef.value = false
      isRunningRef.value = true
      isListening.value = true
      error.value = null
      addVoiceLogEvent({
        level: 'info',
        source: 'asr',
        message: '开始云端语音识别',
        detail: {
          provider: configRef.value.provider,
          language: configRef.value.language,
        },
      })
    }

    recorder.ondataavailable = (event: BlobEvent) => {
      if (event.data.size > 0) {
        chunksRef.value.push(event.data)
      }
    }

    recorder.onerror = (event: Event) => {
      const recorderEvent = event as Event & { error?: DOMException }
      error.value = recorderEvent.error?.message || '云端 ASR 录音失败，请稍后重试。'
      addVoiceLogEvent({
        level: 'error',
        source: 'asr',
        message: '云端 ASR 录音失败',
        detail: { provider: configRef.value.provider, error: error.value },
      })
      cleanupCloudRecorder()
    }

    recorder.onstop = () => {
      void finishCloudCapture()
    }

    mediaRecorderRef.value = recorder
    recorder.start()

    monitorIntervalRef.value = window.setInterval(() => {
      const analyser = analyserRef.value
      if (!analyser) return
      const buffer = new Uint8Array(analyser.frequencyBinCount)
      analyser.getByteFrequencyData(buffer)
      const average = buffer.reduce((sum, value) => sum + value, 0) / Math.max(buffer.length, 1)
      const now = Date.now()
      if (average >= CLOUD_ASR_ACTIVITY_THRESHOLD) {
        speechDetectedRef.value = true
        silenceStartedAtRef.value = null
        return
      }
      if (!speechDetectedRef.value) {
        if ((recordingStartedAtRef.value ?? now) + CLOUD_ASR_IDLE_TIMEOUT_MS <= now) {
          stopCloudRecording()
        }
        return
      }
      if (silenceStartedAtRef.value === null) {
        silenceStartedAtRef.value = now
        return
      }
      if (now - silenceStartedAtRef.value >= CLOUD_ASR_SILENCE_MS) {
        stopCloudRecording()
      }
    }, CLOUD_ASR_MONITOR_INTERVAL_MS)

    recordingTimeoutRef.value = window.setTimeout(() => {
      stopCloudRecording()
    }, CLOUD_ASR_MAX_RECORDING_MS)
  }

  function scheduleRecognitionStart(): void {
    clearRestartTimeout()
      if (!isEnabled.value || suspendedRef.value || isProcessing.value || recognitionConstructor === null) {
      if (!isCloudAsrProvider(configRef.value.provider)) return
      if (!isCloudSupported) return
      if (isEnabled.value && !suspendedRef.value && !isProcessing.value) {
        // cloud ASR does not depend on SpeechRecognition constructor
      }
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

      if (isCloudAsrProvider(configRef.value.provider)) {
        void scheduleCloudRecognitionStart().catch((startError: unknown) => {
          const message = startError instanceof Error ? startError.message : String(startError)
          console.warn('[VoiceInput] Failed to start cloud ASR.', startError)
          error.value = message || '云端 ASR 无法启动，请稍后重试。'
          isEnabled.value = false
          isListening.value = false
          isRunningRef.value = false
          addVoiceLogEvent({
            level: 'error',
            source: 'asr',
            message: '云端 ASR 启动失败',
            detail: { provider: configRef.value.provider, error: error.value },
          })
        })
        return
      }

      if (recognitionConstructor === null) {
        return
      }

      const recognition = recognitionRef.value ?? new recognitionConstructor()
      recognition.continuous = false
      recognition.interimResults = true
      recognition.lang = configRef.value.language
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
          detail: { provider: configRef.value.provider, language: configRef.value.language },
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
          detail: { enabled: isEnabled.value, provider: configRef.value.provider },
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

  watch([
    isEnabled,
    suspendedRef,
    () => configRef.value.provider,
    () => configRef.value.language,
    () => configRef.value.baseUrl,
    () => configRef.value.model,
    () => configRef.value.apiKey,
  ], ([enabled, suspended]) => {
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
      stopCloudRecording()
      clearCloudTimers()
      isListening.value = false
      if (!enabled) isProcessing.value = false
      return
    }

    scheduleRecognitionStart()
  })

  onUnmounted(() => {
    isEnabled.value = false
    clearRestartTimeout()
    stopCloudRecording()
    stopCloudStream()
    if (audioContextRef.value) {
      void audioContextRef.value.close().catch(() => undefined)
    }
    if (recognitionRef.value) {
      try {
        recognitionRef.value.abort()
      } catch (abortError) {
        console.warn('[VoiceInput] Failed to abort recognition during cleanup.', abortError)
      }
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
    const provider = configRef.value.provider
    const providerSupported = provider === 'web_speech' ? isWebSpeechSupported : isCloudSupported
    if (enabled && !providerSupported) {
      error.value = describeUnsupportedProvider(provider)
      addVoiceLogEvent({
        level: 'warn',
        source: 'asr',
        message: '无法开启语音输入：当前环境不支持所选 ASR Provider',
        detail: { provider },
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
      detail: { provider, language: configRef.value.language },
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

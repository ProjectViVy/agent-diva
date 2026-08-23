import { afterEach, describe, expect, it, vi } from 'vitest'
import { useVoiceInput } from './useVoiceInput'

class MockSpeechRecognition extends EventTarget {
  continuous = false
  interimResults = false
  lang = 'zh-CN'
  maxAlternatives = 1
  onend: ((event: Event) => void) | null = null
  onerror: ((event: Event) => void) | null = null
  onresult: ((event: Event) => void) | null = null
  onstart: ((event: Event) => void) | null = null

  abort = vi.fn()
  start = vi.fn()
  stop = vi.fn()
}

describe('useVoiceInput', () => {
  afterEach(() => {
    vi.restoreAllMocks()
    Reflect.deleteProperty(window, 'SpeechRecognition')
    Reflect.deleteProperty(window, 'webkitSpeechRecognition')
  })

  it('does not enable voice input when microphone permission is denied', async () => {
    Object.defineProperty(window, 'webkitSpeechRecognition', {
      configurable: true,
      value: MockSpeechRecognition,
    })
    Object.defineProperty(navigator, 'mediaDevices', {
      configurable: true,
      value: {
        getUserMedia: vi.fn().mockRejectedValue(new DOMException('denied', 'NotAllowedError')),
      },
    })

    const voiceInput = useVoiceInput({
      onRecognizedText: vi.fn(),
    })

    const enabled = await voiceInput.setEnabled(true)

    expect(enabled).toBe(false)
    expect(voiceInput.isEnabled.value).toBe(false)
    expect(voiceInput.error.value).toContain('没有获得麦克风权限')
    expect(navigator.mediaDevices.getUserMedia).toHaveBeenCalledWith({ audio: true })
  })

  it('enables voice input after microphone permission is granted', async () => {
    Object.defineProperty(window, 'webkitSpeechRecognition', {
      configurable: true,
      value: MockSpeechRecognition,
    })
    Object.defineProperty(navigator, 'mediaDevices', {
      configurable: true,
      value: {
        getUserMedia: vi.fn().mockResolvedValue({
          getTracks: () => [{ stop: vi.fn() }],
        }),
      },
    })

    const voiceInput = useVoiceInput({
      onRecognizedText: vi.fn(),
    })

    const enabled = await voiceInput.setEnabled(true)

    expect(enabled).toBe(true)
    expect(voiceInput.isEnabled.value).toBe(true)
    expect(voiceInput.error.value).toBeNull()
  })
})

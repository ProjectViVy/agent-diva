import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

import { invoke } from '@tauri-apps/api/core'
import { ttsService, type TTSVoiceConfig } from './tts-service'

const invokeMock = vi.mocked(invoke)

function createMiniMaxConfig(overrides: Partial<TTSVoiceConfig> = {}): TTSVoiceConfig {
  return {
    enabled: true,
    provider: 'minimax',
    apiKey: 'test-key',
    baseUrl: '',
    model: null,
    voiceId: null,
    referenceVoice: null,
    referenceText: null,
    speed: 1,
    volume: 1,
    ...overrides,
  }
}

function createSiliconFlowConfig(overrides: Partial<TTSVoiceConfig> = {}): TTSVoiceConfig {
  return {
    enabled: true,
    provider: 'siliconflow',
    apiKey: 'sf-key',
    baseUrl: '',
    model: null,
    voiceId: null,
    referenceVoice: null,
    referenceText: null,
    speed: 1,
    volume: 1,
    ...overrides,
  }
}

describe('ttsService MiniMax', () => {
  beforeEach(() => {
    invokeMock.mockReset()
    vi.stubGlobal('URL', {
      createObjectURL: vi.fn().mockReturnValue('blob:minimax-audio'),
      revokeObjectURL: vi.fn(),
    })
    vi.stubGlobal('SpeechSynthesisUtterance', class MockSpeechSynthesisUtterance {
      lang = 'zh-CN'
      onend: ((event: Event) => void) | null = null
      onerror: ((event: Event) => void) | null = null
      rate = 1
      text: string
      volume = 1

      constructor(text: string) {
        this.text = text
      }
    })
    Object.defineProperty(window, 'speechSynthesis', {
      configurable: true,
      value: {
        cancel: vi.fn(),
        getVoices: vi.fn().mockReturnValue([]),
        speak: vi.fn((utterance: SpeechSynthesisUtterance) => {
          utterance.onend?.(new Event('end') as SpeechSynthesisEvent)
        }),
      },
    })
  })

  afterEach(() => {
    ttsService.stopPlayback()
    vi.restoreAllMocks()
    vi.unstubAllGlobals()
  })

  it('calls the Tauri MiniMax command with default provider values', async () => {
    invokeMock.mockResolvedValue({
      base64Data: btoa('minimax-audio'),
      contentType: 'audio/mpeg',
    })

    const response = await ttsService.synthesize(
      { text: '你好，MiniMax' },
      createMiniMaxConfig(),
    )

    expect(response?.audioUrl).toBe('blob:minimax-audio')
    expect(invokeMock).toHaveBeenCalledWith(
      'pet_minimax_synthesize',
      expect.objectContaining({
        payload: expect.objectContaining({
          apiKey: 'test-key',
          baseUrl: 'https://api.minimaxi.com',
          model: 'speech-2.8-hd',
          speed: 1,
          text: '你好，MiniMax',
          volume: 1,
          voiceId: 'male-qn-qingse',
        }),
      }),
    )
  })

  it('falls back to browser speech when MiniMax synthesis fails', async () => {
    const speakMock = vi.fn((utterance: SpeechSynthesisUtterance) => {
      utterance.onend?.(new Event('end') as SpeechSynthesisEvent)
    })
    Object.defineProperty(window, 'speechSynthesis', {
      configurable: true,
      value: {
        cancel: vi.fn(),
        getVoices: vi.fn().mockReturnValue([]),
        speak: speakMock,
      },
    })
    invokeMock.mockRejectedValue(new Error('boom'))

    await ttsService.speakText('fallback path', createMiniMaxConfig())

    expect(invokeMock).toHaveBeenCalledOnce()
    expect(speakMock).toHaveBeenCalled()
  })
})

describe('ttsService SiliconFlow', () => {
  beforeEach(() => {
    invokeMock.mockReset()
    vi.stubGlobal('URL', {
      createObjectURL: vi.fn().mockReturnValue('blob:siliconflow-audio'),
      revokeObjectURL: vi.fn(),
    })
  })

  afterEach(() => {
    ttsService.stopPlayback()
    vi.restoreAllMocks()
    vi.unstubAllGlobals()
  })

  it('calls the SiliconFlow factory handler through the Tauri command', async () => {
    invokeMock.mockResolvedValue({
      base64Data: btoa('siliconflow-audio'),
      contentType: 'audio/mpeg',
    })

    const response = await ttsService.synthesize(
      { text: '你好，SiliconFlow' },
      createSiliconFlowConfig(),
    )

    expect(response?.audioUrl).toBe('blob:siliconflow-audio')
    expect(invokeMock).toHaveBeenCalledWith(
      'pet_siliconflow_synthesize',
      expect.objectContaining({
        payload: expect.objectContaining({
          apiKey: 'sf-key',
          baseUrl: 'https://api.siliconflow.cn/v1',
          model: 'fnlp/MOSS-TTSD-v0.5',
          text: '你好，SiliconFlow',
          voice: 'fnlp/MOSS-TTSD-v0.5:anna',
        }),
      }),
    )
  })
})

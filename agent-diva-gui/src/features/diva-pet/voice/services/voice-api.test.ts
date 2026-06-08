import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

import { invoke } from '@tauri-apps/api/core'
import { DEFAULT_PET_CONFIG } from '../../types'
import {
  DEFAULT_SILICONFLOW_ASR_BASE_URL,
  DEFAULT_SILICONFLOW_ASR_MODEL,
  getAsrProviderDefaults,
  loadPetConfigFromCore,
  resolveAsrTranscriptionConfig,
  savePetConfigToCore,
} from './voice-api'

const invokeMock = vi.mocked(invoke)

describe('voice-api ASR config', () => {
  beforeEach(() => {
    invokeMock.mockReset()
  })

  it('loadPetConfigFromCore maps siliconflow ASR config', async () => {
    invokeMock.mockResolvedValueOnce(JSON.stringify({
      pet: {
        asr_enabled: true,
        asr_provider: 'siliconflow',
        asr_language: 'zh-CN',
        asr_api_key: 'asr-key',
        asr_base_url: 'https://api.siliconflow.cn/v1',
        asr_model: 'FunAudioLLM/SenseVoiceSmall',
      },
    }))

    const config = await loadPetConfigFromCore()

    expect(config.asrEnabled).toBe(true)
    expect(config.asrProvider).toBe('siliconflow')
    expect(config.asrApiKey).toBe('asr-key')
    expect(config.asrBaseUrl).toBe('https://api.siliconflow.cn/v1')
    expect(config.asrModel).toBe('FunAudioLLM/SenseVoiceSmall')
  })

  it('savePetConfigToCore persists ASR fields', async () => {
    invokeMock.mockResolvedValueOnce(JSON.stringify({ pet: {} }))
    invokeMock.mockResolvedValueOnce(undefined)

    await savePetConfigToCore({
      ...DEFAULT_PET_CONFIG,
      asrProvider: 'siliconflow',
      asrApiKey: 'saved-key',
      asrBaseUrl: 'https://api.siliconflow.cn/v1',
      asrModel: 'FunAudioLLM/SenseVoiceSmall',
    })

    expect(invokeMock).toHaveBeenNthCalledWith(1, 'load_config')
    expect(invokeMock).toHaveBeenNthCalledWith(
      2,
      'save_config',
      expect.objectContaining({
        raw: expect.stringContaining('"asr_provider": "siliconflow"'),
      }),
    )
    expect(invokeMock.mock.calls[1]?.[1]).toEqual(expect.objectContaining({
      raw: expect.stringContaining('"asr_api_key": "saved-key"'),
    }))
  })

  it('loadPetConfigFromCore maps provider-specific TTS keys and ignores legacy shared key', async () => {
    invokeMock.mockResolvedValueOnce(JSON.stringify({
      pet: {
        tts_provider: 'minimax',
        tts_api_key: 'legacy-shared-key',
        tts_minimax_api_key: 'minimax-key',
        tts_base_url: 'https://api.minimaxi.com',
        tts_model: 'speech-2.8-hd',
      },
    }))

    const config = await loadPetConfigFromCore()

    expect(config.ttsProvider).toBe('minimax')
    expect(config.ttsApiKey).toBeNull()
    expect(config.ttsMinimaxApiKey).toBe('minimax-key')
    expect(config.ttsBaseUrl).toBe('https://api.minimaxi.com')
    expect(config.ttsModel).toBe('speech-2.8-hd')
  })

  it('savePetConfigToCore persists provider-specific TTS keys and clears legacy shared key', async () => {
    invokeMock.mockResolvedValueOnce(JSON.stringify({ pet: {} }))
    invokeMock.mockResolvedValueOnce(undefined)

    await savePetConfigToCore({
      ...DEFAULT_PET_CONFIG,
      ttsProvider: 'minimax',
      ttsApiKey: 'legacy-shared-key',
      ttsMinimaxApiKey: 'saved-minimax-key',
      ttsBaseUrl: 'https://api.minimaxi.com',
      ttsModel: 'speech-2.8-hd',
    })

    expect(invokeMock).toHaveBeenNthCalledWith(1, 'load_config')
    expect(invokeMock).toHaveBeenNthCalledWith(
      2,
      'save_config',
      expect.objectContaining({
        raw: expect.stringContaining('"tts_minimax_api_key": "saved-minimax-key"'),
      }),
    )
    expect(invokeMock.mock.calls[1]?.[1]).toEqual(expect.objectContaining({
      raw: expect.stringContaining('"tts_api_key": null'),
    }))
  })

  it('savePetConfigToCore does not persist frontend-only scene selection', async () => {
    invokeMock.mockResolvedValueOnce(JSON.stringify({ pet: {} }))
    invokeMock.mockResolvedValueOnce(undefined)

    await savePetConfigToCore({
      ...DEFAULT_PET_CONFIG,
      selectedGaussSceneId: 'sea',
    })

    const payload = invokeMock.mock.calls[1]?.[1] as { raw?: string } | undefined
    expect(payload?.raw).not.toContain('selectedGaussSceneId')
    expect(payload?.raw).not.toContain('selected_gauss_scene_id')
    expect(payload?.raw).not.toContain('gaussSceneList')
    expect(payload?.raw).not.toContain('gauss_scene_list')
  })

  it('resolveAsrTranscriptionConfig falls back to siliconflow defaults', () => {
    const resolved = resolveAsrTranscriptionConfig({
      apiKey: 'key',
      base64Data: 'Zm9v',
      fileName: 'sample.wav',
      provider: 'siliconflow',
    })

    expect(resolved.endpoint).toBe(`${DEFAULT_SILICONFLOW_ASR_BASE_URL}/audio/transcriptions`)
    expect(resolved.model).toBe(DEFAULT_SILICONFLOW_ASR_MODEL)
  })

  it('getAsrProviderDefaults returns empty defaults for web speech', () => {
    expect(getAsrProviderDefaults('web_speech')).toEqual({
      baseUrl: '',
      model: null,
    })
  })
})

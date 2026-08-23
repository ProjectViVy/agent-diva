import { invoke } from '@tauri-apps/api/core'
import type { MateConfig } from '../../types'
import { DEFAULT_MATE_CONFIG } from '../../types'
import type { VoiceFileReader } from './tts-service'
import { addVoiceLogEvent } from './voice-log'

export interface VoiceOption {
  id: string
  label: string
  relativePath: string
  source: string
}

export interface ResolvedVoiceConfig {
  enabled: boolean
  provider: string
  apiKey: string | null
  baseUrl: string
  model: string | null
  voiceId: string | null
  referenceVoice: string | null
  referenceText: string | null
  speed: number
  volume: number
}

export interface LoadedVoiceAssets {
  activeVoice: ResolvedVoiceConfig
  configDirectoryPath: string
  voiceOptions: VoiceOption[]
  voiceDirectoryPath: string
}

export interface SaveVoiceSelectionPayload {
  enabled: boolean
  provider: string
  apiKey: string | null
  baseUrl: string | null
  model: string | null
  voiceId: string | null
  referenceVoice: string | null
  referenceText: string | null
  speed: number
  volume: number
}

export interface VoiceFileData {
  base64Data: string
  contentType: string
  fileName: string
}

export interface TranscribeAudioPayload {
  base64Data: string
  fileName: string
  apiKey: string
  provider?: MateConfig['asrProvider']
  baseUrl?: string | null
  model?: string | null
  language?: string | null
  contentType?: string | null
}

interface TranscribeAudioResponse {
  text?: string
  text_list?: string[]
}

interface CoreConfigPayload {
  mate?: {
    enabled?: boolean
    vrm_model?: string
    tts_enabled?: boolean
    asr_enabled?: boolean
    asr_provider?: string
    asr_language?: string
    asr_api_key?: string | null
    asr_base_url?: string
    asr_model?: string | null
    tts_provider?: string
    tts_api_key?: string | null
    tts_openai_api_key?: string | null
    tts_siliconflow_api_key?: string | null
    tts_minimax_api_key?: string | null
    tts_base_url?: string
    tts_model?: string | null
    tts_voice_id?: string | null
    tts_reference_voice?: string | null
    tts_reference_text?: string | null
    tts_speed?: number
    tts_volume?: number
  }
  [key: string]: unknown
}

export const DEFAULT_SILICONFLOW_ASR_BASE_URL = 'https://api.siliconflow.cn/v1'
export const DEFAULT_SILICONFLOW_ASR_MODEL = 'FunAudioLLM/SenseVoiceSmall'

function normalizeTtsProvider(value: unknown): MateConfig['ttsProvider'] {
  if (value === 'openai' || value === 'siliconflow' || value === 'minimax') return value
  return 'browser'
}

function normalizeAsrProvider(value: unknown): MateConfig['asrProvider'] {
  if (value === 'web_speech' || value === 'siliconflow') return value
  return 'web_speech'
}

function normalizeNullableString(value: unknown): string | null {
  return typeof value === 'string' && value.trim() ? value : null
}

function mateFromCore(raw: CoreConfigPayload): MateConfig {
  const mate = raw.mate ?? {}
  return {
    ...DEFAULT_MATE_CONFIG,
    enabled: mate.enabled ?? DEFAULT_MATE_CONFIG.enabled,
    vrmModel: typeof mate.vrm_model === 'string' ? mate.vrm_model : DEFAULT_MATE_CONFIG.vrmModel,
    ttsEnabled: mate.tts_enabled ?? DEFAULT_MATE_CONFIG.ttsEnabled,
    asrEnabled: mate.asr_enabled ?? DEFAULT_MATE_CONFIG.asrEnabled,
    asrProvider: normalizeAsrProvider(mate.asr_provider),
    asrLanguage: typeof mate.asr_language === 'string' && mate.asr_language.trim()
      ? mate.asr_language
      : DEFAULT_MATE_CONFIG.asrLanguage,
    asrApiKey: normalizeNullableString(mate.asr_api_key),
    asrBaseUrl: typeof mate.asr_base_url === 'string' ? mate.asr_base_url : DEFAULT_MATE_CONFIG.asrBaseUrl,
    asrModel: normalizeNullableString(mate.asr_model),
    ttsProvider: normalizeTtsProvider(mate.tts_provider),
    ttsApiKey: null,
    ttsOpenaiApiKey: normalizeNullableString(mate.tts_openai_api_key),
    ttsSiliconflowApiKey: normalizeNullableString(mate.tts_siliconflow_api_key),
    ttsMinimaxApiKey: normalizeNullableString(mate.tts_minimax_api_key),
    ttsBaseUrl: typeof mate.tts_base_url === 'string' ? mate.tts_base_url : DEFAULT_MATE_CONFIG.ttsBaseUrl,
    ttsModel: normalizeNullableString(mate.tts_model),
    ttsVoiceId: normalizeNullableString(mate.tts_voice_id),
    ttsReferenceVoice: normalizeNullableString(mate.tts_reference_voice),
    ttsReferenceText: normalizeNullableString(mate.tts_reference_text),
    ttsSpeed: typeof mate.tts_speed === 'number' && Number.isFinite(mate.tts_speed) && mate.tts_speed > 0
      ? mate.tts_speed
      : DEFAULT_MATE_CONFIG.ttsSpeed,
    ttsVolume: typeof mate.tts_volume === 'number' && Number.isFinite(mate.tts_volume)
      ? mate.tts_volume
      : DEFAULT_MATE_CONFIG.ttsVolume,
  }
}

function applyMateToCore(raw: CoreConfigPayload, mate: MateConfig): CoreConfigPayload {
  return {
    ...raw,
    mate: {
      ...(raw.mate ?? {}),
      enabled: mate.enabled,
      vrm_model: mate.vrmModel,
      tts_enabled: mate.ttsEnabled,
      asr_enabled: mate.asrEnabled,
      asr_provider: mate.asrProvider,
      asr_language: mate.asrLanguage,
      asr_api_key: mate.asrApiKey,
      asr_base_url: mate.asrBaseUrl,
      asr_model: mate.asrModel,
      tts_provider: mate.ttsProvider,
      tts_api_key: null,
      tts_openai_api_key: mate.ttsOpenaiApiKey,
      tts_siliconflow_api_key: mate.ttsSiliconflowApiKey,
      tts_minimax_api_key: mate.ttsMinimaxApiKey,
      tts_base_url: mate.ttsBaseUrl,
      tts_model: mate.ttsModel,
      tts_voice_id: mate.ttsVoiceId,
      tts_reference_voice: mate.ttsReferenceVoice,
      tts_reference_text: mate.ttsReferenceText,
      tts_speed: mate.ttsSpeed,
      tts_volume: mate.ttsVolume,
    },
  }
}

export async function loadMateConfigFromCore(): Promise<MateConfig> {
  const raw = await invoke<string>('load_config')
  return mateFromCore(JSON.parse(raw) as CoreConfigPayload)
}

export async function saveMateConfigToCore(mate: MateConfig): Promise<void> {
  const raw = await invoke<string>('load_config')
  const nextConfig = applyMateToCore(JSON.parse(raw) as CoreConfigPayload, mate)
  await invoke('save_config', { raw: JSON.stringify(nextConfig, null, 2) })
}

export function loadVoiceAssets(): Promise<LoadedVoiceAssets> {
  return invoke<LoadedVoiceAssets>('mate_load_voice_assets')
}

export function saveVoiceSelection(payload: SaveVoiceSelectionPayload): Promise<LoadedVoiceAssets> {
  return invoke<LoadedVoiceAssets>('mate_save_voice_selection', { payload })
}

export async function importVoiceFile(file: File): Promise<LoadedVoiceAssets> {
  const base64Data = await fileToBase64(file)
  return invoke<LoadedVoiceAssets>('mate_import_voice_file', {
    payload: { base64Data, fileName: file.name },
  })
}

export function deleteVoiceFile(relativePath: string): Promise<LoadedVoiceAssets> {
  return invoke<LoadedVoiceAssets>('mate_delete_voice_file', {
    payload: { relativePath },
  })
}

export function readVoiceFile(relativePath: string): Promise<VoiceFileData> {
  return invoke<VoiceFileData>('mate_read_voice_file', { relativePath })
}

export function getAsrProviderDefaults(provider: MateConfig['asrProvider']) {
  if (provider === 'siliconflow') {
    return {
      baseUrl: DEFAULT_SILICONFLOW_ASR_BASE_URL,
      model: DEFAULT_SILICONFLOW_ASR_MODEL,
    }
  }
  return {
    baseUrl: '',
    model: null,
  }
}

export function resolveAsrTranscriptionConfig(payload: TranscribeAudioPayload) {
  const provider = payload.provider ?? 'siliconflow'
  const defaults = getAsrProviderDefaults(provider)
  const baseUrl = (payload.baseUrl?.trim() || defaults.baseUrl).replace(/\/+$/, '')
  const model = payload.model?.trim() || defaults.model || DEFAULT_SILICONFLOW_ASR_MODEL
  return {
    provider,
    endpoint: `${baseUrl}/audio/transcriptions`,
    model,
    language: payload.language?.trim() || undefined,
  }
}

export async function transcribeAudio(payload: TranscribeAudioPayload): Promise<string> {
  try {
    const resolved = resolveAsrTranscriptionConfig(payload)
    const byteCharacters = atob(payload.base64Data)
    const byteNumbers = new Array(byteCharacters.length)
    for (let index = 0; index < byteCharacters.length; index += 1) {
      byteNumbers[index] = byteCharacters.charCodeAt(index)
    }

    const formData = new FormData()
    formData.append(
      'file',
      new Blob([new Uint8Array(byteNumbers)], { type: payload.contentType || 'application/octet-stream' }),
      payload.fileName,
    )
    formData.append('model', resolved.model)
    if (resolved.language) {
      formData.append('language', resolved.language)
    }

    const response = await fetch(resolved.endpoint, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${payload.apiKey}`,
      },
      body: formData,
    })

    if (!response.ok) {
      const errorText = await response.text()
      addVoiceLogEvent({
        level: 'warn',
        source: 'asr',
        message: '云端 ASR 转写失败',
        detail: {
          provider: resolved.provider,
          model: resolved.model,
          status: response.status,
          error: errorText.slice(0, 160),
        },
      })
      return ''
    }

    const result = await response.json() as TranscribeAudioResponse
    const text = (result.text || result.text_list?.[0] || '').trim()
    addVoiceLogEvent({
      level: 'info',
      source: 'asr',
      message: text ? '云端 ASR 转写完成' : '云端 ASR 转写结果为空',
      detail: {
        provider: resolved.provider,
        model: resolved.model,
        textPreview: text.slice(0, 80) || null,
      },
    })
    return text
  } catch (error) {
    addVoiceLogEvent({
      level: 'warn',
      source: 'asr',
      message: '云端 ASR 转写请求异常',
      detail: { provider: payload.provider ?? 'siliconflow', error: String(error) },
    })
    return ''
  }
}

export const tauriVoiceFileReader: VoiceFileReader = {
  async readVoiceFile(relativePath: string) {
    try {
      return await readVoiceFile(relativePath)
    } catch (error) {
      addVoiceLogEvent({
        level: 'error',
        source: 'tts',
        message: '读取参考音色失败',
        detail: { path: relativePath, error: String(error) },
      })
      return null
    }
  },
}

function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onerror = () => reject(new Error('Failed to read voice file.'))
    reader.onload = () => {
      const result = String(reader.result ?? '')
      resolve(result.includes(',') ? result.split(',')[1] : result)
    }
    reader.readAsDataURL(file)
  })
}

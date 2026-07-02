import { invoke } from '@tauri-apps/api/core'
import type { PetConfig } from '../../types'
import { DEFAULT_PET_CONFIG } from '../../types'
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
  provider?: PetConfig['asrProvider']
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
  pet?: {
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

function normalizeTtsProvider(value: unknown): PetConfig['ttsProvider'] {
  if (value === 'openai' || value === 'siliconflow' || value === 'minimax') return value
  return 'browser'
}

function normalizeAsrProvider(value: unknown): PetConfig['asrProvider'] {
  if (value === 'web_speech' || value === 'siliconflow') return value
  return 'web_speech'
}

function normalizeNullableString(value: unknown): string | null {
  return typeof value === 'string' && value.trim() ? value : null
}

function petFromCore(raw: CoreConfigPayload): PetConfig {
  const pet = raw.pet ?? {}
  return {
    ...DEFAULT_PET_CONFIG,
    enabled: pet.enabled ?? DEFAULT_PET_CONFIG.enabled,
    vrmModel: typeof pet.vrm_model === 'string' ? pet.vrm_model : DEFAULT_PET_CONFIG.vrmModel,
    ttsEnabled: pet.tts_enabled ?? DEFAULT_PET_CONFIG.ttsEnabled,
    asrEnabled: pet.asr_enabled ?? DEFAULT_PET_CONFIG.asrEnabled,
    asrProvider: normalizeAsrProvider(pet.asr_provider),
    asrLanguage: typeof pet.asr_language === 'string' && pet.asr_language.trim()
      ? pet.asr_language
      : DEFAULT_PET_CONFIG.asrLanguage,
    asrApiKey: normalizeNullableString(pet.asr_api_key),
    asrBaseUrl: typeof pet.asr_base_url === 'string' ? pet.asr_base_url : DEFAULT_PET_CONFIG.asrBaseUrl,
    asrModel: normalizeNullableString(pet.asr_model),
    ttsProvider: normalizeTtsProvider(pet.tts_provider),
    ttsApiKey: null,
    ttsOpenaiApiKey: normalizeNullableString(pet.tts_openai_api_key),
    ttsSiliconflowApiKey: normalizeNullableString(pet.tts_siliconflow_api_key),
    ttsMinimaxApiKey: normalizeNullableString(pet.tts_minimax_api_key),
    ttsBaseUrl: typeof pet.tts_base_url === 'string' ? pet.tts_base_url : DEFAULT_PET_CONFIG.ttsBaseUrl,
    ttsModel: normalizeNullableString(pet.tts_model),
    ttsVoiceId: normalizeNullableString(pet.tts_voice_id),
    ttsReferenceVoice: normalizeNullableString(pet.tts_reference_voice),
    ttsReferenceText: normalizeNullableString(pet.tts_reference_text),
    ttsSpeed: typeof pet.tts_speed === 'number' && Number.isFinite(pet.tts_speed) && pet.tts_speed > 0
      ? pet.tts_speed
      : DEFAULT_PET_CONFIG.ttsSpeed,
    ttsVolume: typeof pet.tts_volume === 'number' && Number.isFinite(pet.tts_volume)
      ? pet.tts_volume
      : DEFAULT_PET_CONFIG.ttsVolume,
  }
}

function applyPetToCore(raw: CoreConfigPayload, pet: PetConfig): CoreConfigPayload {
  return {
    ...raw,
    pet: {
      ...(raw.pet ?? {}),
      enabled: pet.enabled,
      vrm_model: pet.vrmModel,
      tts_enabled: pet.ttsEnabled,
      asr_enabled: pet.asrEnabled,
      asr_provider: pet.asrProvider,
      asr_language: pet.asrLanguage,
      asr_api_key: pet.asrApiKey,
      asr_base_url: pet.asrBaseUrl,
      asr_model: pet.asrModel,
      tts_provider: pet.ttsProvider,
      tts_api_key: null,
      tts_openai_api_key: pet.ttsOpenaiApiKey,
      tts_siliconflow_api_key: pet.ttsSiliconflowApiKey,
      tts_minimax_api_key: pet.ttsMinimaxApiKey,
      tts_base_url: pet.ttsBaseUrl,
      tts_model: pet.ttsModel,
      tts_voice_id: pet.ttsVoiceId,
      tts_reference_voice: pet.ttsReferenceVoice,
      tts_reference_text: pet.ttsReferenceText,
      tts_speed: pet.ttsSpeed,
      tts_volume: pet.ttsVolume,
    },
  }
}

export async function loadPetConfigFromCore(): Promise<PetConfig> {
  const raw = await invoke<string>('load_config')
  return petFromCore(JSON.parse(raw) as CoreConfigPayload)
}

export async function savePetConfigToCore(pet: PetConfig): Promise<void> {
  const raw = await invoke<string>('load_config')
  const nextConfig = applyPetToCore(JSON.parse(raw) as CoreConfigPayload, pet)
  await invoke('save_config', { raw: JSON.stringify(nextConfig, null, 2) })
}

export function loadVoiceAssets(): Promise<LoadedVoiceAssets> {
  return invoke<LoadedVoiceAssets>('pet_load_voice_assets')
}

export function saveVoiceSelection(payload: SaveVoiceSelectionPayload): Promise<LoadedVoiceAssets> {
  return invoke<LoadedVoiceAssets>('pet_save_voice_selection', { payload })
}

export async function importVoiceFile(file: File): Promise<LoadedVoiceAssets> {
  const base64Data = await fileToBase64(file)
  return invoke<LoadedVoiceAssets>('pet_import_voice_file', {
    payload: { base64Data, fileName: file.name },
  })
}

export function deleteVoiceFile(relativePath: string): Promise<LoadedVoiceAssets> {
  return invoke<LoadedVoiceAssets>('pet_delete_voice_file', {
    payload: { relativePath },
  })
}

export function readVoiceFile(relativePath: string): Promise<VoiceFileData> {
  return invoke<VoiceFileData>('pet_read_voice_file', { relativePath })
}

export function getAsrProviderDefaults(provider: PetConfig['asrProvider']) {
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

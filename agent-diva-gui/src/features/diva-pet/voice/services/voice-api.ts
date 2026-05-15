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
  baseUrl?: string | null
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
    tts_provider?: string
    tts_api_key?: string | null
    tts_base_url?: string
    tts_model?: string | null
    tts_reference_voice?: string | null
    tts_reference_text?: string | null
    tts_speed?: number
    tts_volume?: number
  }
  [key: string]: unknown
}

function normalizeTtsProvider(value: unknown): PetConfig['ttsProvider'] {
  if (value === 'openai' || value === 'siliconflow') return value
  return 'browser'
}

function normalizeAsrProvider(value: unknown): PetConfig['asrProvider'] {
  if (value === 'web_speech') return value
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
    ttsProvider: normalizeTtsProvider(pet.tts_provider),
    ttsApiKey: normalizeNullableString(pet.tts_api_key),
    ttsBaseUrl: typeof pet.tts_base_url === 'string' ? pet.tts_base_url : DEFAULT_PET_CONFIG.ttsBaseUrl,
    ttsModel: normalizeNullableString(pet.tts_model),
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
      tts_provider: pet.ttsProvider,
      tts_api_key: pet.ttsApiKey,
      tts_base_url: pet.ttsBaseUrl,
      tts_model: pet.ttsModel,
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

export async function transcribeAudio(payload: TranscribeAudioPayload): Promise<string> {
  try {
    const endpoint = `${payload.baseUrl || 'https://api.siliconflow.cn/v1'}/audio/transcriptions`
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
    formData.append('model', 'FunAudioLLM/SenseVoiceSmall')

    const response = await fetch(endpoint, {
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
        message: '参考音色转写失败',
        detail: { status: response.status, error: errorText.slice(0, 160) },
      })
      return ''
    }

    const result = await response.json() as TranscribeAudioResponse
    const text = (result.text || result.text_list?.[0] || '').trim()
    addVoiceLogEvent({
      level: 'info',
      source: 'asr',
      message: text ? '参考音色转写完成' : '参考音色转写结果为空',
      detail: { textPreview: text.slice(0, 80) || null },
    })
    return text
  } catch (error) {
    addVoiceLogEvent({
      level: 'warn',
      source: 'asr',
      message: '参考音色转写请求异常',
      detail: { error: String(error) },
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

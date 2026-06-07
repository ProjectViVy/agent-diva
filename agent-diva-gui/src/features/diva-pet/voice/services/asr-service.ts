import type { PetConfig } from '../../types'
import {
  getAsrProviderDefaults,
  transcribeAudio,
} from './voice-api'

export interface ASRProviderConfig {
  provider: PetConfig['asrProvider']
  language: string
  apiKey: string | null
  baseUrl: string
  model: string | null
}

export interface CloudAsrRequest {
  audioBlob: Blob
  fileName?: string
  config: ASRProviderConfig
}

function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onerror = () => reject(new Error('Failed to read ASR audio blob.'))
    reader.onload = () => {
      const result = String(reader.result ?? '')
      resolve(result.includes(',') ? result.split(',')[1] : result)
    }
    reader.readAsDataURL(blob)
  })
}

export function resolveAsrConfigDefaults(
  provider: PetConfig['asrProvider'],
  config: Pick<ASRProviderConfig, 'baseUrl' | 'model'>,
) {
  const defaults = getAsrProviderDefaults(provider)
  return {
    baseUrl: config.baseUrl.trim() || defaults.baseUrl,
    model: config.model?.trim() || defaults.model,
  }
}

export function isCloudAsrProvider(provider: PetConfig['asrProvider']): boolean {
  return provider === 'siliconflow'
}

export async function transcribeWithAsrProvider(request: CloudAsrRequest): Promise<string> {
  const { config } = request
  if (!isCloudAsrProvider(config.provider)) {
    return ''
  }
  if (!config.apiKey?.trim()) {
    throw new Error('当前 ASR Provider 缺少 API Key。')
  }

  const resolved = resolveAsrConfigDefaults(config.provider, {
    baseUrl: config.baseUrl,
    model: config.model,
  })
  const base64Data = await blobToBase64(request.audioBlob)

  return transcribeAudio({
    apiKey: config.apiKey,
    base64Data,
    provider: config.provider,
    baseUrl: resolved.baseUrl,
    model: resolved.model,
    language: config.language,
    fileName: request.fileName || `diva-pet-asr-${Date.now()}.webm`,
    contentType: request.audioBlob.type || 'audio/webm',
  })
}

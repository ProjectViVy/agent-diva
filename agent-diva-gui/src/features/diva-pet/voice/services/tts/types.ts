import type { VoiceLogEvent } from '../voice-log'

export interface VoiceFileReader {
  readVoiceFile(relativePath: string): Promise<{
    base64Data: string
    contentType: string
    fileName: string
  } | null>
}

export interface TTSRequest {
  text: string
  speed?: number
}

export interface TTSVoiceConfig {
  enabled: boolean
  provider: string
  apiKey: string | null
  baseUrl: string
  model: string | null
  voiceId?: string | null
  referenceVoice: string | null
  referenceText: string | null
  speed: number
  volume?: number
}

export type TTSProvider = 'browser' | 'openai' | 'siliconflow' | 'minimax' | 'local'

export interface TTSResponse {
  audioUrl: string
  durationMs?: number
  isCloned?: boolean
}

export interface RemoteVoiceListEntry {
  customName?: string
  model?: string
  uri?: string
}

export interface RemoteVoiceListResponse {
  results?: RemoteVoiceListEntry[]
}

export interface RemoteVoiceUploadResponse {
  uri?: string
}

export interface MiniMaxSynthesizePayload {
  apiKey: string
  baseUrl?: string | null
  model?: string | null
  speed?: number
  text: string
  voiceId?: string | null
  volume?: number
}

export interface MiniMaxSynthesizeResponse {
  base64Data: string
  contentType: string
}

export interface SiliconFlowSynthesizePayload {
  apiKey: string
  baseUrl?: string | null
  model?: string | null
  voice?: string | null
  speed?: number
  gain?: number
  text: string
  references?: Array<{ audio: string; text: string }>
}

export interface SiliconFlowSynthesizeResponse {
  base64Data: string
  contentType: string
}

export const PROVIDER_DEFAULTS: Record<string, { baseUrl: string; model: string }> = {
  openai: {
    baseUrl: 'https://api.openai.com/v1',
    model: 'tts-1',
  },
  siliconflow: {
    baseUrl: 'https://api.siliconflow.cn/v1',
    model: 'fnlp/MOSS-TTSD-v0.5',
  },
  minimax: {
    baseUrl: 'https://api.minimaxi.com',
    model: 'speech-2.8-hd',
  },
}

export const PROVIDER_VOICE_DEFAULTS: Record<string, string> = {
  openai: 'alloy',
  siliconflow: 'fnlp/MOSS-TTSD-v0.5:anna',
  minimax: 'male-qn-qingse',
}

export const DEFAULT_SPEED = 1.0

export type FactoryTTSProvider = 'minimax' | 'openai' | 'siliconflow'

export type TTSLogEvent = Omit<VoiceLogEvent, 'id' | 'at'>

export interface TTSProviderFactoryContext {
  createAudioResponse: (
    base64Data: string,
    contentType: string | null | undefined,
    isCloned?: boolean,
  ) => TTSResponse
  invokeCommand: <TResponse>(command: string, payload: unknown) => Promise<TResponse>
  logEvent: (event: TTSLogEvent) => void
  resolveProviderConfig: (
    provider: FactoryTTSProvider,
    baseUrl: string,
    model: string | null,
  ) => { baseUrl: string; model: string }
}

export interface TTSProviderHandler {
  readonly provider: 'minimax' | 'siliconflow'
  synthesize(request: TTSRequest, voiceConfig: TTSVoiceConfig): Promise<TTSResponse | null>
}

export interface SiliconFlowTTSProviderHandler extends TTSProviderHandler {
  readonly provider: 'siliconflow'
  synthesizeInlineClone(
    request: TTSRequest,
    voiceConfig: TTSVoiceConfig,
    options: {
      model: string
      referenceAudio: string
      referenceText: string
    },
  ): Promise<TTSResponse | null>
  synthesizeReusableClone(
    request: TTSRequest,
    voiceConfig: TTSVoiceConfig,
    options: {
      cloneVoiceUri: string
      model: string
    },
  ): Promise<TTSResponse>
}

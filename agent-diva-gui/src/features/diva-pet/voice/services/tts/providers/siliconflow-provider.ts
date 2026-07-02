import {
  DEFAULT_SPEED,
  type SiliconFlowSynthesizePayload,
  type SiliconFlowSynthesizeResponse,
  type SiliconFlowTTSProviderHandler,
  PROVIDER_VOICE_DEFAULTS,
  type TTSProviderFactoryContext,
  type TTSRequest,
  type TTSResponse,
  type TTSVoiceConfig,
} from '../types'

export class SiliconFlowProviderHandler implements SiliconFlowTTSProviderHandler {
  readonly provider = 'siliconflow' as const

  constructor(private readonly context: TTSProviderFactoryContext) {}

  async synthesize(
    request: TTSRequest,
    voiceConfig: TTSVoiceConfig,
  ): Promise<TTSResponse | null> {
    const apiKey = voiceConfig.apiKey
    if (!apiKey) {
      console.warn('[TTSService] siliconflow voice API key is not configured.')
      this.context.logEvent({
        level: 'warn',
        source: 'tts',
        message: 'SiliconFlow TTS 未填写 API Key，请在当前 Provider 下配置',
        detail: { provider: this.provider },
      })
      return null
    }

    const { baseUrl, model } = this.context.resolveProviderConfig(
      this.provider,
      voiceConfig.baseUrl,
      voiceConfig.model,
    )
    const voice = voiceConfig.voiceId?.trim() || PROVIDER_VOICE_DEFAULTS[this.provider]
    const payload = await this.context.invokeCommand<SiliconFlowSynthesizeResponse>(
      'pet_siliconflow_synthesize',
      {
        apiKey,
        baseUrl,
        model,
        text: request.text,
        voice,
        speed: request.speed || voiceConfig.speed || DEFAULT_SPEED,
        gain: 0.0,
      } satisfies SiliconFlowSynthesizePayload,
    )

    return this.context.createAudioResponse(
      payload.base64Data,
      payload.contentType,
      false,
    )
  }

  async synthesizeInlineClone(
    request: TTSRequest,
    voiceConfig: TTSVoiceConfig,
    options: {
      model: string
      referenceAudio: string
      referenceText: string
    },
  ): Promise<TTSResponse | null> {
    const apiKey = voiceConfig.apiKey
    if (!apiKey) {
      return null
    }

    const { baseUrl } = this.context.resolveProviderConfig(
      this.provider,
      voiceConfig.baseUrl,
      voiceConfig.model,
    )
    const payload = await this.context.invokeCommand<SiliconFlowSynthesizeResponse>(
      'pet_siliconflow_synthesize',
      {
        apiKey,
        baseUrl,
        model: options.model,
        text: request.text,
        voice: '',
        speed: request.speed || voiceConfig.speed || DEFAULT_SPEED,
        gain: 0.0,
        references: [
          {
            audio: options.referenceAudio,
            text: options.referenceText,
          },
        ],
      } satisfies SiliconFlowSynthesizePayload,
    )

    return this.context.createAudioResponse(
      payload.base64Data,
      payload.contentType,
      true,
    )
  }

  async synthesizeReusableClone(
    request: TTSRequest,
    voiceConfig: TTSVoiceConfig,
    options: {
      cloneVoiceUri: string
      model: string
    },
  ): Promise<TTSResponse> {
    const apiKey = voiceConfig.apiKey!
    const { baseUrl } = this.context.resolveProviderConfig(
      this.provider,
      voiceConfig.baseUrl,
      voiceConfig.model,
    )
    const payload = await this.context.invokeCommand<SiliconFlowSynthesizeResponse>(
      'pet_siliconflow_synthesize',
      {
        apiKey,
        baseUrl,
        model: options.model,
        text: request.text,
        voice: options.cloneVoiceUri,
        speed: request.speed || voiceConfig.speed || DEFAULT_SPEED,
        gain: 0.0,
      } satisfies SiliconFlowSynthesizePayload,
    )

    return this.context.createAudioResponse(
      payload.base64Data,
      payload.contentType,
      true,
    )
  }
}

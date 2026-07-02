import {
  DEFAULT_SPEED,
  type MiniMaxSynthesizePayload,
  type MiniMaxSynthesizeResponse,
  PROVIDER_VOICE_DEFAULTS,
  type TTSProviderFactoryContext,
  type TTSProviderHandler,
  type TTSRequest,
  type TTSResponse,
  type TTSVoiceConfig,
} from '../types'

export class MiniMaxTTSProviderHandler implements TTSProviderHandler {
  readonly provider = 'minimax' as const

  constructor(private readonly context: TTSProviderFactoryContext) {}

  async synthesize(
    request: TTSRequest,
    voiceConfig: TTSVoiceConfig,
  ): Promise<TTSResponse | null> {
    const apiKey = voiceConfig.apiKey

    if (!apiKey) {
      console.warn('[TTSService] MiniMax voice API key is not configured.')
      this.context.logEvent({
        level: 'warn',
        source: 'tts',
        message: 'MiniMax TTS 未填写 API Key，请在当前 Provider 下配置',
        detail: { provider: this.provider },
      })
      return null
    }

    const { baseUrl, model } = this.context.resolveProviderConfig(
      this.provider,
      voiceConfig.baseUrl,
      voiceConfig.model,
    )
    const resolvedVoiceId =
      voiceConfig.voiceId?.trim() || PROVIDER_VOICE_DEFAULTS[this.provider]

    if (voiceConfig.referenceVoice) {
      this.context.logEvent({
        level: 'info',
        source: 'tts',
        message: 'MiniMax 忽略参考音色配置，使用系统音色',
        detail: { provider: this.provider, voiceId: resolvedVoiceId },
      })
    }

    const payload = await this.context.invokeCommand<MiniMaxSynthesizeResponse>(
      'pet_minimax_synthesize',
      {
        apiKey,
        baseUrl,
        model,
        speed: request.speed || voiceConfig.speed || DEFAULT_SPEED,
        text: request.text,
        voiceId: resolvedVoiceId,
        volume: voiceConfig.volume ?? 1.0,
      } satisfies MiniMaxSynthesizePayload,
    )

    this.context.logEvent({
      level: 'info',
      source: 'tts',
      message: 'MiniMax TTS 合成完成',
      detail: {
        provider: this.provider,
        model,
        voiceId: resolvedVoiceId,
      },
    })

    return this.context.createAudioResponse(
      payload.base64Data,
      payload.contentType,
      false,
    )
  }
}

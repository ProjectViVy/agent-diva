import { MiniMaxTTSProviderHandler } from './providers/minimax-provider'
import { SiliconFlowProviderHandler } from './providers/siliconflow-provider'
import type {
  SiliconFlowTTSProviderHandler,
  TTSProvider,
  TTSProviderFactoryContext,
  TTSProviderHandler,
} from './types'

export function createTTSProviderHandler(
  provider: TTSProvider,
  context: TTSProviderFactoryContext,
): TTSProviderHandler | null {
  switch (provider) {
    case 'minimax':
      return new MiniMaxTTSProviderHandler(context)
    case 'siliconflow':
      return new SiliconFlowProviderHandler(context)
    default:
      return null
  }
}

export function asSiliconFlowProviderHandler(
  handler: TTSProviderHandler | null,
): SiliconFlowTTSProviderHandler {
  if (!handler || handler.provider !== 'siliconflow') {
    throw new Error('SiliconFlow TTS provider factory is unavailable.')
  }

  return handler as SiliconFlowTTSProviderHandler
}

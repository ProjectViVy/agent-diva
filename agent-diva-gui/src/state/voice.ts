/**
 * DN-6C application voice seam: lazily builds the voice controller against
 * the live native facade + media adapters and binds it to the chat owner's
 * invalidation + fresh-reply hooks. Browser mode keeps an inert controller
 * (native_unavailable surfaces through init errors, never crashes).
 */
import { createVoiceController, type VoiceController } from '../features/voice/useVoiceController'
import { createMediaRecorder } from '../features/voice/voiceRecorder'
import { createAudioPlayer } from '../features/voice/voicePlayer'
import { speechDiagnostics, speechNative, NativeUnavailableError } from '../api/speech'
import { isTauriShell } from '../platform/desktop-host'
import { vivyChat } from './chat-instance'
import { recordGuiDiagnostic } from './gui-diagnostics'

let controller: VoiceController | null = null
let initPromise: Promise<void> | null = null

export function voiceController(): VoiceController {
  if (!controller) {
    controller = createVoiceController({
      native: speechNative,
      recorder: createMediaRecorder(),
      player: createAudioPlayer(),
      diagnostics: {
        record: (input) =>
          recordGuiDiagnostic({
            level: (input.level as 'error' | 'warn' | 'info') ?? 'warn',
            component: 'voice',
            message: input.message,
            fields: input.fields,
          }),
      },
      sessionId: () => vivyChat.currentSessionId,
      newId: () => crypto.randomUUID(),
    })
    // Conversation mutations drop session-bound voice work before history
    // changes — the controller releases local media immediately and lets the
    // native generation fence absorb any late bytes.
    vivyChat.onConversationInvalidate((reason: string) => controller?.invalidate(reason))
    vivyChat.onFreshReply(({ runId, text }: { runId: string; text: string }) =>
      void controller?.notifyFreshReply({ runId, text }),
    )
    if (isTauriShell()) {
      // Native speech diagnostics land in the OBS-08 recorder alongside the
      // controller's own records — one diagnostics stream for the window.
      void speechDiagnostics
        .subscribe((diagnostic) =>
          recordGuiDiagnostic({
            level: diagnostic.kind === 'error' ? 'error' : 'info',
            component: 'speech/native',
            message: `${diagnostic.phase}:${diagnostic.kind}`,
            fields: diagnostic as unknown as Record<string, unknown>,
          }),
        )
        .catch(() => undefined)
    }
  }
  return controller
}

/** Initialize once; the same promise serves every caller. */
export function initVoice(): Promise<void> {
  if (!initPromise) {
    initPromise = voiceController()
      .init()
      .catch((err) => {
        if (err instanceof NativeUnavailableError) return // browser mode: inert
        recordGuiDiagnostic({
          level: 'warn',
          component: 'voice',
          message: 'voice init failed',
          fields: { error: err instanceof Error ? err.message : String(err) },
        })
      })
  }
  return initPromise
}

/**
 * DN-6C typed native facade over the C2-4 speech seam. The controller and
 * UI see one capability contract; browser mode surfaces the honest
 * `native_unavailable` error instead of a fake path.
 */
import {
  isTauriShell,
  speechCancel,
  speechConfigGet,
  speechConfigUpdate,
  speechContextSet,
  speechCredentialDelete,
  speechCredentialSet,
  speechSynthesize,
  speechTranscribe,
  voiceAssetDelete,
  voiceAssetImport,
  voiceAssetList,
  voiceAssetRead,
  onSpeechDiagnostic,
  type SpeechConfigReadback,
  type SpeechContext,
  type SpeechDiagnostic,
  type SpeechIdentity,
  type TranscribeReply,
  type VoiceAssetDescriptor,
} from '../platform/desktop-host'
import type { VoiceNative } from '../features/voice/useVoiceController'

export class NativeUnavailableError extends Error {
  readonly code = 'native_unavailable'
  constructor() {
    super('native speech is unavailable in this context')
  }
}

function requireNative<T>(fn: () => Promise<T>): Promise<T> {
  if (!isTauriShell()) return Promise.reject(new NativeUnavailableError())
  return fn()
}

export const speechNative: VoiceNative = {
  configGet: () => requireNative(() => speechConfigGet()),
  configUpdate: (input) => requireNative(() => speechConfigUpdate(input)),
  contextSet: (sessionId, generation) => requireNative(() => speechContextSet(sessionId, generation)),
  transcribe: (identity, wav) =>
    requireNative(() => speechTranscribe(wav, identity)).then((reply) => ({
      status: reply.status,
      text: reply.text,
    })),
  synthesize: (identity, text) => requireNative(() => speechSynthesize(identity, text)),
  cancel: (requestId) => requireNative(() => speechCancel(requestId)),
}

export type {
  SpeechConfigReadback,
  SpeechContext,
  SpeechDiagnostic,
  SpeechIdentity,
  TranscribeReply,
  VoiceAssetDescriptor,
}

/** Full-readback config surface for settings (VoiceNative stays the
 * controller's narrower contract). */
export const speechConfig = {
  get: (): Promise<SpeechConfigReadback> => requireNative(() => speechConfigGet()),
  update: (input: { base_revision: number; preferences: unknown }): Promise<SpeechConfigReadback> =>
    requireNative(() => speechConfigUpdate(input)),
}

export const speechCredentials = {
  set: (provider: string, key: string) => requireNative(() => speechCredentialSet(provider, key)),
  delete: (provider: string) => requireNative(() => speechCredentialDelete(provider)),
}

export const voiceAssets = {
  import: (bytes: ArrayBuffer, displayName: string, mimeType: string) =>
    requireNative(() => voiceAssetImport(bytes, displayName, mimeType)),
  list: () => requireNative(() => voiceAssetList()),
  read: (assetId: string) => requireNative(() => voiceAssetRead(assetId)),
  delete: (assetId: string) => requireNative(() => voiceAssetDelete(assetId)),
}

export const speechDiagnostics = {
  subscribe: (handler: (diagnostic: SpeechDiagnostic) => void) =>
    requireNative(() => onSpeechDiagnostic(handler)),
}

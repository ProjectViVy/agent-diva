/**
 * DN-6C voice controller (framework core). One owner for mic tracks, Blob
 * playback, the native window generation and fresh-answer eligibility.
 *
 * Fences that define the contract:
 * - STT output lands in `draft` only — nothing ever auto-sends.
 * - Every native request carries a full SpeechIdentity minted from the
 *   CURRENT generation; a reply whose captured identity is stale (context
 *   advanced, newer utterance, invalidation) is dropped before playback —
 *   late bytes never create a Blob URL.
 * - Init reads the native window generation via config_get and increments
 *   through context_set; a reloaded controller never restarts at zero.
 * - Auto-read is opt-in and fires only for a fresh primary reply — never
 *   history, child, cancelled, failed, or replayed output.
 * - Every exit path (cancel, invalidate, dispose) stops local media first,
 *   cancels in-flight native requests, then releases counters to zero.
 */
import { computed, ref, type Ref } from 'vue'

export interface SpeechIdentity {
  request_id: string
  session_id: string
  run_id?: string
  utterance_id: string
  generation: number
}

export interface VoiceNative {
  configGet(): Promise<{
    revision: number
    preferences: unknown
    credential_state: { siliconflow: string; minimax: string }
    window_context: { session_id: string; generation: number } | null
  }>
  configUpdate(input: { base_revision: number; preferences: unknown }): Promise<{ revision: number; preferences?: unknown }>
  contextSet(sessionId: string, generation: number): Promise<{ session_id: string; generation: number }>
  transcribe(identity: SpeechIdentity, wav: ArrayBuffer): Promise<{ status: 'transcribed' | 'no_speech'; text: string }>
  synthesize(identity: SpeechIdentity, text: string): Promise<ArrayBuffer>
  cancel(requestId: string): Promise<{ request_id: string; status: 'cancelled' | 'settled' }>
}

export interface VoicePlayer {
  play(bytes: ArrayBuffer, utteranceId: string): Promise<void>
  stop(): void
  dispose(): void
}

export interface VoiceRecorder {
  start(): void
  stop(): Promise<ArrayBuffer>
  cancel(): void
  dispose(): void
}

export interface VoiceDiagnostics {
  record(input: { level?: string; component?: string; message: string; fields?: Record<string, unknown> }): void
}

export interface VoiceControllerDeps {
  native: VoiceNative
  recorder: VoiceRecorder
  player: VoicePlayer
  diagnostics?: VoiceDiagnostics
  sessionId(): string | null
  newId(): string
}

export interface FreshReplyMeta {
  runId?: string
  text: string
  fromHistory?: boolean
  child?: boolean
  cancelled?: boolean
  failed?: boolean
}

export type VoiceState = 'idle' | 'recording' | 'transcribing' | 'speaking'

export interface VoiceController {
  state: Ref<VoiceState>
  draft: Ref<string>
  error: Ref<string | null>
  generation: Ref<number>
  playing: Ref<boolean>
  openResources: Ref<number>
  autoRead: Ref<boolean>
  init(): Promise<void>
  startRecording(): Promise<void>
  stopRecording(): Promise<void>
  cancelRecording(): void
  requestReply(runId: string | undefined, text: string): Promise<void>
  notifyFreshReply(meta: FreshReplyMeta): Promise<void>
  stopSpeaking(): void
  advanceContext(sessionId: string): Promise<void>
  invalidate(reason: string): void
  dispose(): void
}

export function createVoiceController(deps: VoiceControllerDeps): VoiceController {
  const state = ref<VoiceState>('idle')
  const draft = ref('')
  const error = ref<string | null>(null)
  const generation = ref(0)
  const playing = ref(false)
  const autoRead = ref(false)

  const pending = new Set<string>()
  let activeUtterance: string | null = null
  let recorderOpen = false
  let playerOpen = false
  let disposed = false
  let ready = false
  let boundSession: string | null = null

  const openResources = computed(() => pending.size + (recorderOpen ? 1 : 0) + (playerOpen ? 1 : 0))

  const report = (message: string, fields?: Record<string, unknown>, level = 'warn') => {
    deps.diagnostics?.record({ level, component: 'voice', message, fields })
  }

  function mintIdentity(runId?: string): SpeechIdentity {
    return {
      request_id: deps.newId(),
      session_id: boundSession ?? deps.sessionId() ?? '',
      ...(runId ? { run_id: runId } : {}),
      utterance_id: deps.newId(),
      generation: generation.value,
    }
  }

  /** Drop a captured request's work when its identity is no longer live. */
  function isStale(identity: SpeechIdentity): boolean {
    return disposed || identity.generation !== generation.value
  }

  async function init(): Promise<void> {
    const config = await deps.native.configGet()
    autoRead.value = Boolean(
      (config.preferences as { auto_read_replies?: boolean } | null)?.auto_read_replies,
    )
    // Contract: a reloaded controller reads the native window generation
    // and advances — never restarts at zero, never reuses a stale epoch.
    generation.value = config.window_context?.generation ?? 0
    const session = deps.sessionId()
    if (session) {
      boundSession = session
      const ctx = await deps.native.contextSet(session, generation.value + 1)
      generation.value = ctx.generation
    }
    ready = true
  }

  async function startRecording(): Promise<void> {
    if (!ready || disposed) return
    if (state.value !== 'idle') {
      error.value = 'voice_busy'
      return
    }
    error.value = null
    deps.recorder.start()
    recorderOpen = true
    state.value = 'recording'
  }

  async function stopRecording(): Promise<void> {
    if (state.value !== 'recording') return
    state.value = 'transcribing'
    const identity = mintIdentity()
    pending.add(identity.request_id)
    try {
      const wav = await deps.recorder.stop()
      recorderOpen = false
      const reply = await deps.native.transcribe(identity, wav)
      if (isStale(identity)) return
      if (reply.status === 'transcribed') {
        draft.value = reply.text
      } else {
        // no_speech is an honest result, never an error, never a send.
        draft.value = ''
      }
    } catch (err) {
      if (!isStale(identity)) {
        error.value = err instanceof Error ? err.message : String(err)
        report('transcribe failed', { request_id: identity.request_id })
      }
    } finally {
      pending.delete(identity.request_id)
      recorderOpen = false
      if (state.value === 'transcribing') state.value = 'idle'
    }
  }

  function cancelRecording(): void {
    if (state.value === 'recording' || state.value === 'transcribing') {
      deps.recorder.cancel()
      recorderOpen = false
    }
    state.value = 'idle'
  }

  /** The one speak path: explicit replay and eligible auto-read both mint a
   * fresh utterance and stay fenced to the captured identity. */
  async function speak(runId: string | undefined, text: string): Promise<void> {
    if (!ready || disposed) return
    if (!text.trim()) return
    const identity = mintIdentity(runId)
    activeUtterance = identity.utterance_id
    pending.add(identity.request_id)
    state.value = 'speaking'
    try {
      const bytes = await deps.native.synthesize(identity, text)
      if (isStale(identity) || activeUtterance !== identity.utterance_id) return
      playerOpen = true
      playing.value = true
      await deps.player.play(bytes, identity.utterance_id)
      if (!isStale(identity) && activeUtterance === identity.utterance_id) {
        playing.value = false
      }
    } catch (err) {
      if (!isStale(identity)) {
        error.value = err instanceof Error ? err.message : String(err)
        report('synthesize failed', { request_id: identity.request_id })
      }
    } finally {
      pending.delete(identity.request_id)
      if (activeUtterance === identity.utterance_id) {
        playerOpen = false
        playing.value = false
      }
      if (state.value === 'speaking' && pending.size === 0) state.value = 'idle'
    }
  }

  async function requestReply(runId: string | undefined, text: string): Promise<void> {
    // Explicit message replay is always a fresh utterance — never reuse a
    // cached/in-flight one.
    await speak(runId, text)
  }

  async function notifyFreshReply(meta: FreshReplyMeta): Promise<void> {
    if (!autoRead.value) return
    if (meta.fromHistory || meta.child || meta.cancelled || meta.failed) return
    await speak(meta.runId, meta.text)
  }

  /** Cancel every in-flight native request and stop local playback first —
   * the ordering contract keeps stale bytes from arriving mid-advance. */
  function quiesce(): void {
    deps.player.stop()
    playing.value = false
    playerOpen = false
    activeUtterance = null
    if (state.value === 'recording' || state.value === 'transcribing') {
      deps.recorder.cancel()
      recorderOpen = false
    }
    for (const requestId of pending) {
      void deps.native.cancel(requestId).catch(() => undefined)
    }
    pending.clear()
    state.value = 'idle'
  }

  /** Interrupt: stop playback + cancel in-flight synth/transcribe without
   * touching the native window generation. */
  function stopSpeaking(): void {
    quiesce()
  }

  async function advanceContext(sessionId: string): Promise<void> {
    quiesce()
    boundSession = sessionId
    const ctx = await deps.native.contextSet(sessionId, generation.value + 1)
    generation.value = ctx.generation
  }

  function invalidate(reason: string): void {
    // DN-2B hook: drop session-bound voice work before history mutates.
    // Stop local media first; generation advances lazily on next bind.
    quiesce()
    report('voice invalidated', { reason }, 'info')
  }

  function dispose(): void {
    if (disposed) return
    disposed = true
    quiesce()
    deps.recorder.dispose()
    deps.player.dispose()
    recorderOpen = false
    playerOpen = false
  }

  return {
    state, draft, error, generation, playing, openResources, autoRead,
    init, startRecording, stopRecording, cancelRecording,
    requestReply, notifyFreshReply, stopSpeaking, advanceContext, invalidate, dispose,
  }
}

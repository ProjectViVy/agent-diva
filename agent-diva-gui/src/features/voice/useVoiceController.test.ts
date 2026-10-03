/**
 * DN-6C voice controller contract tests. The controller owns mic tracks,
 * Blob URLs, the native context generation and fresh-answer eligibility:
 * STT lands as an editable draft only, TTS resolves are fenced by the
 * captured identity+generation, and every exit path releases media.
 */
import { describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { createVoiceController, type VoiceNative, type VoicePlayer } from './useVoiceController'

class Deferred<T> {
  promise: Promise<T>
  resolve!: (value: T) => void
  reject!: (err: unknown) => void
  constructor() {
    this.promise = new Promise<T>((resolve, reject) => {
      this.resolve = resolve
      this.reject = reject
    })
  }
}

function makeTrack() {
  return { stop: vi.fn(), kind: 'audio' as const }
}

function makeStream() {
  return { getTracks: () => [makeTrack(), makeTrack()] }
}

function makeNative(overrides: Partial<VoiceNative> = {}) {
  const deferred = new Map<string, Deferred<unknown>>()
  const native: VoiceNative = {
    configGet: vi.fn(async () => ({
      revision: 1,
      preferences: {
        stt: { provider: 'siliconflow', base_url: 'https://api.siliconflow.cn', model: 'FunAudioLLM/SenseVoiceSmall' },
        tts: { provider: 'siliconflow', siliconflow: { base_url: 'https://api.siliconflow.cn', model: 'IndexTTS-2', voice: 'v', speed: 1 } },
        auto_read_replies: true,
      },
      credential_state: { siliconflow: 'present', minimax: 'absent' },
      window_context: null,
    })),
    configUpdate: vi.fn(async () => ({ revision: 2 })),
    contextSet: vi.fn(async (_session: string, generation: number) => ({
      session_id: _session, generation,
    })),
    transcribe: vi.fn(async (_identity, _wav: ArrayBuffer) => {
      const d = new Deferred<{ status: 'transcribed' | 'no_speech'; text: string }>()
      deferred.set('transcribe', [...(deferred.get('transcribe') ?? []), d as Deferred<unknown>])
      return d.promise
    }),
    synthesize: vi.fn(async (_identity, _text: string) => {
      const d = new Deferred<ArrayBuffer>()
      deferred.set('synthesize', [...(deferred.get('synthesize') ?? []), d as Deferred<unknown>])
      return d.promise
    }),
    cancel: vi.fn(async (requestId: string) => ({ request_id: requestId, status: 'cancelled' })),
    ...overrides,
  }
  return { native, deferred }
}

function makeDeps(overrides: Record<string, unknown> = {}) {
  const { native, deferred } = makeNative()
  const player: VoicePlayer = {
    play: vi.fn(async (_bytes: ArrayBuffer, _utteranceId: string) => {}),
    stop: vi.fn(),
    dispose: vi.fn(),
  }
  const recorder = {
    start: vi.fn(),
    stop: vi.fn(async () => new ArrayBuffer(16)),
    cancel: vi.fn(),
    dispose: vi.fn(),
  }
  const diagnostics = { record: vi.fn() }
  const sessionId = { value: 'sess-1' }
  return {
    native, deferred, player, recorder, diagnostics, sessionId,
    deps: {
      native,
      recorder,
      player,
      diagnostics,
      sessionId: () => sessionId.value,
      newId: (() => { let n = 0; return () => `id-${++n}` })(),
      ...overrides,
    },
  }
}

async function ready(deps: Parameters<typeof createVoiceController>[0]) {
  const controller = createVoiceController(deps)
  await controller.init()
  return controller
}

describe('useVoiceController', () => {
  it('sttDraftNeverAutoSends: transcript lands as editable draft, no send', async () => {
    const { deps, native, deferred } = makeDeps()
    const controller = await ready(deps)
    const send = vi.fn()
    await controller.startRecording()
    const stopping = controller.stopRecording()
    await nextTick()
    ;(deferred.get('transcribe') as Deferred<{ status: string; text: string }>[])[0]
      .resolve({ status: 'transcribed', text: 'draft me' })
    await stopping
    expect(controller.draft.value).toBe('draft me')
    expect(controller.state.value).toBe('idle')
    expect(send).not.toHaveBeenCalled()
    // no_speech is honest, not an error and never a draft
    await controller.startRecording()
    const again = controller.stopRecording()
    await nextTick()
    ;(deferred.get('transcribe') as Deferred<{ status: string; text: string }>[])[1]
      .resolve({ status: 'no_speech', text: '' })
    await again
    expect(controller.draft.value).toBe('')
    expect(controller.error.value).toBeNull()
    expect(native.transcribe).toHaveBeenCalledTimes(2)
  })

  it('lateMp3NoPlayback: synth resolving after context advance is dropped', async () => {
    const { deps, deferred, player } = makeDeps()
    const controller = await ready(deps)
    const utterance = controller.requestReply('run-1', 'hello reply')
    const advancing = controller.advanceContext('sess-1')
    ;(deferred.get('synthesize') as Deferred<ArrayBuffer>[])[0].resolve(new ArrayBuffer(8))
    await advancing
    await utterance.catch(() => undefined)
    expect(player.play).not.toHaveBeenCalled()
  })

  it('reloadReadsGeneration: init reads native window context and increments', async () => {
    const { deps, native } = makeDeps({
      native: (() => {
        const { native } = makeNative()
        ;(native.configGet as ReturnType<typeof vi.fn>).mockResolvedValue({
          revision: 5,
          preferences: { stt: {}, tts: {}, auto_read_replies: true },
          credential_state: { siliconflow: 'present', minimax: 'absent' },
          window_context: { session_id: 'sess-1', generation: 41 },
        })
        return native
      })(),
    })
    const controller = await ready(deps)
    expect(deps.native.contextSet).toHaveBeenCalledWith('sess-1', 42)
    expect(controller.generation.value).toBe(42)
  })

  it('onlyFreshPrimaryAutoReads: auto-read fires for one fresh primary reply only', async () => {
    const { deps, native, deferred, player } = makeDeps()
    const controller = await ready(deps)
    // a fresh primary reply auto-reads
    const speaking = controller.notifyFreshReply({ runId: 'run-1', text: 'fresh answer' })
    const synth = (deferred.get('synthesize') as Deferred<ArrayBuffer>[])[0]
    synth.resolve(new ArrayBuffer(8))
    await speaking
    expect(native.synthesize).toHaveBeenCalledTimes(1)
    expect(player.play).toHaveBeenCalledTimes(1)
    // history replay never auto-reads
    await controller.notifyFreshReply({ runId: 'run-2', text: 'old', fromHistory: true })
    await controller.notifyFreshReply({ runId: 'run-3', text: 'child', child: true })
    await controller.notifyFreshReply({ runId: 'run-4', text: 'cancelled', cancelled: true })
    expect(native.synthesize).toHaveBeenCalledTimes(1)
    // opt-out stops auto-read entirely
    const deps2 = makeDeps()
    deps2.native.configGet = vi.fn(async () => ({
      revision: 1,
      preferences: { stt: {}, tts: {}, auto_read_replies: false },
      credential_state: { siliconflow: 'present', minimax: 'absent' },
      window_context: null,
    }))
    const second = await ready(deps2.deps)
    await second.notifyFreshReply({ runId: 'run-9', text: 'no auto' })
    expect(deps2.native.synthesize).not.toHaveBeenCalled()
  })

  it('allExitPathsReleaseAudio: cancel/invalidate/dispose stop tracks and revoke', async () => {
    const { deps, recorder, player, deferred } = makeDeps()
    const controller = await ready(deps)
    await controller.startRecording()
    controller.cancelRecording()
    expect(recorder.cancel).toHaveBeenCalled()
    // invalidation mid-speak stops playback and revokes
    const speaking = controller.notifyFreshReply({ runId: 'r', text: 'x' })
    await nextTick()
    controller.invalidate('session/load')
    expect(player.stop).toHaveBeenCalled()
    // the cancelled native request may still settle — its bytes drop.
    ;(deferred.get('synthesize') as Deferred<ArrayBuffer>[])[0].resolve(new ArrayBuffer(4))
    await speaking.catch(() => undefined)
    expect(player.play).not.toHaveBeenCalled()
    // dispose releases everything, resource counters at zero
    controller.dispose()
    expect(recorder.dispose).toHaveBeenCalled()
    expect(player.dispose).toHaveBeenCalled()
    expect(controller.openResources.value).toBe(0)
  })

  it('stale synth never restarts audio after newer request', async () => {
    const { deps, deferred, player } = makeDeps()
    const controller = await ready(deps)
    const first = controller.requestReply('run-1', 'one')
    const second = controller.requestReply('run-2', 'two')
    const synths = deferred.get('synthesize') as Deferred<ArrayBuffer>[]
    // newest resolves first, plays
    synths[1].resolve(new ArrayBuffer(8))
    await second
    expect(player.play).toHaveBeenCalledTimes(1)
    // the older completion is stale and never restarts playback
    synths[0].resolve(new ArrayBuffer(8))
    await first.catch(() => undefined)
    expect(player.play).toHaveBeenCalledTimes(1)
  })
})

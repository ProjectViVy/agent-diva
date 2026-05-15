import type { VRM } from '@pixiv/three-vrm'
import {
  AVATAR_COMMAND_NAMES,
  AVATAR_EVENT_NAMES,
  type AvatarAnimationState,
  type AvatarCharacterSpec,
  type AvatarInitOptions,
  type AvatarMotionEntry,
  type AvatarMotionState,
  type AvatarMotionStatePatch,
  type AvatarMood,
  type AvatarRuntime,
  type AvatarRuntimeCommandHandlers,
  type AvatarRuntimeErrorPayload,
  type AvatarSpeechState,
  type AvatarTransform,
  type AvatarViewportSize,
  type ChunkPlayPayload,
  type SubtitlePayload,
} from '../protocol'
import { RuntimeBridge } from './bridge'
import { AnimationController } from './animation-controller'
import ChunkAnimationController from './chunk-controller'
import type { ChunkControllerHooks } from './chunk-controller'
import { DEFAULT_CAPABILITIES, RUNTIME_VERSION } from './constants'
import { ExpressionController } from './expression-controller'
import HoverAutoHideController from './hover-auto-hide-controller'
import { LookAtController } from './lookat-controller'
import { VrmModelLoader } from './model-loader'
import { MotionController } from './motion-controller'
import PointerLockController from './pointerlock-controller'
import PttController from './ptt-controller'
import type { PttControllerHooks } from './ptt-controller'
import { SceneManager } from './scene-manager'
import { SpeechController } from './speech-controller'
import SubtitleController from './subtitle-controller'
import { TransformController } from './transform-controller'
import { VmcController } from './vmc-controller'
import type { GaussSceneId } from './gauss-scene-controller'

export class VrmRuntime implements AvatarRuntime {
  private readonly bridgeInternal = new RuntimeBridge()
  private readonly sceneManager: SceneManager
  private readonly modelLoader = new VrmModelLoader()
  private readonly expressionController = new ExpressionController()
  private readonly speechController = new SpeechController()
  private readonly lookAtController = new LookAtController()
  private readonly vmcController = new VmcController()
  private readonly animationController = new AnimationController({
    onStateChange: (state) =>
      this.bridgeInternal.emit(AVATAR_EVENT_NAMES.animationStateChange, state),
    onError: (error) => this.bridgeInternal.emit(AVATAR_EVENT_NAMES.animationError, error),
  })
  private readonly motionController = new MotionController({
    onCatalogChange: (catalog) =>
      this.bridgeInternal.emit(AVATAR_EVENT_NAMES.motionCatalogChange, catalog),
    onStateChange: (state) =>
      this.bridgeInternal.emit(AVATAR_EVENT_NAMES.motionStateChange, state),
    onMotionStart: (motion) =>
      this.bridgeInternal.emit(AVATAR_EVENT_NAMES.motionStart, motion),
    onMotionEnd: (motion) =>
      this.bridgeInternal.emit(AVATAR_EVENT_NAMES.motionEnd, motion),
    onError: (error) => this.bridgeInternal.emit(AVATAR_EVENT_NAMES.motionError, error),
    onProceduralIdleSuppressionChange: (suppressed) =>
      this.animationController.setProceduralIdleSuppressed(suppressed),
  })
  private readonly chunkController = new ChunkAnimationController({
    onChunkStart: (chunkId, expression) =>
      this.bridgeInternal.emit(AVATAR_EVENT_NAMES.chunkStart, { chunkId, expression }),
    onChunkEnd: (chunkId, expression) =>
      this.bridgeInternal.emit(AVATAR_EVENT_NAMES.chunkEnd, { chunkId, expression }),
    onAllChunksEnd: () =>
      this.bridgeInternal.emit(AVATAR_EVENT_NAMES.chunkAllEnd, undefined),
  })
  private readonly hoverAutoHideController = new HoverAutoHideController()
  private readonly pointerLockController: PointerLockController
  private readonly pttController: PttController
  private readonly subtitleController: SubtitleController
  private readonly transformController: TransformController

  private initialized = false
  private currentModel: VRM | null = null
  private speechState: AvatarSpeechState = { speaking: false }
  private currentMood: AvatarMood = 'neutral'
  private paused = false
  private disposeFrameHandler: (() => void) | null = null
  private interactionBound = false
  private vmcLastSent = 0
  private readonly VMC_SEND_INTERVAL = 1000 / 30 // 30 fps throttling

  constructor(
    private readonly container: HTMLElement,
    options: AvatarInitOptions,
  ) {
    this.sceneManager = new SceneManager(this.container, options)
    this.transformController = new TransformController(
      this.sceneManager.camera,
      this.sceneManager.controls,
    )
    this.pointerLockController = new PointerLockController(
      this.sceneManager.camera,
      this.sceneManager.renderer.domElement,
      this.sceneManager.scene,
    )
    this.pointerLockController.attach()
    this.pttController = new PttController({
      onStateChange: (state) =>
        this.bridgeInternal.emit(AVATAR_EVENT_NAMES.pttStateChange, state),
      onAudioReady: (payload) =>
        this.bridgeInternal.emit(AVATAR_EVENT_NAMES.pttAudioReady, payload),
      onTranscription: (payload) =>
        this.bridgeInternal.emit(AVATAR_EVENT_NAMES.pttTranscription, payload),
      onError: (error) =>
        this.bridgeInternal.emit(AVATAR_EVENT_NAMES.runtimeError, {
          code: error.code,
          message: error.message,
          recoverable: true,
        }),
    })
    this.subtitleController = new SubtitleController(this.container)
    this.disposeFrameHandler = this.sceneManager.onFrame((deltaSeconds) => {
      // Bone-level look-at must run BEFORE vrm.update() to avoid
      // having its quaternion changes overwritten by spring bones.
      this.lookAtController.update(deltaSeconds)
      this.currentModel?.update(deltaSeconds)
      this.speechController.update(deltaSeconds)

      // Only update the procedural animation controller (idle/breath/blink)
      // when no VRMA clip is actively playing.  The MotionController's
      // mixer and the AnimationController's mixer both operate on the
      // same vrm.scene, and the procedural idle animates 12 body bones
      // (spine/chest/neck/arms/hands/shoulders) that overlap with VRMA
      // bone targets.  Running both simultaneously causes the bones to
      // fight between two independent mixers → visible position jumps.
      if (!this.motionController.hasActiveBinding()) {
        this.animationController.update(deltaSeconds)
      }

      this.motionController.update(deltaSeconds)
      this.pointerLockController.update(deltaSeconds)

      // Throttled VMC frame emission
      const now = performance.now()
      if (now - this.vmcLastSent >= this.VMC_SEND_INTERVAL) {
        this.vmcLastSent = now
        const bones = this.vmcController.getBoneData()
        const blends = this.vmcController.getBlendData()
        if (bones.length > 0) {
          void this.bridgeInternal.emit(AVATAR_EVENT_NAMES.vmcFrame, {
            bones,
            blends,
            timestamp: now,
          })
        }
      }
    })
    this.sceneManager.setMetricsHandler((metrics) => {
      void this.bridgeInternal.emit(AVATAR_EVENT_NAMES.metrics, metrics)
    })
  }

  get bridge(): RuntimeBridge {
    return this.bridgeInternal
  }

  async init(options: AvatarInitOptions): Promise<void> {
    this.sceneManager.setInteractionEnabled(options.allowInteraction)
    this.registerCommandHandlers()
    this.initialized = true
    this.bindInteractionEvents()
    this.sceneManager.resume()
    await this.bridgeInternal.emit(AVATAR_EVENT_NAMES.ready, {
      runtimeVersion: RUNTIME_VERSION,
      capabilities: [...DEFAULT_CAPABILITIES],
    })
  }

  async loadCharacter(spec: AvatarCharacterSpec): Promise<void> {
    this.assertInitialized()

    if (spec.kind !== 'vrm') {
      const error = this.createError('UNSUPPORTED_CHARACTER_KIND', `Unsupported avatar kind: ${spec.kind}`, true, {
        requestedKind: spec.kind,
      })
      await this.reportRuntimeError(error, spec.id)
      throw new Error(error.message)
    }

    await this.bridgeInternal.emit(AVATAR_EVENT_NAMES.loadStart, {
      characterId: spec.id,
    })

    try {
      this.unloadCurrentModel()
      const vrm = await this.modelLoader.load(spec.modelSource, this.sceneManager.camera)
      this.currentModel = vrm
      this.sceneManager.scene.add(vrm.scene)
      this.transformController.attachModel(vrm.scene)
      this.expressionController.attach(vrm)
      this.speechController.attach(vrm)
      this.lookAtController.attach(vrm, this.sceneManager.camera)
      this.vmcController.attach(vrm)
      this.animationController.attach(vrm)
      this.chunkController.attach(vrm)
      this.hoverAutoHideController.attach(
        this.sceneManager.renderer,
        this.sceneManager.camera,
        vrm.scene,
      )
      try {
        await this.motionController.attach(vrm)
      } catch (error) {
        await this.bridgeInternal.emit(AVATAR_EVENT_NAMES.motionError, {
          code: 'MOTION_INIT_FAILED',
          message: error instanceof Error ? error.message : String(error),
          recoverable: true,
        })
      }
      this.currentMood = spec.initialMood ?? 'neutral'
      this.expressionController.setMood(this.currentMood)
      this.speechController.setState(this.speechState)
      if (this.paused) {
        this.animationController.pause()
        this.motionController.pause()
      }
      await this.bridgeInternal.emit(AVATAR_EVENT_NAMES.loadSuccess, {
        characterId: spec.id,
      })
    } catch (error) {
      const runtimeError = this.createError(
        'VRM_LOAD_FAILED',
        error instanceof Error ? error.message : String(error),
        true,
      )
      await this.bridgeInternal.emit(AVATAR_EVENT_NAMES.loadError, {
        characterId: spec.id,
        error: runtimeError,
      })
      await this.bridgeInternal.emit(AVATAR_EVENT_NAMES.runtimeError, runtimeError)
      throw error
    }
  }

  setMood(mood: AvatarMood): void {
    this.currentMood = mood
    this.expressionController.setMood(mood)
  }

  async setSpeechState(state: AvatarSpeechState): Promise<void> {
    const next = {
      speaking: state.speaking,
      viseme: state.viseme ?? null,
      intensity: state.intensity ?? 0.8,
    }
    const wasSpeaking = this.speechState.speaking
    this.speechState = next
    this.speechController.setState(next)

    if (!wasSpeaking && next.speaking) {
      await this.bridgeInternal.emit(AVATAR_EVENT_NAMES.speechStart, {
        viseme: next.viseme ?? null,
      })
    }
    if (wasSpeaking && !next.speaking) {
      await this.bridgeInternal.emit(AVATAR_EVENT_NAMES.speechEnd, undefined)
    }
  }

  async setAnimationState(state: Partial<AvatarAnimationState>): Promise<void> {
    this.animationController.setState(state)
  }

  getAnimationState(): AvatarAnimationState {
    return this.animationController.getState()
  }

  getMotionCatalog(): AvatarMotionEntry[] {
    return this.motionController.getCatalog()
  }

  getMotionState(): AvatarMotionState {
    return this.motionController.getState()
  }

  async setMotionState(state: AvatarMotionStatePatch): Promise<void> {
    this.motionController.setState(state)
  }

  async playMotion(id: string): Promise<boolean> {
    return this.motionController.playMotion(id)
  }

  stopMotion(): boolean {
    return this.motionController.stopMotion()
  }

  async setTransform(transform: Partial<AvatarTransform>): Promise<void> {
    const next = this.transformController.setTransform(transform)
    await this.bridgeInternal.emit(AVATAR_EVENT_NAMES.transformChange, next)
  }

  getTransform(): AvatarTransform {
    return this.transformController.getTransform()
  }

  pause(): void {
    this.paused = true
    this.sceneManager.pause()
    this.animationController.pause()
    this.motionController.pause()
  }

  resume(): void {
    this.paused = false
    this.animationController.resume()
    this.motionController.resume()
    this.sceneManager.resume()
  }

  resize(size: AvatarViewportSize): void {
    this.sceneManager.resize(size)
  }

  async setBackgroundScene(
    sceneId: GaussSceneId | string,
    url?: string,
  ): Promise<void> {
    await this.sceneManager.setBackgroundScene(sceneId, url)
  }

  /**
   * Switch between normal perspective rendering and 360° equirectangular
   * panorama mode (useful for OBS streaming).
   *
   * @param mode - `'normal'` for standard perspective, `'panorama'` for
   *   CubeCamera-based equirectangular output.
   */
  setRenderMode(mode: 'normal' | 'panorama'): void {
    this.sceneManager.setRenderMode(mode)
  }

  setShadowEnabled(enabled: boolean): void {
    this.sceneManager.setShadowEnabled(enabled)
  }

  // ── Tier 3: Hover Auto-Hide ──────────────────────────────────────────

  /**
   * Enable or disable auto-hide when the mouse hovers over the model.
   * When enabled, the model fades out on hover and fades back in when
   * the mouse moves away.
   */
  setHoverAutoHide(enabled: boolean): void {
    this.hoverAutoHideController.setEnabled(enabled)
    void this.bridgeInternal.emit(
      AVATAR_EVENT_NAMES.hoverStateChange,
      this.hoverAutoHideController.getState(),
    )
  }

  // ── Tier 3: Pointer Lock ─────────────────────────────────────────────

  /**
   * Toggle first-person PointerLock controls on or off.
   * When locked, WASD+QE keys control camera movement.
   */
  setPointerLock(locked: boolean): void {
    if (locked) {
      this.pointerLockController.lock()
    } else {
      this.pointerLockController.unlock()
    }
    void this.bridgeInternal.emit(
      AVATAR_EVENT_NAMES.pointerlockChange,
      this.pointerLockController.getState(),
    )
  }

  // ── Tier 3: Chunk Animation ──────────────────────────────────────────

  /**
   * Play an audio chunk with formant-based lip sync animation.
   * The audio data URL (base64) is decoded and analysed in real-time
   * to drive the VRM vowel blendshapes (aa, ih, ou, ee, oh).
   */
  async chunkPlay(payload: ChunkPlayPayload): Promise<void> {
    const audioBuffer = await this.decodeAudioDataUrl(payload.audioDataUrl)
    await this.chunkController.playChunk(
      payload.chunkId,
      audioBuffer,
      payload.expression,
    )
  }

  /**
   * Stop a specific chunk by ID. Cancels animation and releases
   * audio resources. Fires {@link AVATAR_EVENT_NAMES.chunkEnd}.
   */
  chunkStop(payload: { chunkId: string }): void {
    this.chunkController.stopChunk(payload.chunkId)
  }

  /**
   * Stop all active chunk animations immediately.
   */
  chunkStopAll(): void {
    this.chunkController.stopAll()
  }

  // ── Tier 3: Push-to-Talk ─────────────────────────────────────────────

  /**
   * Start push-to-talk microphone recording.
   * Once stopped, the audio is encoded to 16kHz mono WAV and
   * emitted via {@link AVATAR_EVENT_NAMES.pttAudioReady}.
   */
  async pttStart(): Promise<void> {
    await this.pttController.start()
  }

  /** Stop active push-to-talk recording and encode captured audio. */
  pttStop(): void {
    this.pttController.stop()
  }

  // ── Tier 3: Subtitle Overlay ─────────────────────────────────────────

  /** Show or update the subtitle overlay with the given text. */
  subtitleUpdate(payload: SubtitlePayload): void {
    this.subtitleController.setText(payload.text, payload.chunkIndex)
    void this.bridgeInternal.emit(AVATAR_EVENT_NAMES.subtitleChange, payload)
  }

  /** Fade out and clear the subtitle overlay. */
  subtitleClear(): void {
    this.subtitleController.clear()
  }

  // ── Lifecycle ────────────────────────────────────────────────────────

  async destroy(): Promise<void> {
    this.unloadCurrentModel()
    this.pointerLockController.detach()
    this.pttController.destroy()
    this.subtitleController.destroy()
    this.disposeFrameHandler?.()
    this.disposeFrameHandler = null
    this.sceneManager.destroy()
  }

  private bindInteractionEvents(): void {
    if (this.interactionBound) {
      return
    }

    const controls = this.sceneManager.controls
    controls.addEventListener('start', () => {
      void this.bridgeInternal.emit(AVATAR_EVENT_NAMES.interactionStart, {
        kind: 'rotate',
      })
    })
    controls.addEventListener('change', () => {
      const next = this.transformController.syncFromInteraction()
      void this.bridgeInternal.emit(AVATAR_EVENT_NAMES.transformChange, next)
    })
    controls.addEventListener('end', () => {
      void this.bridgeInternal.emit(AVATAR_EVENT_NAMES.interactionEnd, {
        kind: 'rotate',
      })
    })
    this.interactionBound = true
  }

  private registerCommandHandlers(): void {
    const handlers: AvatarRuntimeCommandHandlers = {
      [AVATAR_COMMAND_NAMES.init]: async (payload) => {
        await this.init(payload)
      },
      [AVATAR_COMMAND_NAMES.loadCharacter]: async (payload) => {
        await this.loadCharacter(payload)
      },
      [AVATAR_COMMAND_NAMES.setMood]: async (payload) => {
        await this.setMood(payload.mood)
      },
      [AVATAR_COMMAND_NAMES.setSpeechState]: async (payload) => {
        await this.setSpeechState(payload)
      },
      [AVATAR_COMMAND_NAMES.setAnimationState]: async (payload) => {
        await this.setAnimationState(payload)
      },
      [AVATAR_COMMAND_NAMES.setMotionState]: async (payload) => {
        await this.setMotionState(payload)
      },
      [AVATAR_COMMAND_NAMES.playMotion]: async (payload) => {
        await this.playMotion(payload.motionId)
      },
      [AVATAR_COMMAND_NAMES.stopMotion]: async () => {
        this.stopMotion()
      },
      [AVATAR_COMMAND_NAMES.setTransform]: async (payload) => {
        await this.setTransform(payload)
      },
      [AVATAR_COMMAND_NAMES.pause]: async () => {
        this.pause()
      },
      [AVATAR_COMMAND_NAMES.resume]: async () => {
        this.resume()
      },
      [AVATAR_COMMAND_NAMES.resize]: async (payload) => {
        this.resize(payload)
      },
      [AVATAR_COMMAND_NAMES.destroy]: async () => {
        await this.destroy()
      },
      [AVATAR_COMMAND_NAMES.setHoverAutoHide]: async (payload) => {
        this.setHoverAutoHide(payload.enabled)
      },
      [AVATAR_COMMAND_NAMES.setPointerLock]: async (payload) => {
        this.setPointerLock(payload.locked)
      },
      [AVATAR_COMMAND_NAMES.chunkPlay]: async (payload) => {
        await this.chunkPlay(payload)
      },
      [AVATAR_COMMAND_NAMES.chunkStop]: async (payload) => {
        this.chunkStop(payload)
      },
      [AVATAR_COMMAND_NAMES.chunkStopAll]: async () => {
        this.chunkStopAll()
      },
      [AVATAR_COMMAND_NAMES.pttStart]: async () => {
        await this.pttStart()
      },
      [AVATAR_COMMAND_NAMES.pttStop]: async () => {
        this.pttStop()
      },
      [AVATAR_COMMAND_NAMES.subtitleUpdate]: async (payload) => {
        this.subtitleUpdate(payload)
      },
      [AVATAR_COMMAND_NAMES.subtitleClear]: async () => {
        this.subtitleClear()
      },
    }
    this.bridgeInternal.setCommandHandlers(handlers)
  }

  private unloadCurrentModel(): void {
    if (!this.currentModel) {
      this.animationController.detach()
      this.chunkController.detach()
      this.hoverAutoHideController.detach()
      this.motionController.detach()
      this.lookAtController.detach()
      this.vmcController.detach()
      this.transformController.attachModel(null)
      return
    }

    this.sceneManager.scene.remove(this.currentModel.scene)
    this.expressionController.detach()
    this.speechController.detach()
    this.lookAtController.detach()
    this.vmcController.detach()
    this.animationController.detach()
    this.chunkController.detach()
    this.hoverAutoHideController.detach()
    this.motionController.detach()
    this.transformController.attachModel(null)
    this.modelLoader.dispose(this.currentModel)
    this.currentModel = null
  }

  private assertInitialized(): void {
    if (!this.initialized) {
      throw new Error('Runtime must be initialized before use')
    }
  }

  private createError(
    code: string,
    message: string,
    recoverable: boolean,
    detail?: Record<string, string | number | boolean | null>,
  ): AvatarRuntimeErrorPayload {
    return { code, message, recoverable, detail }
  }

  private async reportRuntimeError(
    error: AvatarRuntimeErrorPayload,
    characterId?: string,
  ): Promise<void> {
    if (characterId) {
      await this.bridgeInternal.emit(AVATAR_EVENT_NAMES.loadError, {
        characterId,
        error,
      })
    }
    await this.bridgeInternal.emit(AVATAR_EVENT_NAMES.runtimeError, error)
  }

  /**
   * Decode a base64 audio data URL into an {@link AudioBuffer} for use
   * with the chunk animation controller.
   */
  private async decodeAudioDataUrl(dataUrl: string): Promise<AudioBuffer> {
    const response = await fetch(dataUrl)
    const arrayBuffer = await response.arrayBuffer()
    const audioContext = new AudioContext()
    const audioBuffer = await audioContext.decodeAudioData(arrayBuffer)
    audioContext.close()
    return audioBuffer
  }
}

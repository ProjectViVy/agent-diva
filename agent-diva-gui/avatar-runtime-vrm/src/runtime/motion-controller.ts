import type { VRM } from '@pixiv/three-vrm'
import type { AnimationAction, AnimationMixer, Event as ThreeEvent } from 'three'
import { LoopOnce, AnimationMixer as ThreeAnimationMixer } from 'three'
import type {
  AvatarMotionEntry,
  AvatarMotionEventPayload,
  AvatarMotionState,
  AvatarRuntimeErrorPayload,
} from '../protocol'
import { getBuiltinMotionCatalog } from './motion-catalog'
import {
  DEFAULT_MOTION_STATE,
  type MotionClipBinding,
  type MotionControllerHooks,
  type MotionStatePatch,
} from './motion-types'
import { VrmaLoader } from './vrma-loader'

interface FinishedEvent extends ThreeEvent {
  action?: AnimationAction
}

export class MotionController {
  private readonly catalog = getBuiltinMotionCatalog()
  private readonly vrmaLoader = new VrmaLoader()

  private mixer: AnimationMixer | null = null
  private state: AvatarMotionState = { ...DEFAULT_MOTION_STATE }
  private bindings = new Map<string, MotionClipBinding>()
  private idleMotionIds: string[] = []
  private idleCursor = -1
  private currentIdleBinding: MotionClipBinding | null = null
  private currentOneShotBinding: MotionClipBinding | null = null
  private readonly handleFinished = (event: FinishedEvent) => {
    this.onMixerFinished(event.action ?? null)
  }

  constructor(private readonly hooks: MotionControllerHooks = {}) {}

  getCatalog(): AvatarMotionEntry[] {
    return this.catalog.map((motion) => ({ ...motion }))
  }

  getState(): AvatarMotionState {
    return { ...this.state }
  }

  async attach(vrm: VRM): Promise<void> {
    this.detach()
    this.mixer = new ThreeAnimationMixer(vrm.scene)

    try {
      const bindings = await Promise.all(
        this.catalog.map(async (entry) => {
          const clip = await this.vrmaLoader.loadClip(entry.source, vrm)
          const action = this.mixer!.clipAction(clip)
          action.setLoop(LoopOnce, 1)
          action.clampWhenFinished = true
          action.enabled = false
          action.paused = !this.state.runtimePlaying
          return { entry, clip, action }
        }),
      )

      for (const binding of bindings) {
        this.bindings.set(binding.entry.id, binding)
      }
      this.idleMotionIds = bindings
        .filter((binding) => binding.entry.kind === 'idle')
        .map((binding) => binding.entry.id)
      this.mixer.addEventListener('finished', this.handleFinished)
      this.updateState({ ...DEFAULT_MOTION_STATE, runtimePlaying: true })
      void this.hooks.onCatalogChange?.(this.getCatalog())
      this.syncProceduralIdleSuppression()
      this.ensureIdlePlayback()
    } catch (error) {
      this.detach()
      this.reportError({
        code: 'MOTION_INIT_FAILED',
        message: error instanceof Error ? error.message : String(error),
        recoverable: true,
      })
      throw error
    }
  }

  detach(): void {
    if (this.mixer) {
      this.mixer.removeEventListener('finished', this.handleFinished)
      this.mixer.stopAllAction()
      this.mixer.uncacheRoot(this.mixer.getRoot())
    }

    this.currentIdleBinding = null
    this.currentOneShotBinding = null
    this.bindings.clear()
    this.idleMotionIds = []
    this.idleCursor = -1
    this.mixer = null
    this.updateState({ ...DEFAULT_MOTION_STATE })
    this.syncProceduralIdleSuppression()
  }

  update(deltaSeconds: number): void {
    if (!this.state.runtimePlaying) {
      return
    }

    this.mixer?.update(deltaSeconds)
  }

  /**
   * Returns true when a VRMA animation (one-shot or idle) is actively
   * bound and playing.  The procedural idle system MUST be paused while
   * any VRMA clip is active, because both mixers operate on the same
   * vrm.scene and the procedural idle clip animates 12 body bones
   * (spine/chest/neck/head/arms/hands/shoulders) that overlap with
   * typical VRMA bone targets.  Two concurrent mixers on the same bones
   * produce visible position jumps.
   */
  hasActiveBinding(): boolean {
    return this.currentIdleBinding !== null || this.currentOneShotBinding !== null
  }

  pause(): void {
    this.updateState({ runtimePlaying: false })
    this.syncPausedState()
  }

  resume(): void {
    if (!this.mixer) {
      return
    }

    this.updateState({ runtimePlaying: true })
    this.syncPausedState()
  }

  setState(patch: MotionStatePatch): AvatarMotionState {
    if (patch.idleEnabled === undefined || patch.idleEnabled === this.state.idleEnabled) {
      return this.getState()
    }

    this.updateState({ idleEnabled: patch.idleEnabled })
    if (patch.idleEnabled) {
      this.ensureIdlePlayback()
    } else if (!this.currentOneShotBinding) {
      this.stopIdlePlayback()
      this.updateState({
        activeMotionId: null,
        activeMotionKind: null,
        idlePlaying: false,
      })
    } else {
      this.stopIdlePlayback()
      this.updateState({ idlePlaying: false })
    }
    this.syncProceduralIdleSuppression()
    return this.getState()
  }

  async playMotion(id: string): Promise<boolean> {
    const binding = this.bindings.get(id)
    if (!binding) {
      this.reportError({
        code: 'MOTION_NOT_FOUND',
        message: `Motion "${id}" is not available in the catalog.`,
        recoverable: true,
        detail: { motionId: id },
      })
      return false
    }

    if (binding.entry.kind !== 'oneshot') {
      this.reportError({
        code: 'MOTION_NOT_ONESHOT',
        message: `Motion "${id}" is not a one-shot motion.`,
        recoverable: true,
        detail: { motionId: id, kind: binding.entry.kind },
      })
      return false
    }

    if (this.currentOneShotBinding) {
      this.reportError({
        code: 'MOTION_BUSY',
        message: `Motion "${this.currentOneShotBinding.entry.id}" is already playing.`,
        recoverable: true,
        detail: {
          activeMotionId: this.currentOneShotBinding.entry.id,
          requestedMotionId: id,
        },
      })
      return false
    }

    this.stopIdlePlayback()
    this.syncProceduralIdleSuppression()
    this.currentOneShotBinding = binding
    this.playBinding(binding)
    this.updateState({
      activeMotionId: binding.entry.id,
      activeMotionKind: binding.entry.kind,
      idlePlaying: false,
      oneShotPlaying: true,
    })
    await this.emitMotionStart(binding.entry)
    return true
  }

  stopMotion(): boolean {
    if (!this.currentOneShotBinding) {
      return false
    }

    const ended = this.currentOneShotBinding.entry
    this.currentOneShotBinding.action.stop()
    this.currentOneShotBinding.action.enabled = false
    this.currentOneShotBinding = null
    void this.emitMotionEnd(ended)
    this.updateState({
      activeMotionId: null,
      activeMotionKind: null,
      oneShotPlaying: false,
    })
    this.ensureIdlePlayback()
    return true
  }

  private playBinding(binding: MotionClipBinding): void {
    binding.action.enabled = true
    binding.action.paused = !this.state.runtimePlaying
    binding.action.reset()
    binding.action.setEffectiveWeight(1)
    binding.action.play()
  }

  private stopIdlePlayback(): void {
    if (!this.currentIdleBinding) {
      return
    }

    this.currentIdleBinding.action.stop()
    this.currentIdleBinding.action.enabled = false
    this.currentIdleBinding = null
  }

  private ensureIdlePlayback(): void {
    if (!this.mixer || !this.state.idleEnabled || this.currentOneShotBinding) {
      this.syncProceduralIdleSuppression()
      return
    }

    if (this.currentIdleBinding) {
      this.updateState({
        activeMotionId: this.currentIdleBinding.entry.id,
        activeMotionKind: this.currentIdleBinding.entry.kind,
        idlePlaying: true,
      })
      this.syncProceduralIdleSuppression()
      return
    }

    const nextIdle = this.pickNextIdleBinding()
    if (!nextIdle) {
      this.updateState({
        activeMotionId: null,
        activeMotionKind: null,
        idlePlaying: false,
      })
      this.syncProceduralIdleSuppression()
      return
    }

    this.currentIdleBinding = nextIdle
    this.playBinding(nextIdle)
    this.updateState({
      activeMotionId: nextIdle.entry.id,
      activeMotionKind: nextIdle.entry.kind,
      idlePlaying: true,
      oneShotPlaying: false,
    })
    void this.emitMotionStart(nextIdle.entry)
    this.syncProceduralIdleSuppression()
  }

  private pickNextIdleBinding(): MotionClipBinding | null {
    if (this.idleMotionIds.length === 0) {
      return null
    }

    if (this.idleMotionIds.length === 1) {
      return this.bindings.get(this.idleMotionIds[0]) ?? null
    }

    let nextIndex = this.idleCursor
    while (nextIndex === this.idleCursor) {
      nextIndex = Math.floor(Math.random() * this.idleMotionIds.length)
    }
    this.idleCursor = nextIndex
    return this.bindings.get(this.idleMotionIds[nextIndex]) ?? null
  }

  private onMixerFinished(action: AnimationAction | null): void {
    if (!action) {
      return
    }

    if (this.currentOneShotBinding?.action === action) {
      const ended = this.currentOneShotBinding.entry
      // Stop and disable the finished action so its clamped last-frame
      // values won't blend with the next idle animation (the mixer
      // blends all enabled actions with weight > 0 on the same bone).
      action.stop()
      action.enabled = false
      this.currentOneShotBinding = null
      void this.emitMotionEnd(ended)
      this.updateState({
        activeMotionId: null,
        activeMotionKind: null,
        oneShotPlaying: false,
      })
      this.ensureIdlePlayback()
      return
    }

    if (this.currentIdleBinding?.action === action) {
      const ended = this.currentIdleBinding.entry
      action.stop()
      action.enabled = false
      this.currentIdleBinding = null
      void this.emitMotionEnd(ended)
      this.updateState({
        activeMotionId: null,
        activeMotionKind: null,
        idlePlaying: false,
      })
      this.ensureIdlePlayback()
    }
  }

  private syncPausedState(): void {
    for (const binding of this.bindings.values()) {
      binding.action.paused = !this.state.runtimePlaying
    }
  }

  private syncProceduralIdleSuppression(): void {
    const suppressed =
      this.state.idleEnabled && this.idleMotionIds.length > 0 && (this.currentIdleBinding !== null || this.currentOneShotBinding !== null)
    this.hooks.onProceduralIdleSuppressionChange?.(suppressed)
  }

  private updateState(patch: Partial<AvatarMotionState>): void {
    const next = { ...this.state, ...patch }
    const changed =
      next.activeMotionId !== this.state.activeMotionId ||
      next.activeMotionKind !== this.state.activeMotionKind ||
      next.idleEnabled !== this.state.idleEnabled ||
      next.idlePlaying !== this.state.idlePlaying ||
      next.oneShotPlaying !== this.state.oneShotPlaying ||
      next.runtimePlaying !== this.state.runtimePlaying

    this.state = next
    if (changed) {
      void this.hooks.onStateChange?.(this.getState())
    }
  }

  private emitMotionStart(entry: AvatarMotionEntry): Promise<void> {
    return Promise.resolve(this.hooks.onMotionStart?.(this.toEventPayload(entry))) as Promise<void>
  }

  private emitMotionEnd(entry: AvatarMotionEntry): Promise<void> {
    return Promise.resolve(this.hooks.onMotionEnd?.(this.toEventPayload(entry))) as Promise<void>
  }

  private toEventPayload(entry: AvatarMotionEntry): AvatarMotionEventPayload {
    return {
      motionId: entry.id,
      name: entry.name,
      kind: entry.kind,
      source: entry.source,
    }
  }

  private reportError(error: AvatarRuntimeErrorPayload): void {
    void this.hooks.onError?.(error)
  }
}

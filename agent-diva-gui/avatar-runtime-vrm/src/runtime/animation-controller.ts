import type { VRM } from '@pixiv/three-vrm'
import type { AvatarAnimationState, AvatarRuntimeErrorPayload } from '../protocol'
import { type AnimationClip, AnimationMixer, LoopRepeat } from 'three'
import { createBlinkClip } from './blink-animation'
import { createBreathClip } from './breath-animation'
import {
  type AnimationChannel,
  type AnimationChannelActionMap,
  DEFAULT_ANIMATION_STATE,
} from './animation-types'
import { applyNaturalPose, createProceduralIdleClip } from './procedural-idle'

interface AnimationControllerHooks {
  onStateChange?: (state: AvatarAnimationState) => void | Promise<void>
  onError?: (error: AvatarRuntimeErrorPayload) => void | Promise<void>
}

export class AnimationController {
  private vrm: VRM | null = null
  private mixer: AnimationMixer | null = null
  private state: AvatarAnimationState = { ...DEFAULT_ANIMATION_STATE }
  private readonly actionNames: AnimationChannelActionMap = {}
  private proceduralIdleSuppressed = false

  constructor(private readonly hooks: AnimationControllerHooks = {}) {}

  attach(vrm: VRM): void {
    this.detach()

    try {
      applyNaturalPose(vrm)
      this.vrm = vrm
      this.mixer = new AnimationMixer(vrm.scene)
      this.actionNames.idleEnabled = this.playAction('idleEnabled', createProceduralIdleClip(vrm))
      this.actionNames.breathEnabled = this.playAction('breathEnabled', createBreathClip(vrm))
      this.actionNames.blinkEnabled = this.playAction('blinkEnabled', createBlinkClip(vrm))
      this.state = {
        ...DEFAULT_ANIMATION_STATE,
        runtimePlaying: true,
      }
      this.emitState()
    } catch (error) {
      this.detach()
      void this.hooks.onError?.({
        code: 'ANIMATION_INIT_FAILED',
        message: error instanceof Error ? error.message : String(error),
        recoverable: true,
      })
    }
  }

  detach(): void {
    if (this.mixer) {
      this.mixer.stopAllAction()
      this.mixer.uncacheRoot(this.mixer.getRoot())
    }

    this.mixer = null
    this.vrm = null
    this.actionNames.idleEnabled = undefined
    this.actionNames.breathEnabled = undefined
    this.actionNames.blinkEnabled = undefined
    this.updateState({
      ...DEFAULT_ANIMATION_STATE,
      runtimePlaying: false,
    })
  }

  update(deltaSeconds: number): void {
    if (!this.state.runtimePlaying) {
      return
    }

    this.mixer?.update(deltaSeconds)
  }

  pause(): void {
    this.updateState({ runtimePlaying: false })
  }

  resume(): void {
    if (!this.vrm || !this.mixer) {
      return
    }

    this.updateState({ runtimePlaying: true })
  }

  setState(nextState: Partial<AvatarAnimationState>): AvatarAnimationState {
    const patch: Partial<AvatarAnimationState> = {}

    for (const key of ['idleEnabled', 'breathEnabled', 'blinkEnabled'] as AnimationChannel[]) {
      const requested = nextState[key]
      if (requested === undefined || requested === this.state[key]) {
        continue
      }

      this.setChannelEnabled(key, requested)
      patch[key] = requested
    }

    if (nextState.runtimePlaying !== undefined && nextState.runtimePlaying !== this.state.runtimePlaying) {
      if (nextState.runtimePlaying) {
        this.resume()
      } else {
        this.pause()
      }
      patch.runtimePlaying = nextState.runtimePlaying
    }

    if (Object.keys(patch).length > 0) {
      this.updateState(patch)
    }

    return this.getState()
  }

  getState(): AvatarAnimationState {
    return { ...this.state }
  }

  setProceduralIdleSuppressed(suppressed: boolean): void {
    if (this.proceduralIdleSuppressed === suppressed) {
      return
    }

    this.proceduralIdleSuppressed = suppressed
    this.syncChannelAction('idleEnabled')
  }

  private playAction(
    channel: AnimationChannel,
    clip: AnimationClip | null,
  ) {
    if (!clip || !this.mixer) {
      return undefined
    }

    const action = this.mixer.clipAction(clip)
    action.enabled = this.state[channel]
    action.setLoop(LoopRepeat, Infinity)
    action.play()
    action.paused = !this.state.runtimePlaying
    return action
  }

  private setChannelEnabled(channel: AnimationChannel, enabled: boolean): void {
    if (!this.mixer) {
      return
    }

    const actionName = this.actionNames[channel]
    if (!actionName) {
      return
    }

    const action = actionName
    const shouldPlay = this.isChannelEffectivelyEnabled(channel, enabled)
    action.enabled = shouldPlay
    action.setEffectiveWeight(shouldPlay ? 1 : 0)
    if (shouldPlay) {
      action.play()
    } else {
      action.stop()
      action.reset()
    }
  }

  private updateState(patch: Partial<AvatarAnimationState>): void {
    const next = { ...this.state, ...patch }
    const changed =
      next.idleEnabled !== this.state.idleEnabled ||
      next.breathEnabled !== this.state.breathEnabled ||
      next.blinkEnabled !== this.state.blinkEnabled ||
      next.runtimePlaying !== this.state.runtimePlaying

    this.state = next

    if (this.mixer) {
      for (const channel of ['idleEnabled', 'breathEnabled', 'blinkEnabled'] as AnimationChannel[]) {
        this.syncChannelAction(channel)
      }
    }

    if (changed) {
      this.emitState()
    }
  }

  private emitState(): void {
    void this.hooks.onStateChange?.(this.getState())
  }

  private isChannelEffectivelyEnabled(channel: AnimationChannel, requested = this.state[channel]): boolean {
    if (channel === 'idleEnabled' && this.proceduralIdleSuppressed) {
      return false
    }

    return requested
  }

  private syncChannelAction(channel: AnimationChannel): void {
    const action = this.actionNames[channel]
    if (!action) {
      return
    }

    const enabled = this.isChannelEffectivelyEnabled(channel)
    action.enabled = enabled
    action.setEffectiveWeight(enabled ? 1 : 0)
    action.paused = !this.state.runtimePlaying
    if (enabled) {
      action.play()
    } else {
      action.stop()
      action.reset()
    }
  }
}

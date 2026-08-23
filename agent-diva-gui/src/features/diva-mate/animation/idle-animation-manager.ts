/**
 * Idle Animation Manager — core of the Phase 2 VRMA Animation System.
 *
 * Manages the VRMA motion queue, idle animation loop, one-shot
 * animation triggers, and mode switching between VRMA, procedural,
 * and disabled modes.
 *
 * Reference: super-agent-party IdleAnimationManager class
 *            (vrm.js:536-896)
 *
 * Key adaptation: Uses avatar-runtime-vrm's playMotion/stopMotion
 *                 API instead of direct Three.js AnimationMixer access.
 *                 Animation completion is estimated via setTimeout
 *                 since the crate does not currently expose a
 *                 motion-finished event.
 */

import type { AvatarMotionState } from 'avatar-runtime-vrm'
import type { AnimationMode, IdleAnimationConfig } from './types'
import { DEFAULT_IDLE_ANIMATION_CONFIG, VrmAnimationError, VrmAnimationErrorCode } from './types'

// ── Runtime interface (minimal) ─────────────────────────────────

/**
 * Minimal VRM runtime interface used by the animation manager.
 *
 * Extracted as a structural interface to avoid depending on the full
 * RuntimeHandle type from avatar-runtime-vrm, which is defined
 * privately inside DivaVrmAvatar.vue.
 */
export interface VrmRuntimeHandle {
  playMotion(id: string): Promise<boolean> | boolean
  stopMotion(): Promise<boolean> | boolean
  setMotionState(state: Partial<AvatarMotionState>): Promise<void> | void
}

// ── Constants ───────────────────────────────────────────────────

/** Minimum pause between consecutive idle animations (ms) */
const MIN_PAUSE_BETWEEN_MS = 500

// ── IdleAnimationManager ────────────────────────────────────────

export class IdleAnimationManager {
  // ── State ─────────────────────────────────────────────────────

  private runtime: VrmRuntimeHandle | null = null
  private animationQueue: string[] = []  // Motion ID list
  private currentIndex = -1
  private isActive = false
  private currentMode: AnimationMode = 'none'
  private currentOneShotId: string | null = null

  private config: IdleAnimationConfig

  // Timer handles for cleanup
  private playbackTimerId: ReturnType<typeof setTimeout> | null = null
  private scheduleTimerId: ReturnType<typeof setTimeout> | null = null

  /** Callback invoked on errors (for logging) */
  private onError: ((error: VrmAnimationError, context: string) => void) | null = null

  // ── Constructor ───────────────────────────────────────────────

  constructor(config?: Partial<IdleAnimationConfig>) {
    this.config = { ...DEFAULT_IDLE_ANIMATION_CONFIG, ...config }
  }

  // ── Lifecycle ─────────────────────────────────────────────────

  /**
   * Attach a VRM runtime to this manager.
   * Must be called before any animation methods.
   */
  setRuntime(runtime: VrmRuntimeHandle | null): void {
    this.runtime = runtime
  }

  /**
   * Set the animation queue (motion IDs available for idle loop).
   * Replaces the existing queue and resets the index.
   */
  setAnimationQueue(motionIds: string[]): void {
    this.animationQueue = [...motionIds]
    this.currentIndex = -1
    console.log(
      `[IdleAnimationManager] Queue set: ${motionIds.length} motions`,
    )
  }

  /**
   * Set error callback for logging integration.
   */
  setErrorHandler(handler: (error: VrmAnimationError, context: string) => void): void {
    this.onError = handler
  }

  /** Check if the manager is currently active */
  get active(): boolean {
    return this.isActive
  }

  /** Get the current animation mode */
  get mode(): AnimationMode {
    return this.currentMode
  }

  /** Check if a one-shot animation is currently playing */
  get isOneShotPlaying(): boolean {
    return this.currentOneShotId !== null
  }

  // ── Mode control ──────────────────────────────────────────────

  /**
   * Start the idle animation loop.
   *
   * If VRMA animations are available in the queue, starts VRMA mode.
   * Otherwise falls back to procedural mode.
   */
  async startIdleLoop(): Promise<void> {
    if (!this.runtime) {
      this.signalError(VrmAnimationErrorCode.RUNTIME_UNAVAILABLE, 'startIdleLoop')
      return
    }

    if (this.isActive) {
      console.log('[IdleAnimationManager] Already active, restarting')
      this.stopAllAnimations()
    }

    if (this.animationQueue.length === 0) {
      console.log('[IdleAnimationManager] No VRMA available, falling back to procedural')
      this.switchToProceduralMode()
      return
    }

    this.switchToVRMAMode()
    console.log('[IdleAnimationManager] Idle loop started (VRMA mode)')
  }

  /**
   * Switch to VRMA mode (actual VRMA file playback).
   */
  private switchToVRMAMode(): void {
    if (this.currentMode === 'vrma' && this.isActive) {
      return
    }

    // Disable crate's built-in procedural idle since we manage it ourselves
    void this.runtime?.setMotionState({ idleEnabled: false })

    this.currentMode = 'vrma'
    this.isActive = true
    this.currentIndex = this.pickFirstIndex()
    void this.playNextVRMAAnimation()
  }

  /**
   * Switch to procedural mode (sine-wave based idle via ProceduralIdleGenerator).
   */
  private switchToProceduralMode(): void {
    if (this.currentMode === 'procedural' && this.isActive) {
      return
    }

    // Enable the crate's built-in idle motion
    void this.runtime?.setMotionState({ idleEnabled: true })

    this.currentMode = 'procedural'
    this.isActive = true
  }

  /**
   * Disable all animation (stop idle loop, stop one-shots).
   */
  stopAllAnimations(): void {
    this.isActive = false
    this.currentMode = 'none'
    this.currentOneShotId = null

    this.clearAllTimers()

    void this.runtime?.stopMotion()
    void this.runtime?.setMotionState({ idleEnabled: false })
  }

  // ── VRMA idle loop ────────────────────────────────────────────

  /** Pick the first animation index (random if shuffle enabled) */
  private pickFirstIndex(): number {
    if (this.animationQueue.length === 0) return -1
    return this.config.shuffle
      ? Math.floor(Math.random() * this.animationQueue.length)
      : 0
  }

  /** Pick the next animation index (random, not repeating previous) */
  private pickNextIndex(): number {
    const len = this.animationQueue.length
    if (len === 0) return -1
    if (len === 1) return 0

    let next: number
    do {
      next = Math.floor(Math.random() * len)
    } while (next === this.currentIndex && len > 1)

    return next
  }

  /** Play the next VRMA animation in the idle loop */
  private async playNextVRMAAnimation(): Promise<void> {
    if (!this.isActive || !this.runtime) return
    if (this.animationQueue.length === 0) {
      this.switchToProceduralMode()
      return
    }
    // Don't interrupt a one-shot
    if (this.currentOneShotId) return

    const motionId = this.animationQueue[this.currentIndex]
    if (!motionId) {
      this.currentIndex = this.pickFirstIndex()
      return
    }

    console.log(`[IdleAnimationManager] Playing idle: ${motionId}`)

    try {
      const ok = await this.runtime.playMotion(motionId)
      if (!ok) {
        this.signalError(
          VrmAnimationErrorCode.MOTION_NOT_FOUND,
          `playNextVRMAAnimation: ${motionId}`,
        )
        this.scheduleNextAnimation(MIN_PAUSE_BETWEEN_MS)
        return
      }

      // Immediately schedule the next animation after the estimated
      // duration.  Unlike super-agent-party which fills the 1.5 s gap
      // with a continuously-playing defaultPoseAction, agent-diva has
      // no baseline pose to cover the transition.  Chaining animations
      // directly avoids the visible dead pause.
      this.currentIndex = this.config.shuffle
        ? this.pickNextIndex()
        : (this.currentIndex + 1) % this.animationQueue.length

      this.playbackTimerId = setTimeout(() => {
        this.playbackTimerId = null
        if (this.currentMode === 'vrma' && this.isActive && !this.currentOneShotId) {
          void this.playNextVRMAAnimation()
        }
      }, this.config.estimatedDurationMs)

    } catch (error) {
      this.signalError(
        VrmAnimationErrorCode.MOTION_PLAYBACK_FAILED,
        `playNextVRMAAnimation: ${motionId}`,
      )
      this.scheduleNextAnimation(MIN_PAUSE_BETWEEN_MS)
    }
  }

  // ── One-shot animation ────────────────────────────────────────

  /**
   * Play a one-shot animation by motion ID.
   *
   * Interrupts the idle loop, plays the requested animation once,
   * then restores the idle state.
   *
   * @returns true if the motion was found and playback started
   */
  async playOneShotAnimation(motionId: string): Promise<boolean> {
    if (!this.runtime) {
      this.signalError(VrmAnimationErrorCode.RUNTIME_UNAVAILABLE, 'playOneShotAnimation')
      return false
    }

    // Block overlapping one-shots
    if (this.currentOneShotId) {
      console.log(`[IdleAnimationManager] One-shot already playing: ${this.currentOneShotId}, queuing ${motionId}`)
    }

    // Validate motion exists in queue
    if (!this.animationQueue.includes(motionId)) {
      this.signalError(VrmAnimationErrorCode.MOTION_NOT_FOUND, `playOneShotAnimation: ${motionId}`)
      return false
    }

    console.log(`[IdleAnimationManager] One-shot: ${motionId}`)

    // Clear pending idle timer but do NOT explicitly stopMotion().
    // An explicit stopMotion() creates a visible freeze frame between
    // the current animation ending and the one-shot starting.
    // The crate's playMotion() internally handles stopping the current
    // animation — matching super-agent-party's crossfade approach
    // where the old action fades out while the new one fades in.
    this.clearPlaybackTimer()

    this.currentOneShotId = motionId

    try {
      const ok = await this.runtime.playMotion(motionId)
      if (!ok) {
        this.currentOneShotId = null
        this.resetToIdle()
        return false
      }

      // Wait estimated duration then restore idle
      this.playbackTimerId = setTimeout(() => {
        this.playbackTimerId = null
        this.currentOneShotId = null
        this.resetToIdle()
      }, this.config.estimatedDurationMs)

      return true

    } catch (error) {
      this.currentOneShotId = null
      this.signalError(
        VrmAnimationErrorCode.MOTION_PLAYBACK_FAILED,
        `playOneShotAnimation: ${motionId}`,
      )
      this.resetToIdle()
      return false
    }
  }

  /**
   * Play a one-shot animation by display name (for AI semantic calling).
   * Matches against motion names in the queue (e.g., "问候" or "greeting").
   */
  async playOneShotByName(name: string): Promise<boolean> {
    // TODO: Motion name resolver integration
    // For now, try exact ID match
    return this.playOneShotAnimation(name)
  }

  // ── Internal helpers ──────────────────────────────────────────

  private resetToIdle(): void {
    if (!this.isActive) return
    if (this.currentMode === 'vrma' && this.animationQueue.length > 0) {
      void this.playNextVRMAAnimation()
    } else {
      this.switchToProceduralMode()
    }
  }

  private scheduleNextAnimation(delayMs: number): void {
    this.clearScheduleTimer()
    this.scheduleTimerId = setTimeout(() => {
      this.scheduleTimerId = null
      if (!this.currentOneShotId && this.currentMode === 'vrma' && this.isActive) {
        void this.playNextVRMAAnimation()
      }
    }, delayMs)
  }

  // ── Timer cleanup ─────────────────────────────────────────────

  private clearPlaybackTimer(): void {
    if (this.playbackTimerId !== null) {
      clearTimeout(this.playbackTimerId)
      this.playbackTimerId = null
    }
  }

  private clearScheduleTimer(): void {
    if (this.scheduleTimerId !== null) {
      clearTimeout(this.scheduleTimerId)
      this.scheduleTimerId = null
    }
  }

  private clearAllTimers(): void {
    this.clearPlaybackTimer()
    this.clearScheduleTimer()
  }

  // ── Error handling ────────────────────────────────────────────

  private signalError(code: VrmAnimationErrorCode, context: string): void {
    const message = `[${code}] ${context}`
    console.warn(`[IdleAnimationManager] ${message}`)
    this.onError?.(new VrmAnimationError(message, code), context)
  }

  // ── Destroy ───────────────────────────────────────────────────

  /** Fully destroy the animation manager and release all resources */
  destroy(): void {
    this.stopAllAnimations()
    this.runtime = null
    this.onError = null
  }
}

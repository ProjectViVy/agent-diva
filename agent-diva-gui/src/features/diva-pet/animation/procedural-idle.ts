/**
 * Procedural idle animation generator.
 *
 * Provides a fallback idle animation when no VRMA motion files are
 * available.  Produces sinusoidally-varying blend values for breath
 * and subtle body micro-movements.
 *
 * Reference: super-agent-party createIdleClip() (vrm.js:1096-1251)
 *            and createBreathClip() (vrm.js:1254-1280)
 *
 * Key adaptation: Instead of creating THREE.AnimationClip objects
 * (which require direct Three.js access), this generator outputs
 * procedural blend values that can be fed to the avatar-runtime-vrm
 * crate APIs (setMood / setSpeechState).
 */

import type { ProceduralIdleBlend, ProceduralIdleConfig } from './types'
import { DEFAULT_PROCEDURAL_IDLE_CONFIG } from './types'

export class ProceduralIdleGenerator {
  private config: ProceduralIdleConfig
  private startTime = 0
  private running = false
  private animationId: number | null = null
  private lastUpdate = 0

  /** Callback invoked with current blend values at each tick */
  private onUpdate: ((blend: ProceduralIdleBlend) => void) | null = null

  constructor(config?: Partial<ProceduralIdleConfig>) {
    this.config = { ...DEFAULT_PROCEDURAL_IDLE_CONFIG, ...config }
  }

  // ── Public API ─────────────────────────────────────────────────

  /**
   * Start the procedural idle loop.
   *
   * @param onUpdate  - Called with current blend values each tick
   */
  start(onUpdate: (blend: ProceduralIdleBlend) => void): void {
    if (this.running) {
      console.warn('[ProceduralIdle] Already running')
      return
    }

    if (!this.config.enabled) {
      console.log('[ProceduralIdle] Disabled, skipping start')
      return
    }

    this.running = true
    this.startTime = performance.now()
    this.lastUpdate = 0
    this.onUpdate = onUpdate
    this.scheduleNextTick()
  }

  /** Stop the procedural idle loop */
  stop(): void {
    this.running = false
    if (this.animationId !== null) {
      cancelAnimationFrame(this.animationId)
      this.animationId = null
    }
    this.onUpdate?.({
      breath: 0,
      microX: 0,
      microY: 0,
      microZ: 0,
    })
    this.onUpdate = null
  }

  /** Update configuration at runtime */
  setConfig(config: Partial<ProceduralIdleConfig>): void {
    this.config = { ...this.config, ...config }
  }

  /** Check if the generator is currently running */
  get isRunning(): boolean {
    return this.running
  }

  // ── Internal tick loop ─────────────────────────────────────────

  private scheduleNextTick(): void {
    this.animationId = requestAnimationFrame(() => this.tick())
  }

  private tick(): void {
    if (!this.running) return

    const now = performance.now()

    // Throttle updates to configured interval
    const elapsed = now - this.lastUpdate
    if (elapsed < this.config.updateIntervalMs) {
      this.scheduleNextTick()
      return
    }
    this.lastUpdate = now

    const elapsedTotal = (now - this.startTime) / 1000 // seconds

    this.onUpdate?.(this.computeBlend(elapsedTotal))
    this.scheduleNextTick()
  }

  // ── Blend computation ──────────────────────────────────────────

  /**
   * Compute the current procedural idle blend values.
   *
   * The algorithm uses three independent sine waves at different
   * frequencies to produce naturalistic, non-repeating motion:
   *
   * - Breath: slow sinusoid (period ~4s) modulating intensity
   * - Micro X: medium-fast sinusoid (period ~2.3s) for lateral sway
   * - Micro Y: fast sinusoid (period ~1.7s) for vertical bob
   * - Micro Z: medium sinusoid (period ~3.1s) for forward/back lean
   *
   * Frequencies are chosen to be relatively prime so the pattern
   * doesn't visibly repeat within a reasonable time window.
   *
   * Reference: vrm.js createIdleClip() uses similar layered sine
   *            approach on bone rotation tracks.
   */
  private computeBlend(elapsed: number): ProceduralIdleBlend {
    const { breathIntensity, microMovementIntensity } = this.config

    // Breath: slow ~0.25 Hz (4s period), mapped to [0.3, 1.0] for
    // visible but not extreme effect
    const breath =
      ((Math.sin(elapsed * Math.PI * 0.5) + 1) / 2) * 0.7 + 0.3
    const breathScaled = breath * breathIntensity

    // Micro-movements: faster frequencies create subtle body sway
    const microX =
      Math.sin(elapsed * Math.PI * 0.87) * microMovementIntensity
    const microY =
      Math.sin(elapsed * Math.PI * 1.17) * microMovementIntensity * 0.6
    const microZ =
      Math.sin(elapsed * Math.PI * 0.64) * microMovementIntensity * 0.4

    return {
      breath: breathScaled,
      microX,
      microY,
      microZ,
    }
  }

  // ── Cleanup ────────────────────────────────────────────────────

  /** Fully destroy the generator and release resources */
  destroy(): void {
    this.stop()
  }
}

/**
 * Look-at Controller — Phase 4 Expression & Gaze Enhancement.
 *
 * Calculates neck/head rotation angles to track a world-space target
 * with human-like angle limits and smooth interpolation.  The crate's
 * internal LookAtController handles the actual bone manipulation when
 * the VRM is loaded; this class provides the configuration and
 * monitoring layer on the TypeScript side.
 *
 * Reference: super-agent-party vrm.js:2153-2213
 *            avatar-runtime-vrm LookAtController class
 *
 * Algorithm:
 *   1. Compute view vector from model origin to target
 *   2. Decompose into yaw (horizontal) and pitch (vertical) angles
 *   3. Clamp to human-like limits (±45° yaw, 40° up / 20° down)
 *   4. Behind threshold (> 110°) resets to neutral
 *   5. Lerp smooth interpolation to avoid jerky transitions
 *
 * Integration:
 *   When the crate exposes a setLookAtTarget API, the composable will
 *   pass the computed target through.  Until then, this controller
 *   serves as a state management layer + future-proof API surface.
 */

import type { VrmLookAtTarget } from '../types'

// ── Configuration ────────────────────────────────────────────────

/** Configuration for look-at tracking behaviour */
export interface LookAtConfig {
    /** Maximum horizontal rotation (degrees, left/right) */
    yawLimitDeg: number
    /** Maximum upward rotation (degrees) */
    pitchUpLimitDeg: number
    /** Maximum downward rotation (degrees) */
    pitchDownLimitDeg: number
    /** Angle behind which tracking is disabled (degrees) */
    behindLimitDeg: number
    /** Lerp speed multiplier (higher = faster) */
    lerpSpeed: number
    /** Fraction of yaw/pitch applied to neck bone */
    neckRatio: number
    /** Fraction of yaw/pitch applied to head bone (additive on top of neck) */
    headRatio: number
}

/** Default look-at configuration — matches the crate's built-in defaults */
export const DEFAULT_LOOK_AT_CONFIG: LookAtConfig = {
    yawLimitDeg: 45,
    pitchUpLimitDeg: 40,
    pitchDownLimitDeg: 20,
    behindLimitDeg: 110,
    lerpSpeed: 3.0,
    neckRatio: 1.0,
    headRatio: 0.5,
}

// ── Runtime state output ─────────────────────────────────────────

/** Per-frame look-at state computed by update() */
export interface LookAtState {
    /** Current yaw angle in radians */
    yaw: number
    /** Current pitch angle in radians */
    pitch: number
    /** Whether tracking is active (target is within limits) */
    tracking: boolean
    /** The target position this frame */
    target: Readonly<VrmLookAtTarget>
}

// ── Private helpers ──────────────────────────────────────────────

function degToRad(deg: number): number {
    return (deg * Math.PI) / 180
}

function radToDeg(rad: number): number {
    return (rad * 180) / Math.PI
}

function clamp(value: number, min: number, max: number): number {
    return Math.max(min, Math.min(max, value))
}

function lerp(current: number, target: number, speed: number, deltaTime: number): number {
    const t = speed * deltaTime
    if (t >= 1) return target
    return current + (target - current) * t
}

// ── LookAtController ─────────────────────────────────────────────

export class LookAtController {
    // ── Configuration ────────────────────────────────────────────

    private config: LookAtConfig

    // ── State ────────────────────────────────────────────────────

    private enabled = false
    private target: VrmLookAtTarget = { x: 0, y: 1.5, z: 4 }
    private currentYaw = 0
    private currentPitch = 0
    private isTracking = false

    // ── Model origin (assumed to be at head-level) ───────────────

    private modelOrigin: VrmLookAtTarget = { x: 0, y: 1.5, z: 0 }

    constructor(config?: Partial<LookAtConfig>) {
        this.config = { ...DEFAULT_LOOK_AT_CONFIG, ...config }
    }

    // ── Public API ───────────────────────────────────────────────

    /** Enable look-at tracking with an optional target override */
    enable(target?: VrmLookAtTarget): void {
        this.enabled = true
        if (target) {
            this.target = { ...target }
        }
        // Reset interpolation state to avoid a jump
        this.currentYaw = 0
        this.currentPitch = 0
    }

    /** Disable look-at tracking and reset to neutral */
    disable(): void {
        this.enabled = false
        this.isTracking = false
        this.currentYaw = 0
        this.currentPitch = 0
    }

    /** Set the tracking target in world-space coordinates */
    setTarget(target: VrmLookAtTarget): void {
        this.target = { ...target }
    }

    /** Set the model origin (base neck position in world space) */
    setModelOrigin(origin: VrmLookAtTarget): void {
        this.modelOrigin = { ...origin }
    }

    /** Update configuration at runtime */
    setConfig(patch: Partial<LookAtConfig>): void {
        this.config = { ...this.config, ...patch }
    }

    /** Get the current configuration (read-only) */
    getConfig(): Readonly<LookAtConfig> {
        return this.config
    }

    /** @returns whether tracking is currently enabled */
    get isEnabled(): boolean {
        return this.enabled
    }

    /** @returns whether a target is currently being tracked */
    get tracking(): boolean {
        return this.isTracking
    }

    /**
     * Per-frame update.  Computes target yaw/pitch, applies limits,
     * and lerps toward the target.
     *
     * Call this once per frame (typically inside requestAnimationFrame).
     *
     * @param deltaTime  Seconds since last frame
     * @returns The computed look-at state for this frame
     */
    update(deltaTime: number): LookAtState {
        if (!this.enabled) {
            this.isTracking = false
            // Lerp back to neutral
            this.currentYaw = lerp(this.currentYaw, 0, this.config.lerpSpeed * 0.5, deltaTime)
            this.currentPitch = lerp(this.currentPitch, 0, this.config.lerpSpeed * 0.5, deltaTime)
            return {
                yaw: this.currentYaw,
                pitch: this.currentPitch,
                tracking: false,
                target: this.target,
            }
        }

        // Compute view vector from model origin to target
        const dx = this.target.x - this.modelOrigin.x
        const dy = this.target.y - this.modelOrigin.y
        const dz = this.target.z - this.modelOrigin.z

        // Horizontal distance (for pitch calculation and behind detection)
        const horizontalDist = Math.sqrt(dx * dx + dz * dz)

        if (horizontalDist < 0.001) {
            // Target is directly above/below — maintain current angles
            this.isTracking = false
            return {
                yaw: this.currentYaw,
                pitch: this.currentPitch,
                tracking: false,
                target: this.target,
            }
        }

        // Raw yaw: angle in the XZ plane
        const rawYaw = Math.atan2(dx, dz)

        // Raw pitch: vertical angle from horizontal
        const rawPitch = Math.atan2(dy, horizontalDist)

        // Apply 0.6 scaling (matches super-agent-party behavior)
        const scaledYaw = rawYaw * 0.6
        const scaledPitch = rawPitch * 0.6

        // Target yaw/pitch before clamping
        let targetYaw = scaledYaw
        let targetPitch = scaledPitch

        // Behind limit: if the raw yaw exceeds the behind threshold,
        // the model cannot see the target — reset to neutral.
        const behindLimitRad = degToRad(this.config.behindLimitDeg)
        if (Math.abs(rawYaw) > behindLimitRad) {
            targetYaw = 0
            targetPitch = 0
            this.isTracking = false
        } else {
            // Clamp to human-like limits
            const yawLimitRad = degToRad(this.config.yawLimitDeg)
            const pitchUpLimitRad = degToRad(this.config.pitchUpLimitDeg)
            const pitchDownLimitRad = degToRad(this.config.pitchDownLimitDeg)

            targetYaw = clamp(targetYaw, -yawLimitRad, yawLimitRad)
            targetPitch = clamp(targetPitch, -pitchDownLimitRad, pitchUpLimitRad)
            this.isTracking = true
        }

        // Smooth interpolation
        const speed = this.config.lerpSpeed
        this.currentYaw = lerp(this.currentYaw, targetYaw, speed, deltaTime)
        this.currentPitch = lerp(this.currentPitch, targetPitch, speed, deltaTime)

        return {
            yaw: this.currentYaw,
            pitch: this.currentPitch,
            tracking: this.isTracking,
            target: this.target,
        }
    }

    /**
     * Calculate neck and head quaternion components for the current
     * look-at state.  The neck gets the full rotation, and the head
     * gets an additive fraction.
     *
     * Returns yaw (Y-axis) and pitch (X-axis) pairs for both bones.
     * The consumer applies these as quaternion rotations on the
     * respective bone nodes.
     *
     * @returns Neck and head rotation angles in radians
     */
    getBoneRotations(): {
        neck: { yaw: number; pitch: number }
        head: { yaw: number; pitch: number }
    } {
        const neckYaw = this.currentYaw * this.config.neckRatio
        const neckPitch = this.currentPitch * this.config.neckRatio
        const headYaw = this.currentYaw * this.config.headRatio
        const headPitch = this.currentPitch * this.config.headRatio

        return {
            neck: { yaw: neckYaw, pitch: neckPitch },
            head: { yaw: headYaw, pitch: headPitch },
        }
    }

    /** Reset all internal state */
    reset(): void {
        this.currentYaw = 0
        this.currentPitch = 0
        this.isTracking = false
    }

    /** Get debug information */
    getDebugInfo(): {
        currentYawDeg: number
        currentPitchDeg: number
        target: VrmLookAtTarget
        tracking: boolean
        enabled: boolean
    } {
        return {
            currentYawDeg: radToDeg(this.currentYaw),
            currentPitchDeg: radToDeg(this.currentPitch),
            target: { ...this.target },
            tracking: this.isTracking,
            enabled: this.enabled,
        }
    }
}

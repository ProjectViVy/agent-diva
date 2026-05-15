/**
 * Animation system type definitions for the Phase 2 VRMA Animation System.
 *
 * Defines the core types for idle animation scheduling, VRMA motion
 * playback, procedural idle fallback, and the composable API surface.
 *
 * Adapted from super-agent-party's IdleAnimationManager and related constants.
 */

// ── Animation runtime mode ─────────────────────────────────────

/** Animation execution mode for the idle animation system */
export type AnimationMode = 'vrma' | 'procedural' | 'none'

// ── Motion data ─────────────────────────────────────────────────

/** Parsed VRMA motion animation data ready for playback */
export interface MotionAnimData {
  /** Unique motion identifier (matches VrmMotionInfo.id) */
  id: string
  /** Human-readable display name */
  name: string
  /** Raw VRMA binary data (ArrayBuffer for passing to avatar-runtime-vrm) */
  data?: ArrayBuffer
  /** Estimated animation duration in milliseconds */
  duration: number
}

/** Result of a motion load attempt */
export interface MotionLoadResult {
  /** The loaded motion data, or null if loading failed */
  motion: MotionAnimData | null
  /** Whether the load succeeded */
  success: boolean
  /** Error message if loading failed */
  error?: string
}

// ── Idle animation configuration ────────────────────────────────

/** Configuration for the IdleAnimationManager */
export interface IdleAnimationConfig {
  /** Whether idle animation is enabled */
  enabled: boolean
  /** Default estimated animation duration when actual duration is unavailable (ms) */
  estimatedDurationMs: number
  /** Whether to randomize the next animation (vs. sequential) */
  shuffle: boolean
}

/** Default idle animation configuration */
export const DEFAULT_IDLE_ANIMATION_CONFIG: IdleAnimationConfig = {
  enabled: true,
  estimatedDurationMs: 3000,
  shuffle: true,
}

// ── Procedural idle ─────────────────────────────────────────────

/** Configuration for the ProceduralIdleGenerator */
export interface ProceduralIdleConfig {
  /** Whether procedural idle is enabled (fallback when no VRMA files available) */
  enabled: boolean
  /** Breath animation intensity (0-1) */
  breathIntensity: number
  /** Micro-movement intensity (0-1) */
  microMovementIntensity: number
  /** Update interval in milliseconds */
  updateIntervalMs: number
}

/** Default procedural idle configuration */
export const DEFAULT_PROCEDURAL_IDLE_CONFIG: ProceduralIdleConfig = {
  enabled: true,
  breathIntensity: 0.5,
  microMovementIntensity: 0.3,
  updateIntervalMs: 1000 / 12, // 12fps for procedural updates
}

/** Output blend values from procedural idle calculation */
export interface ProceduralIdleBlend {
  /** Current breath blend value (0-1, based on sin wave) */
  breath: number
  /** Micro-movement X offset */
  microX: number
  /** Micro-movement Y offset */
  microY: number
  /** Micro-movement Z offset */
  microZ: number
}

// ── Error types ─────────────────────────────────────────────────

/** Error code enumeration for animation system failures */
export enum VrmAnimationErrorCode {
  /** VRMA animation format is not supported by the runtime */
  VRMA_NOT_SUPPORTED = 'VRMA_NOT_SUPPORTED',
  /** Requested motion ID was not found in the loaded queue */
  MOTION_NOT_FOUND = 'MOTION_NOT_FOUND',
  /** Motion playback was interrupted or failed */
  MOTION_PLAYBACK_FAILED = 'MOTION_PLAYBACK_FAILED',
  /** Procedural idle animation could not be activated */
  PROCEDURAL_IDLE_UNAVAILABLE = 'PROCEDURAL_IDLE_UNAVAILABLE',
  /** Runtime is not available (destroyed or not initialized) */
  RUNTIME_UNAVAILABLE = 'RUNTIME_UNAVAILABLE',
}

/** Structured error for animation system failures */
export class VrmAnimationError extends Error {
  constructor(
    message: string,
    public readonly code: VrmAnimationErrorCode,
  ) {
    super(message)
    this.name = 'VrmAnimationError'
  }
}

// ── Composable API surface ──────────────────────────────────────

/** Public API exposed by useVrmAnimation composable */
export interface VrmAnimationApi {
  /** The IdleAnimationManager instance (reactive) */
  idleManager: { value: import('./idle-animation-manager').IdleAnimationManager | null }

  /** Load VRMA animations from motion info list */
  loadAnimations: (motionList: import('../types').VrmMotionInfo[]) => Promise<MotionAnimData[]>

  /** Play a one-shot motion by ID (AI-triggered or user-triggered) */
  playMotion: (id: string) => Promise<boolean>

  /** Stop any currently playing motion */
  stopMotion: () => Promise<boolean>

  /** Enable or disable the idle animation system */
  setAnimationsEnabled: (enabled: boolean) => void

  /** Set the active motion pool (by ID list) */
  setMotionPool: (motionIds: string[]) => void

  /** Switch animation mode */
  setMode: (mode: AnimationMode) => void

  /** Fully destroy the animation manager and clean up */
  destroy: () => void
}

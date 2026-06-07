/**
 * Animation system barrel exports.
 *
 * Phase 2 — VRMA Animation System
 * Phase 3 — Lip Sync System
 * Phase 4 — Look-at & Gaze Tracking
 */

// Core classes
export { IdleAnimationManager } from './idle-animation-manager'
export type { VrmRuntimeHandle } from './idle-animation-manager'
export { ProceduralIdleGenerator } from './procedural-idle'

// Phase 3: Lip sync
export { LipSyncAnalyzer, DEFAULT_LIP_SYNC_CONFIG } from './lip-sync-analyzer'
export type { VowelBlends, LipSyncConfig } from './lip-sync-analyzer'
export { useLipSync } from './use-lip-sync'
export type { UseLipSyncOptions, UseLipSyncResult, LipSyncRuntimeHandle } from './use-lip-sync'

// Vue composable
export { useVrmAnimation, preloadVRMAAnimations } from './use-vrm-animation'
export type { UseVrmAnimationOptions, UseVrmAnimationResult } from './use-vrm-animation'

// Phase 4: Look-at tracking
export { LookAtController, DEFAULT_LOOK_AT_CONFIG } from './look-at-controller'
export type { LookAtConfig, LookAtState } from './look-at-controller'
export { useLookAt } from './use-look-at'
export type { UseLookAtOptions, UseLookAtResult } from './use-look-at'

// Types
export type {
  AnimationMode,
  MotionAnimData,
  MotionLoadResult,
  IdleAnimationConfig,
  ProceduralIdleConfig,
  ProceduralIdleBlend,
  VrmAnimationApi,
} from './types'
export {
  DEFAULT_IDLE_ANIMATION_CONFIG,
  DEFAULT_PROCEDURAL_IDLE_CONFIG,
  VrmAnimationError,
  VrmAnimationErrorCode,
} from './types'

export { default as DivaMateView } from './components/DivaMateView.vue'
export { default as DivaMateModelManager } from './components/DivaMateModelManager.vue'
export { default as DivaVrmAvatar } from './vrm/components/DivaVrmAvatar.vue'
export { default as DivaMateVoicePanel } from './voice/components/DivaMateVoicePanel.vue'
export { default as DesktopMateApp } from './components/DesktopMateApp.vue'
export { default as DesktopMateOverlay } from './components/DesktopMateOverlay.vue'
// ── Phase 5: UI panels ───────────────────────────────────────
export { default as VrmAnimationPanel } from './components/VrmAnimationPanel.vue'
export { default as VrmAppearancePanel } from './components/VrmAppearancePanel.vue'

export { useVoicePlayer } from './voice/composables/useVoicePlayer'
export { useVoiceInput } from './voice/composables/useVoiceInput'

export { ttsService } from './voice/services/tts-service'
export type {
  TTSVoiceConfig,
  TTSProvider,
  TTSRequest,
  TTSResponse,
  VoiceFileReader,
} from './voice/services/tts-service'

export { useMateConfig, getMateConfigSnapshot } from './services/mate-config'
export {
  useAppearanceConfig,
  generateAppearanceId,
  createEmptyAppearance,
} from './services/appearance-config'
export type { AppearanceConfigApi } from './services/appearance-config'

export { scanVRMAnimations, buildKnownMotionInfo, VRM_ANIMATIONS_DIR } from './utils/vrm-animation-scanner'
export { deriveMoodFromMessages, deriveMoodFromText, normalizeMood } from './utils/mood'

// ── Phase 2: Animation system exports ──────────────────────────
export { IdleAnimationManager } from './animation/idle-animation-manager'
export type { VrmRuntimeHandle } from './animation/idle-animation-manager'
export { ProceduralIdleGenerator } from './animation/procedural-idle'
export { useVrmAnimation, preloadVRMAAnimations } from './animation/use-vrm-animation'
export type { UseVrmAnimationOptions, UseVrmAnimationResult } from './animation/use-vrm-animation'
export type {
  AnimationMode,
  MotionAnimData,
  MotionLoadResult,
  IdleAnimationConfig,
  ProceduralIdleConfig,
  ProceduralIdleBlend,
} from './animation/types'
export {
  DEFAULT_IDLE_ANIMATION_CONFIG,
  DEFAULT_PROCEDURAL_IDLE_CONFIG,
  VrmAnimationError,
  VrmAnimationErrorCode,
} from './animation/types'

// ── Phase 4: Look-at & gaze tracking ─────────────────────────────
export { LookAtController, DEFAULT_LOOK_AT_CONFIG } from './animation/look-at-controller'
export type { LookAtConfig, LookAtState } from './animation/look-at-controller'
export { useLookAt } from './animation/use-look-at'
export type { UseLookAtOptions, UseLookAtResult } from './animation/use-look-at'

export type {
  MateConfig,
  MateMessage,
  VrmMood,
  VrmModelInfo,
  VrmLoadState,
  VrmMotionInfo,
  VrmAppearanceConfig,
  VrmLookAtTarget,
  GaussSceneId,
  GaussSceneEntry,
} from './types'
export { DEFAULT_MATE_CONFIG } from './types'

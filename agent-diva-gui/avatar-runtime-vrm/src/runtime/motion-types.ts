import type {
  AvatarMotionEntry,
  AvatarMotionEventPayload,
  AvatarMotionState,
  AvatarMotionStatePatch,
} from '../protocol'
import type { AnimationAction, AnimationClip } from 'three'

export const DEFAULT_MOTION_STATE: AvatarMotionState = {
  activeMotionId: null,
  activeMotionKind: null,
  idleEnabled: true,
  idlePlaying: false,
  oneShotPlaying: false,
  runtimePlaying: false,
}

export interface MotionClipBinding {
  entry: AvatarMotionEntry
  clip: AnimationClip
  action: AnimationAction
}

export interface MotionControllerHooks {
  onCatalogChange?: (catalog: AvatarMotionEntry[]) => void | Promise<void>
  onStateChange?: (state: AvatarMotionState) => void | Promise<void>
  onMotionStart?: (motion: AvatarMotionEventPayload) => void | Promise<void>
  onMotionEnd?: (motion: AvatarMotionEventPayload) => void | Promise<void>
  onError?: (error: {
    code: string
    message: string
    recoverable: boolean
    detail?: Record<string, string | number | boolean | null>
  }) => void | Promise<void>
  onProceduralIdleSuppressionChange?: (suppressed: boolean) => void
}

export type MotionStatePatch = AvatarMotionStatePatch

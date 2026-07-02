import type { AnimationAction } from 'three'
import type { AvatarAnimationState } from '../protocol'

export const DEFAULT_ANIMATION_STATE: AvatarAnimationState = {
  idleEnabled: true,
  breathEnabled: true,
  blinkEnabled: true,
  runtimePlaying: false,
}

export type AnimationChannel = Exclude<keyof AvatarAnimationState, 'runtimePlaying'>

export type AnimationChannelActionMap = Partial<Record<AnimationChannel, AnimationAction>>

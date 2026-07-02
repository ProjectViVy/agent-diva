import type {
  AvatarAnimationState,
  AvatarCharacterSpec,
  AvatarInitOptions,
  AvatarMotionStatePatch,
  AvatarMood,
  AvatarSpeechState,
  AvatarTransform,
  AvatarViewportSize,
  ChunkPlayPayload,
  SubtitlePayload,
} from './types'

export const AVATAR_COMMAND_NAMES = {
  init: 'avatar:init',
  loadCharacter: 'avatar:load-character',
  setMood: 'avatar:set-mood',
  setSpeechState: 'avatar:set-speech-state',
  setAnimationState: 'avatar:set-animation-state',
  setMotionState: 'avatar:set-motion-state',
  playMotion: 'avatar:play-motion',
  stopMotion: 'avatar:stop-motion',
  setTransform: 'avatar:set-transform',
  pause: 'avatar:pause',
  resume: 'avatar:resume',
  resize: 'avatar:resize',
  destroy: 'avatar:destroy',

  // Tier 3 commands
  setHoverAutoHide: 'avatar:set-hover-auto-hide',
  setPointerLock: 'avatar:set-pointerlock',
  chunkPlay: 'avatar:chunk-play',
  chunkStop: 'avatar:chunk-stop',
  chunkStopAll: 'avatar:chunk-stop-all',
  pttStart: 'avatar:ptt-start',
  pttStop: 'avatar:ptt-stop',
  subtitleUpdate: 'avatar:subtitle-update',
  subtitleClear: 'avatar:subtitle-clear',
} as const

export interface AvatarCommandMap {
  'avatar:init': AvatarInitOptions
  'avatar:load-character': AvatarCharacterSpec
  'avatar:set-mood': { mood: AvatarMood }
  'avatar:set-speech-state': AvatarSpeechState
  'avatar:set-animation-state': Partial<AvatarAnimationState>
  'avatar:set-motion-state': AvatarMotionStatePatch
  'avatar:play-motion': { motionId: string }
  'avatar:stop-motion': undefined
  'avatar:set-transform': Partial<AvatarTransform>
  'avatar:pause': undefined
  'avatar:resume': undefined
  'avatar:resize': AvatarViewportSize
  'avatar:destroy': undefined

  // Tier 3 commands
  'avatar:set-hover-auto-hide': { enabled: boolean }
  'avatar:set-pointerlock': { locked: boolean }
  'avatar:chunk-play': ChunkPlayPayload
  'avatar:chunk-stop': { chunkId: string }
  'avatar:chunk-stop-all': undefined
  'avatar:ptt-start': undefined
  'avatar:ptt-stop': undefined
  'avatar:subtitle-update': SubtitlePayload
  'avatar:subtitle-clear': undefined
}

export type AvatarCommandName = keyof AvatarCommandMap

export type AvatarCommandPayload<TName extends AvatarCommandName> = AvatarCommandMap[TName]

export interface AvatarCommandEnvelope<TName extends AvatarCommandName = AvatarCommandName> {
  name: TName
  payload: AvatarCommandMap[TName]
}

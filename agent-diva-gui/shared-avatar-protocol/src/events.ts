import type {
  AvatarAnimationState,
  AvatarMotionEntry,
  AvatarMotionEventPayload,
  AvatarMotionState,
  AvatarRuntimeErrorPayload,
  AvatarRuntimeMetrics,
  AvatarTransform,
  ChunkEventPayload,
  HoverAutoHideState,
  PointerLockState,
  PttAudioReadyPayload,
  PttState,
  PttTranscriptionPayload,
  SubtitlePayload,
  VmcFramePayload,
} from './types'

export const AVATAR_EVENT_NAMES = {
  ready: 'avatar:ready',
  loadStart: 'avatar:load-start',
  loadSuccess: 'avatar:load-success',
  loadError: 'avatar:load-error',
  transformChange: 'avatar:transform-change',
  interactionStart: 'avatar:interaction-start',
  interactionEnd: 'avatar:interaction-end',
  speechStart: 'avatar:speech-start',
  speechEnd: 'avatar:speech-end',
  animationStateChange: 'avatar:animation-state-change',
  animationError: 'avatar:animation-error',
  motionCatalogChange: 'avatar:motion-catalog-change',
  motionStateChange: 'avatar:motion-state-change',
  motionStart: 'avatar:motion-start',
  motionEnd: 'avatar:motion-end',
  motionError: 'avatar:motion-error',
  metrics: 'avatar:metrics',
  runtimeError: 'avatar:runtime-error',
  vmcFrame: 'avatar:vmc-frame',

  // Tier 3 events
  hoverStateChange: 'avatar:hover-state-change',
  pointerlockChange: 'avatar:pointerlock-change',
  chunkStart: 'avatar:chunk-start',
  chunkEnd: 'avatar:chunk-end',
  chunkAllEnd: 'avatar:chunk-all-end',
  pttStateChange: 'avatar:ptt-state-change',
  pttAudioReady: 'avatar:ptt-audio-ready',
  pttTranscription: 'avatar:ptt-transcription',
  subtitleChange: 'avatar:subtitle-change',
} as const

export interface AvatarEventMap {
  'avatar:ready': { runtimeVersion?: string; capabilities?: string[] }
  'avatar:load-start': { characterId: string }
  'avatar:load-success': { characterId: string }
  'avatar:load-error': { characterId?: string; error: AvatarRuntimeErrorPayload }
  'avatar:transform-change': AvatarTransform
  'avatar:interaction-start': { kind: 'drag' | 'rotate' | 'scale' | 'custom' }
  'avatar:interaction-end': { kind: 'drag' | 'rotate' | 'scale' | 'custom' }
  'avatar:speech-start': { viseme?: string | null }
  'avatar:speech-end': undefined
  'avatar:animation-state-change': AvatarAnimationState
  'avatar:animation-error': AvatarRuntimeErrorPayload
  'avatar:motion-catalog-change': AvatarMotionEntry[]
  'avatar:motion-state-change': AvatarMotionState
  'avatar:motion-start': AvatarMotionEventPayload
  'avatar:motion-end': AvatarMotionEventPayload
  'avatar:motion-error': AvatarRuntimeErrorPayload
  'avatar:metrics': AvatarRuntimeMetrics
  'avatar:runtime-error': AvatarRuntimeErrorPayload
  'avatar:vmc-frame': VmcFramePayload

  // Tier 3 events
  'avatar:hover-state-change': HoverAutoHideState
  'avatar:pointerlock-change': PointerLockState
  'avatar:chunk-start': ChunkEventPayload
  'avatar:chunk-end': ChunkEventPayload
  'avatar:chunk-all-end': undefined
  'avatar:ptt-state-change': PttState
  'avatar:ptt-audio-ready': PttAudioReadyPayload
  'avatar:ptt-transcription': PttTranscriptionPayload
  'avatar:subtitle-change': SubtitlePayload
}

export type AvatarEventName = keyof AvatarEventMap

export type AvatarEventPayload<TName extends AvatarEventName> = AvatarEventMap[TName]

export interface AvatarEventEnvelope<TName extends AvatarEventName = AvatarEventName> {
  name: TName
  payload: AvatarEventMap[TName]
}

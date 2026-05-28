export type AvatarRuntimeKind = 'vrm' | 'live2d'

export type AvatarRuntimeMode = 'embedded' | 'desktop-pet'

export type AvatarMood = 'neutral' | 'happy' | 'sad' | 'angry' | 'surprised'

export type AvatarViseme =
  | 'aa'
  | 'ih'
  | 'ou'
  | 'ee'
  | 'oh'
  | 'blink'
  | 'silence'
  | 'custom'

export interface AvatarSpeechState {
  speaking: boolean
  viseme?: AvatarViseme | string | null
  intensity?: number
}

export interface AvatarAnimationState {
  idleEnabled: boolean
  breathEnabled: boolean
  blinkEnabled: boolean
  runtimePlaying: boolean
}

export type AvatarMotionKind = 'idle' | 'oneshot'

export interface AvatarMotionEntry {
  id: string
  name: string
  kind: AvatarMotionKind
  source: string
}

export interface AvatarMotionState {
  activeMotionId: string | null
  activeMotionKind: AvatarMotionKind | null
  idleEnabled: boolean
  selectedIdleMotionIds: string[]
  idlePlaying: boolean
  oneShotPlaying: boolean
  runtimePlaying: boolean
}

export interface AvatarMotionStatePatch {
  idleEnabled?: boolean
  selectedIdleMotionIds?: string[]
}

export interface AvatarMotionEventPayload {
  motionId: string
  name: string
  kind: AvatarMotionKind
  source: string
}

export interface AvatarTransform {
  scale: number
  offsetX: number
  offsetY: number
  rotationAzimuth: number
  rotationPolar: number
}

export interface AvatarCharacterSpec {
  id: string
  kind: AvatarRuntimeKind
  modelSource: string
  displayName?: string
  initialMood?: AvatarMood
  metadata?: Record<string, string | number | boolean | null>
}

export interface AvatarInitOptions {
  mode: AvatarRuntimeMode
  transparent: boolean
  allowInteraction: boolean
  backgroundColor?: string | null
  maxFps?: number | null
}

export interface AvatarViewportSize {
  width: number
  height: number
}

export interface AvatarRuntimeMetrics {
  fps: number
  frameTimeMs: number
  memoryMb?: number
}

export interface AvatarRuntimeErrorPayload {
  code: string
  message: string
  recoverable: boolean
  detail?: Record<string, string | number | boolean | null>
}

// ─── VMC Protocol Types ──────────────────────────────────────────

export interface VmcBoneDatum {
  name: string
  pos: { x: number; y: number; z: number }
  rot: { x: number; y: number; z: number; w: number }
}

export interface VmcBlendDatum {
  name: string
  weight: number
}

export interface VmcFramePayload {
  bones: VmcBoneDatum[]
  blends: VmcBlendDatum[]
  timestamp: number
}

// ─── Tier 3: Hover Auto-Hide ──────────────────────────────────

export interface HoverAutoHideState {
  /** Whether auto-hide feature is enabled */
  enabled: boolean
  /** Whether the model is currently hidden due to hover */
  hidden: boolean
}

// ─── Tier 3: PointerLock ──────────────────────────────────────

export interface PointerLockState {
  /** Whether first-person PointerLock controls are active */
  locked: boolean
}

// ─── Tier 3: Chunk Animation ──────────────────────────────────

export interface ChunkPlayPayload {
  /** Unique identifier for this audio chunk */
  chunkId: string
  /** Base64-encoded audio data URL */
  audioDataUrl: string
  /** MIME type of the audio (e.g., 'audio/wav', 'audio/webm') */
  mimeType?: string
  /** Optional expression name to apply during playback */
  expression?: string
}

export interface ChunkEventPayload {
  /** Identifier of the chunk that started/ended */
  chunkId: string
  /** Expression that was active when the chunk played */
  expression?: string
}

// ─── Tier 3: Push-to-Talk ─────────────────────────────────────

export interface PttState {
  /** Whether PTT recording is currently active */
  recording: boolean
}

export interface PttAudioReadyPayload {
  /** Audio format */
  format: 'wav'
  /** Base64-encoded audio data URL */
  audioDataUrl: string
}

export interface PttTranscriptionPayload {
  /** Transcribed text from speech recognition */
  text: string
  /** Whether this is the final transcription or an interim result */
  isFinal: boolean
}

// ─── Tier 3: Subtitle ─────────────────────────────────────────

export interface SubtitlePayload {
  /** Subtitle text to display */
  text: string
  /** Optional chunk index for tracking */
  chunkIndex?: number
}

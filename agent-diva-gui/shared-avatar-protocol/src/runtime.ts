import type {
  AvatarCommandMap,
  AvatarCommandName,
  AvatarCommandPayload,
} from './commands'
import type {
  AvatarEventName,
  AvatarEventPayload,
} from './events'
import type {
  AvatarAnimationState,
  AvatarCharacterSpec,
  AvatarInitOptions,
  AvatarMotionEntry,
  AvatarMotionState,
  AvatarMotionStatePatch,
  AvatarMood,
  AvatarSpeechState,
  AvatarTransform,
  AvatarViewportSize,
} from './types'

export type AvatarEventHandler<TName extends AvatarEventName> = (
  payload: AvatarEventPayload<TName>,
) => void | Promise<void>

export interface AvatarRuntimeEventEmitter {
  emit<TName extends AvatarEventName>(
    name: TName,
    payload: AvatarEventPayload<TName>,
  ): void | Promise<void>
}

export interface AvatarRuntimeCommandDispatcher {
  dispatch<TName extends AvatarCommandName>(
    name: TName,
    payload: AvatarCommandPayload<TName>,
  ): void | Promise<void>
}

export interface AvatarRuntimeHostBridge
  extends AvatarRuntimeEventEmitter, AvatarRuntimeCommandDispatcher {
  on<TName extends AvatarEventName>(
    name: TName,
    handler: AvatarEventHandler<TName>,
  ): () => void
}

export interface AvatarRuntime {
  init(options: AvatarInitOptions): Promise<void>
  loadCharacter(spec: AvatarCharacterSpec): Promise<void>
  setMood(mood: AvatarMood): void | Promise<void>
  setSpeechState(state: AvatarSpeechState): void | Promise<void>
  setAnimationState(state: Partial<AvatarAnimationState>): void | Promise<void>
  getAnimationState(): AvatarAnimationState
  getMotionCatalog(): AvatarMotionEntry[]
  getMotionState(): AvatarMotionState
  setMotionState(state: AvatarMotionStatePatch): void | Promise<void>
  playMotion(id: string): boolean | Promise<boolean>
  stopMotion(): boolean | Promise<boolean>
  setTransform(transform: Partial<AvatarTransform>): void | Promise<void>
  getTransform(): AvatarTransform
  pause(): void | Promise<void>
  resume(): void | Promise<void>
  resize(size: AvatarViewportSize): void | Promise<void>
  destroy(): Promise<void>
}

export type AvatarRuntimeCommandHandlers = {
  [TName in AvatarCommandName]: (
    payload: AvatarCommandMap[TName],
  ) => void | Promise<void>
}

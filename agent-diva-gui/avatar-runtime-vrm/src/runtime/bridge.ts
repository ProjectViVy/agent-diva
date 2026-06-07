import type {
  AvatarCommandName,
  AvatarCommandPayload,
  AvatarEventHandler,
  AvatarEventName,
  AvatarEventPayload,
  AvatarRuntimeCommandHandlers,
  AvatarRuntimeHostBridge,
} from '../protocol'

type EventHandlerMap = {
  [TName in AvatarEventName]?: Set<AvatarEventHandler<TName>>
}

type CommandHandlerMap = Partial<AvatarRuntimeCommandHandlers>

export class RuntimeBridge implements AvatarRuntimeHostBridge {
  private readonly eventHandlers: EventHandlerMap = {}
  private commandHandlers: CommandHandlerMap = {}

  // ─── Event API (runtime → host) ────────────────────────────────

  on<TName extends AvatarEventName>(
    name: TName,
    handler: AvatarEventHandler<TName>,
  ): () => void {
    const current = this.eventHandlers[name] as Set<AvatarEventHandler<TName>> | undefined
    if (current) {
      current.add(handler)
    } else {
      this.eventHandlers[name] = new Set([handler]) as EventHandlerMap[TName]
    }

    return () => {
      ;(this.eventHandlers[name] as Set<AvatarEventHandler<TName>> | undefined)?.delete(handler)
    }
  }

  async emit<TName extends AvatarEventName>(
    name: TName,
    payload: AvatarEventPayload<TName>,
  ): Promise<void> {
    const handlers = this.eventHandlers[name] as Set<AvatarEventHandler<TName>> | undefined
    if (!handlers) {
      return
    }

    for (const handler of handlers) {
      await handler(payload)
    }
  }

  // ─── Command API (host → runtime) ──────────────────────────────

  /**
   * Register handlers for incoming host commands.
   * Called once by VrmRuntime during init to wire all command names
   * to their corresponding runtime methods.
   */
  setCommandHandlers(handlers: CommandHandlerMap): void {
    this.commandHandlers = { ...handlers }
  }

  async dispatch<TName extends AvatarCommandName>(
    name: TName,
    payload: AvatarCommandPayload<TName>,
  ): Promise<void> {
    const handler = this.commandHandlers[name]
    if (!handler) {
      throw new Error(`No command handler registered for "${name}"`)
    }

    await handler(payload)
  }
}

/** UI chat message model shared by App/ChatView/NormalMode projections. */
export interface ChatMessage {
  id: string
  role: 'user' | 'agent' | 'system' | 'tool'
  content: string
  reasoning?: string
  isThinking?: boolean
  isStreaming?: boolean
  timestamp?: number
  emotion?: string
  toolName?: string
  toolArgs?: string
  toolResult?: string
  toolStatus?: 'running' | 'success' | 'error'
  toolCallId?: string
  retryStatus?: { attempt: number; maxRetries: number; model?: string }
  stalled?: boolean
  rawMeta?: Record<string, unknown>
  fromHistory?: boolean
  attachments?: string[]
  /** Owning run for run-folded segments (DN-6C replay fencing). */
  runId?: string
}

export function generateMessageId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return `msg-${crypto.randomUUID()}`
  }
  return `msg-${Date.now()}-${Math.floor(Math.random() * 100000)}`
}

/** Index of the last agent message still streaming, or -1. */
export function lastStreamingAgentIndex(messages: ChatMessage[]): number {
  for (let i = messages.length - 1; i >= 0; i -= 1) {
    if (messages[i].role === 'agent' && messages[i].isStreaming) return i
  }
  return -1
}

/**
 * Pure reducer: an ordered (deduped, gap-free) run/event stream → UI
 * ChatMessage segment. Mirrors the legacy agent-* listeners' behavior:
 * model deltas append to a streaming agent message, tool cards open on
 * tool.started and settle on tool.finished, terminals finalize.
 *
 * Wire types verified against the DN-0 fixture transcript and the pinned
 * VIVY `internal/domain/event.go` + `internal/runtime/payloads.go`.
 */
import type { RunEvent, SessionMessage } from '../api/vivy/contracts'
import { generateMessageId, lastStreamingAgentIndex, type ChatMessage } from './chat-message'

const TERMINAL_TYPES = new Set(['run.completed', 'run.failed', 'run.cancelled'])

export function isTerminalRunEvent(type: string): boolean {
  return TERMINAL_TYPES.has(type)
}

function ensureStreamingAgent(messages: ChatMessage[], timestamp?: number): ChatMessage {
  const idx = lastStreamingAgentIndex(messages)
  if (idx !== -1) return messages[idx]
  const msg: ChatMessage = {
    id: generateMessageId(),
    role: 'agent',
    content: '',
    isStreaming: true,
    timestamp: timestamp ?? Date.now(),
    emotion: 'normal',
  }
  messages.push(msg)
  return msg
}

function findRunningToolIndex(messages: ChatMessage[], toolCallId?: string): number {
  if (toolCallId) {
    const exact = messages.findIndex(
      (m) => m.role === 'tool' && m.toolStatus === 'running' && m.toolCallId === toolCallId,
    )
    if (exact !== -1) return exact
  }
  for (let i = messages.length - 1; i >= 0; i -= 1) {
    if (messages[i].role === 'tool' && messages[i].toolStatus === 'running') return i
  }
  return -1
}

function argsPreview(args: unknown): string {
  if (args === undefined || args === null) return ''
  if (typeof args === 'string') return args
  try {
    return JSON.stringify(args, null, 2)
  } catch {
    return String(args)
  }
}

function finalizeStreaming(messages: ChatMessage[], finalContent?: string): void {
  const idx = lastStreamingAgentIndex(messages)
  if (idx === -1) return
  const msg = messages[idx]
  if (finalContent && !msg.content) msg.content = finalContent
  msg.isStreaming = false
  msg.isThinking = false
}

function dropEmptyStreamingPlaceholder(messages: ChatMessage[]): void {
  const idx = lastStreamingAgentIndex(messages)
  if (idx === -1) return
  const msg = messages[idx]
  if (!msg.content && !msg.reasoning) messages.splice(idx, 1)
}

/**
 * Fold ordered run events into a message segment. Called with the run's
 * contiguous applied events; callers rebuild the segment from scratch on
 * snapshot resync, so this function must be deterministic and pure.
 */
export function reduceRunMessages(events: RunEvent[], opts: { now?: () => number } = {}): ChatMessage[] {
  const now = opts.now ?? (() => Date.now())
  const messages: ChatMessage[] = []
  const pendingToolArgs = new Map<string, { name: string; args: string }>()

  for (const event of events) {
    const p = (event.payload ?? {}) as Record<string, unknown>
    switch (event.type) {
      case 'run.started':
      case 'model.request':
      case 'model.usage':
      case 'policy.evaluated':
      case 'tool.operation':
      case 'tool.mounted':
      case 'hook.started':
      case 'hook.completed':
        break

      case 'model.delta': {
        const agent = ensureStreamingAgent(messages, event.created_at)
        agent.content += typeof p.delta === 'string' ? p.delta : ''
        break
      }

      case 'model.reasoning_delta': {
        const agent = ensureStreamingAgent(messages, event.created_at)
        agent.reasoning = (agent.reasoning ?? '') + (typeof p.delta === 'string' ? p.delta : '')
        agent.isThinking = true
        break
      }

      case 'provider.retry': {
        const agent = ensureStreamingAgent(messages, event.created_at)
        agent.retryStatus = {
          attempt: typeof p.attempt === 'number' ? p.attempt : 0,
          maxRetries: typeof p.max_retries === 'number' ? p.max_retries : 0,
          model: typeof p.model === 'string' ? p.model : undefined,
        }
        break
      }

      case 'provider.stall': {
        const agent = ensureStreamingAgent(messages, event.created_at)
        agent.stalled = true
        break
      }

      case 'tool.requested': {
        const callId = typeof p.tool_call_id === 'string' ? p.tool_call_id : ''
        if (callId) {
          pendingToolArgs.set(callId, {
            name: typeof p.tool_name === 'string' ? p.tool_name : '',
            args: argsPreview(p.args),
          })
        }
        break
      }

      case 'tool.started': {
        const callId = typeof p.tool_call_id === 'string' ? p.tool_call_id : undefined
        const stashed = callId ? pendingToolArgs.get(callId) : undefined
        // A tool card interrupts the current streaming placeholder.
        dropEmptyStreamingPlaceholder(messages)
        messages.push({
          id: generateMessageId(),
          role: 'tool',
          content: 'tool running',
          timestamp: event.created_at ?? now(),
          toolName: (typeof p.tool_name === 'string' && p.tool_name) || stashed?.name || 'unknown tool',
          toolArgs: stashed?.args,
          toolStatus: 'running',
          toolCallId: callId,
        })
        break
      }

      case 'tool.finished': {
        const callId = typeof p.tool_call_id === 'string' ? p.tool_call_id : undefined
        const result = typeof p.result === 'string' ? p.result : ''
        const isError = p.is_error === true || /^error\b/i.test(result)
        const idx = findRunningToolIndex(messages, callId)
        if (idx !== -1) {
          const tool = messages[idx]
          tool.toolStatus = isError ? 'error' : 'success'
          tool.content = isError ? 'tool error' : 'tool success'
          if (typeof p.tool_name === 'string' && p.tool_name) tool.toolName = p.tool_name
          tool.toolResult = result
        } else {
          messages.push({
            id: generateMessageId(),
            role: 'tool',
            content: isError ? 'tool error' : 'tool success',
            timestamp: event.created_at ?? now(),
            toolName: typeof p.tool_name === 'string' ? p.tool_name : 'unknown tool',
            toolResult: result,
            toolStatus: isError ? 'error' : 'success',
            toolCallId: callId,
          })
        }
        // After a tool settles, the model continues into a fresh placeholder.
        ensureStreamingAgent(messages, event.created_at)
        break
      }

      case 'tool.approval_required':
      case 'tool.approval_decided':
      case 'tool.approval_expired':
      case 'tool.approval_cancelled':
      case 'user.question_required':
      case 'user.question_answered':
      case 'user.question_cancelled':
      case 'user.question_expired':
      case 'context.compacted':
        // Tracked by the controller (pending lists / compaction status),
        // not by the message stream.
        break

      case 'model.completed': {
        const idx = lastStreamingAgentIndex(messages)
        if (idx !== -1) messages[idx].isThinking = false
        break
      }

      case 'run.completed': {
        const summary = typeof p.summary === 'string' ? p.summary : ''
        finalizeStreaming(messages, summary || undefined)
        if (lastStreamingAgentIndex(messages) === -1 && summary && messages.every((m) => m.content !== summary)) {
          // Terminal-only answer: no deltas arrived at all.
          if (!messages.some((m) => m.role === 'agent' && m.content === summary)) {
            messages.push({
              id: generateMessageId(),
              role: 'agent',
              content: summary,
              timestamp: event.created_at ?? now(),
              emotion: 'normal',
            })
          }
        }
        break
      }

      case 'run.cancelled': {
        dropEmptyStreamingPlaceholder(messages)
        finalizeStreaming(messages)
        break
      }

      case 'run.failed': {
        dropEmptyStreamingPlaceholder(messages)
        finalizeStreaming(messages)
        const error =
          (typeof p.error === 'string' && p.error) ||
          (typeof p.message === 'string' && p.message) ||
          'run failed'
        messages.push({
          id: generateMessageId(),
          role: 'system',
          content: `Error: ${error}`,
          timestamp: event.created_at ?? now(),
        })
        break
      }

      default:
        break
    }
  }
  return messages
}

/**
 * Map `session/messages` history into UI messages. Verified against the
 * fixture: roles user/assistant/tool; assistant rows may carry a
 * tool_name + tool_call_id marker with empty content (skip — the tool
 * row carries the output).
 */
export function mapHistoryMessages(sessionMessages: SessionMessage[]): ChatMessage[] {
  const toolNames = new Map<string, string>()
  for (const msg of sessionMessages) {
    if (msg.role === 'assistant' && typeof msg.tool_name === 'string' && typeof msg.tool_call_id === 'string') {
      toolNames.set(msg.tool_call_id, msg.tool_name)
    }
  }
  const out: ChatMessage[] = []
  for (const msg of sessionMessages) {
    const toolCallId = typeof msg.tool_call_id === 'string' ? msg.tool_call_id : undefined
    switch (msg.role) {
      case 'user':
        out.push({
          id: msg.id || generateMessageId(),
          role: 'user',
          content: msg.content || '',
          timestamp: msg.created_at,
          fromHistory: true,
        })
        break
      case 'assistant':
        if (!msg.content && msg.tool_name) break // tool-call marker row
        out.push({
          id: msg.id || generateMessageId(),
          role: 'agent',
          content: msg.content || '',
          timestamp: msg.created_at,
          emotion: 'normal',
          toolCallId,
          fromHistory: true,
        })
        break
      case 'tool':
        out.push({
          id: msg.id || generateMessageId(),
          role: 'tool',
          content: /^error\b/i.test(msg.content || '') ? 'tool error' : 'tool success',
          timestamp: msg.created_at,
          toolName: (typeof msg.tool_name === 'string' && msg.tool_name) || (toolCallId ? toolNames.get(toolCallId) : undefined) || 'unknown tool',
          toolResult: msg.content || '',
          toolStatus: /^error\b/i.test(msg.content || '') ? 'error' : 'success',
          toolCallId,
          fromHistory: true,
        })
        break
      default:
        break
    }
  }
  return out
}

import type {
  ChannelEnvelopeV1,
  StreamPhase,
} from '../../protocol/neuro-link-v1'
import type { ProjectionEnvelopeEvent, ProjectionSource } from './client'

export type NeuroLinkProjectionMethod =
  | 'turn/admission'
  | 'turn/iteration'
  | 'conversation/stream'
  | 'conversation/reasoning'
  | 'tool/lifecycle'
  | 'approval/required'
  | 'question/required'
  | 'planning/changed'
  | 'provider/status'
  | 'context/compaction'
  | 'presentation/event'
  | 'state/gap'
  | 'service/changed'

export interface NeuroLinkToolProjection {
  callId: string
  name?: string
  argsPreview?: string
  status: 'running' | 'success' | 'error'
  isError?: boolean
}

export interface NeuroLinkTurnProjection {
  sessionKey: string
  requestId: string
  traceId?: string
  content: string
  reasoning: string
  phase?: StreamPhase
  admission?: Record<string, unknown>
  tools: Record<string, NeuroLinkToolProjection>
  planning?: Record<string, unknown>
  provider?: Record<string, unknown>
  compaction?: Record<string, unknown>
  error?: string
}

export interface NeuroLinkPresentationState {
  thinking: boolean
  speaking: boolean
  tools: Record<string, NeuroLinkToolProjection>
  waitingForApproval: Record<string, unknown> | null
  subtitle: string
  expressionHint: string | null
  lastEvent: string | null
  shouldSpeak: boolean
}

export interface NeuroLinkProjectionState {
  sessionKey: string | null
  turns: Record<string, NeuroLinkTurnProjection>
  presentation: NeuroLinkPresentationState
  lastCursor: { stream: string; sequence: number } | null
}

export function createNeuroLinkProjectionState(sessionKey: string | null = null): NeuroLinkProjectionState {
  return {
    sessionKey,
    turns: {},
    presentation: {
      thinking: false,
      speaking: false,
      tools: {},
      waitingForApproval: null,
      subtitle: '',
      expressionHint: null,
      lastEvent: null,
      shouldSpeak: false,
    },
    lastCursor: null,
  }
}

export function turnProjectionKey(sessionKey: string, requestId: string): string {
  return `${sessionKey}\u0000${requestId}`
}

export function reduceNeuroLinkProjection(
  state: NeuroLinkProjectionState,
  event: ProjectionEnvelopeEvent,
): NeuroLinkProjectionState {
  const sessionKey = event.envelope.correlation.session_key
  if (state.sessionKey && state.sessionKey !== sessionKey) return state
  const requestId = event.envelope.correlation.request_id
  const next: NeuroLinkProjectionState = {
    ...state,
    sessionKey: state.sessionKey ?? sessionKey,
    turns: { ...state.turns },
    presentation: {
      ...state.presentation,
      tools: { ...state.presentation.tools },
      shouldSpeak: false,
    },
    lastCursor: event.cursor,
  }
  const turn = requestId ? cloneTurn(next.turns[turnProjectionKey(sessionKey, requestId)], sessionKey, requestId) : null
  if (turn && requestId) next.turns[turnProjectionKey(sessionKey, requestId)] = turn

  switch (event.method as NeuroLinkProjectionMethod) {
    case 'conversation/stream': {
      if (!turn) return next
      const payload = event.envelope.payload
      if (payload.kind !== 'stream') return next
      const text = contentText(payload.parts)
      if (payload.phase === 'delta' || payload.phase === 'started') turn.content += text
      if (payload.phase === 'finalized') {
        if (text) turn.content = text
        turn.phase = payload.phase
      } else {
        turn.phase = payload.phase
      }
      if (payload.phase === 'failed') turn.error = text
      return next
    }
    case 'conversation/reasoning': {
      if (!turn) return next
      const payload = event.envelope.payload
      if (payload.kind === 'stream') turn.reasoning += contentText(payload.parts)
      return next
    }
    case 'turn/admission': {
      if (!turn) return next
      turn.admission = controlBody(event.envelope)
      turn.phase = admissionPhase(turn.admission)
      return next
    }
    case 'tool/lifecycle': {
      if (!turn) return next
      const payload = controlBody(event.envelope)
      const operation = controlOperation(event.envelope)
      const callId = stringValue(payload.call_id) ?? `${turn.requestId}:tool`
      const existing = turn.tools[callId]
      if (operation === 'tool/started') {
        turn.tools[callId] = {
          callId,
          name: stringValue(payload.name),
          argsPreview: stringValue(payload.args_preview),
          status: 'running',
        }
      } else if (operation === 'tool/finished') {
        turn.tools[callId] = {
          ...(existing ?? { callId }),
          name: stringValue(payload.name) ?? existing?.name,
          status: payload.is_error === true ? 'error' : 'success',
          isError: payload.is_error === true,
        }
      }
      return next
    }
    case 'planning/changed':
      if (turn) turn.planning = controlBody(event.envelope)
      return next
    case 'provider/status':
      if (turn) turn.provider = controlBody(event.envelope)
      return next
    case 'context/compaction':
      if (turn) turn.compaction = controlBody(event.envelope)
      return next
    case 'presentation/event':
      return reducePresentation(next, event)
    default:
      return next
  }
}

function cloneTurn(
  existing: NeuroLinkTurnProjection | undefined,
  sessionKey: string,
  requestId: string,
): NeuroLinkTurnProjection {
  return existing
    ? { ...existing, tools: { ...existing.tools } }
    : {
        sessionKey,
        requestId,
        content: '',
        reasoning: '',
        tools: {},
      }
}

function reducePresentation(
  state: NeuroLinkProjectionState,
  event: ProjectionEnvelopeEvent,
): NeuroLinkProjectionState {
  const payload = event.envelope.payload
  if (payload.kind !== 'presentation') return state
  const presentation = state.presentation
  presentation.lastEvent = payload.event
  switch (payload.event) {
    case 'assistant.thinking.started':
      presentation.thinking = true
      break
    case 'assistant.thinking.completed':
      presentation.thinking = false
      break
    case 'assistant.speaking.started':
      presentation.speaking = true
      break
    case 'assistant.speaking.completed':
      presentation.speaking = false
      break
    case 'assistant.tool.started': {
      const callId = stringValue(payload.body.call_id) ?? 'tool'
      presentation.tools[callId] = {
        callId,
        name: stringValue(payload.body.name),
        status: 'running',
      }
      break
    }
    case 'assistant.tool.completed': {
      const callId = stringValue(payload.body.call_id) ?? 'tool'
      presentation.tools[callId] = {
        ...(presentation.tools[callId] ?? { callId }),
        name: stringValue(payload.body.name) ?? presentation.tools[callId]?.name,
        status: payload.body.is_error === true ? 'error' : 'success',
        isError: payload.body.is_error === true,
      }
      break
    }
    case 'assistant.waiting_for_approval':
      presentation.waitingForApproval = payload.body
      break
    case 'subtitle.updated':
      presentation.subtitle = stringValue(payload.body.text) ?? ''
      presentation.shouldSpeak = event.source === 'live' && presentation.subtitle.trim().length > 0
      break
    case 'subtitle.cleared':
      presentation.subtitle = ''
      presentation.shouldSpeak = false
      break
    case 'persona.expression_hint':
      presentation.expressionHint = stringValue(payload.body.expression) ?? null
      break
    default:
      break
  }
  return state
}

function controlBody(envelope: ChannelEnvelopeV1): Record<string, unknown> {
  return envelope.payload.kind === 'control' && isRecord(envelope.payload.body)
    ? envelope.payload.body
    : {}
}

function controlOperation(envelope: ChannelEnvelopeV1): string | undefined {
  return envelope.payload.kind === 'control' ? envelope.payload.operation : undefined
}

function admissionPhase(body: Record<string, unknown>): StreamPhase | undefined {
  switch (body.phase) {
    case 'cancelled': return 'cancelled'
    case 'rejected':
    case 'unavailable':
    case 'evicted': return 'failed'
    default: return undefined
  }
}

function contentText(parts: unknown[]): string {
  if (!Array.isArray(parts)) return ''
  return parts.map((part) => {
    if (!isRecord(part)) return ''
    if (part.kind === 'text' && typeof part.text === 'string') return part.text
    if (part.kind === 'markdown' && typeof part.markdown === 'string') return part.markdown
    if (part.kind === 'audio' && isRecord(part.attachment) && typeof part.transcript === 'string') return part.transcript
    return ''
  }).join('')
}

function stringValue(value: unknown): string | undefined {
  return typeof value === 'string' ? value : undefined
}

function isRecord(value: unknown): value is Record<string, any> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export type { ProjectionSource }

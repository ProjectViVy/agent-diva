/**
 * Transport-neutral typed caller (DN-1). Forwards method names verbatim —
 * never interprets business methods, never retries. A timeout after a
 * mutating call was submitted is surfaced as `unknownOutcome` and must be
 * reconciled through reads, never auto-resubmitted.
 */
import {
  VivyCallError,
  type ApprovalListResult,
  type BridgeErrorBody,
  type InitializeResult,
  type RunGetResult,
  type RunLogResult,
  type RunSubscribeResult,
  type SessionGetResult,
  type SessionListResult,
  type SessionMessagesResult,
  type SessionMessage,
  type TurnStartResult,
  type VivySession,
  type WireEvent,
} from './contracts'
import type { VivyTransport } from './transport'

export interface CallOptions {
  timeoutMs?: number
  /** Mark mutating calls so a timeout/transport loss is reported as an
   * unknown outcome instead of a safe no-op. */
  mutation?: boolean
}

function isBridgeError(e: unknown): e is BridgeErrorBody {
  return (
    typeof e === 'object' &&
    e !== null &&
    typeof (e as BridgeErrorBody).kind === 'string' &&
    typeof (e as BridgeErrorBody).code === 'number' &&
    typeof (e as BridgeErrorBody).message === 'string'
  )
}

export class VivyClient {
  constructor(private readonly transport: VivyTransport) {}

  async call<T>(method: string, params?: unknown, opts: CallOptions = {}): Promise<T> {
    try {
      return (await this.transport.call({
        method,
        params,
        timeoutMs: opts.timeoutMs,
      })) as T
    } catch (e) {
      if (isBridgeError(e)) {
        const unknown =
          !!opts.mutation && (e.kind === 'timeout' || e.kind === 'transport_lost')
        throw new VivyCallError(e, unknown)
      }
      throw e
    }
  }

  /** Subscribe to bridge events; returns an unsubscribe function. */
  onEvent(handler: (event: WireEvent) => void): Promise<() => void> {
    return this.transport.onEvent(handler)
  }

  /** Detaches frontend listeners only; the embedded host keeps running. */
  close(): void {
    this.transport.close()
  }

  // --- Frozen methods verified by the DN-0 core-rpc fixture ---

  initialize(): Promise<InitializeResult> {
    return this.call('initialize')
  }
  sessionCreate(title?: string): Promise<VivySession> {
    return this.call('session/create', title ? { title } : null, { mutation: true })
  }
  sessionList(): Promise<SessionListResult> {
    return this.call('session/list')
  }
  sessionGet(sessionId: string): Promise<SessionGetResult> {
    return this.call('session/get', { session_id: sessionId })
  }
  sessionMessages(sessionId: string): Promise<SessionMessagesResult> {
    return this.call('session/messages', { session_id: sessionId })
  }
  sessionRename(sessionId: string, title: string): Promise<VivySession> {
    return this.call('session/rename', { session_id: sessionId, title }, { mutation: true })
  }
  sessionDelete(sessionId: string): Promise<{ deleted: boolean }> {
    return this.call('session/delete', { session_id: sessionId }, { mutation: true })
  }
  turnStart(sessionId: string, text: string): Promise<TurnStartResult> {
    return this.call('turn/start', { session_id: sessionId, text }, { mutation: true })
  }
  runSubscribe(runId: string): Promise<RunSubscribeResult> {
    return this.call('run/subscribe', { run_id: runId })
  }
  runGet(runId: string): Promise<RunGetResult> {
    return this.call('run/get', { run_id: runId })
  }
  runLog(runId: string): Promise<RunLogResult> {
    return this.call('run/log', { run_id: runId })
  }
  runCancel(runId: string): Promise<unknown> {
    return this.call('run/cancel', { run_id: runId }, { mutation: true })
  }
  approvalList(): Promise<ApprovalListResult> {
    return this.call('approval/list')
  }
  approvalRespond(approvalId: string, decision: 'approved' | 'denied', reason?: string): Promise<unknown> {
    return this.call(
      'approval/respond',
      { approval_id: approvalId, decision, ...(reason ? { reason } : {}) },
      { mutation: true },
    )
  }
  questionList(): Promise<{ questions: Array<Record<string, unknown>> }> {
    return this.call('question/list')
  }
  questionRespond(questionId: string, answer: string): Promise<unknown> {
    return this.call('question/respond', { question_id: questionId, answer }, { mutation: true })
  }
}

/** Last-messages helper kept generic for projection consumers. */
export type { SessionMessage }

/**
 * VivyChatController (DN-2) — the single orchestration authority for the
 * migrated chat/session/approval/question/plan flows. It owns the VIVY
 * client + session projection, composes the message timeline from history
 * snapshots plus per-run event reduction, and performs every decision RPC
 * against real backend ids. Framework-agnostic: the component layer copies
 * the plain getters into refs on `subscribe` notifications.
 *
 * Invariants kept from the plan:
 * - history comes from session/messages (via session/get snapshot);
 * - turn/start returns the authoritative run; we never resend on timeout —
 *   an `unknownOutcome` is reconciled through session/messages + session/work;
 * - run/cancel is the only cancellation path;
 * - approvals/questions come from review/list + question/list and are
 *   answered through review/respond / question/respond with backend ids;
 * - plan state comes from session/work (+ session/todos); plan decisions go
 *   through plan/decide with expected_version — never an approval alias.
 */
import {
  VivyCallError,
  type ReviewItem,
  type SessionMessage,
  type SessionWorkResult,
  type VivySession,
} from '../api/vivy/contracts'
import type { VivyClient } from '../api/vivy/client'
import { reviewsToApprovalViews, type ApprovalView } from '../api/approvals'
import {
  todoToPlanRuntime,
  workToPlanRuntime,
  type PlanRuntimeState,
  type PlanRuntimeTodo,
} from '../api/planning'
import type { AskUserQuestionView } from '../components/AskUserQuestionCard.vue'
import { generateMessageId, type ChatMessage } from './chat-message'
import { mapHistoryMessages, reduceRunMessages } from './vivy-run-messages'
import { VivySessionProjection } from './vivy-session'

export interface SessionInfo {
  session_key: string
  chat_id: string
  snippet: string
  timestamp: number
  title?: string
  last_message?: string
  message_count: number
  title_generated: boolean
  title_manually_set: boolean
  pinned?: boolean
}

export type ConnectionStatus = 'connecting' | 'connected' | 'reconnecting' | 'error'

/** Matches the tool.approval_* / user.question_* families in the domain. */
const INTERACTION_EVENT_PREFIXES = ['tool.approval_', 'user.question_']

function sessionToInfo(session: VivySession): SessionInfo {
  return {
    session_key: session.id,
    chat_id: session.id,
    snippet: session.title || '',
    timestamp: session.updated_at * 1000,
    title: session.title,
    message_count: 0,
    title_generated: false,
    title_manually_set: false,
  }
}

export class VivyChatController {
  readonly projection = new VivySessionProjection()

  /** Ordered run ids per session, discovered from sends + run/get snapshots. */
  private readonly runOwner = new Map<string, string>()
  private readonly echoBySession = new Map<string, ChatMessage[]>()
  private readonly reviews = new Map<string, ReviewItem>()
  private readonly workBySession = new Map<string, SessionWorkResult>()
  private readonly todosBySession = new Map<string, PlanRuntimeTodo[]>()
  private readonly listeners = new Set<() => void>()

  /** Per-request decision errors + ambiguity markers for the drawers. */
  readonly actionErrors = new Map<string, string>()
  readonly outcomeUnknown = new Set<string>()
  private readonly submitting = new Set<string>()

  currentSessionId: string | null = null
  private activeRunId: string | null = null
  private ready = false
  private sendFailed = false

  private detachProjection: (() => void) | null = null
  private detachEvents: (() => void) | null = null
  private interactionsRefresh: Promise<void> | null = null
  private workRefresh = new Map<string, Promise<void>>()

  constructor(private readonly client: VivyClient) {}

  subscribe(listener: () => void): () => void {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  private emit(): void {
    for (const listener of this.listeners) listener()
  }

  /** Attach listener-first, then load the authoritative snapshots. */
  async connect(): Promise<void> {
    await this.projection.attach(this.client)
    this.detachProjection = this.projection.subscribe(() => {
      this.emit()
    })
    this.detachEvents = await this.client.onEvent((event) => {
      if (event.kind !== 'vivy') return
      if (event.method === 'run/event') {
        const runEvent = (event.params as { event?: { type?: string; run_id?: string } }).event
        const type = runEvent?.type ?? ''
        if (type === 'context.compacted') {
          this.compaction = { at: Date.now() }
          this.emit()
        } else if (INTERACTION_EVENT_PREFIXES.some((p) => type.startsWith(p))) {
          void this.refreshInteractions()
        }
      } else if (event.method === 'session/work/event') {
        const sessionId = (event.params as { session_id?: string }).session_id
          ?? (this.currentSessionId ?? undefined)
        if (sessionId) void this.refreshWork(sessionId)
      }
    })
    const [sessions, reviewList] = await Promise.all([
      this.client.sessionList(),
      this.client.reviewList(),
    ])
    this.projection.applySnapshot({ type: 'session/list', result: sessions })
    this.applyReviewList(reviewList.reviews)
    if (!this.currentSessionId && sessions.sessions.length > 0) {
      const latest = [...sessions.sessions].sort((a, b) => b.updated_at - a.updated_at)[0]
      await this.loadSession(latest.id)
    }
    this.ready = true
    this.emit()
  }

  disconnect(): void {
    this.detachProjection?.()
    this.detachProjection = null
    this.detachEvents?.()
    this.detachEvents = null
    this.projection.detach()
    this.ready = false
  }

  // --- Views ---------------------------------------------------------------

  status(): ConnectionStatus {
    const c = this.projection.connection
    if (c === 'connected') return 'connected'
    if (c === 'gap') return 'reconnecting'
    if (c === 'lost') return 'error'
    return this.ready ? 'error' : 'connecting'
  }

  sessions(): SessionInfo[] {
    return [...this.projection.sessions.values()]
      .map(sessionToInfo)
      .sort((a, b) => b.timestamp - a.timestamp)
  }

  isTyping(): boolean {
    if (this.sendFailed) return false
    const runId = this.activeRunId
    if (!runId) return false
    const phase = this.projection.run(runId)?.phase
    return phase === 'active' || phase == null
  }

  /** run ids that belong to a session, in the order they were learned. */
  private sessionRunIds(sessionId: string): string[] {
    const out: string[] = []
    for (const id of this.projection.runIds()) {
      const owner = this.runOwner.get(id) ?? this.projection.run(id)?.sessionId
      if (owner === sessionId) out.push(id)
    }
    return out
  }

  messages(): ChatMessage[] {
    const sessionId = this.currentSessionId
    if (!sessionId) return []
    const history = mapHistoryMessages(this.projection.messages.get(sessionId) ?? [])
    const echoes = this.echoBySession.get(sessionId) ?? []
    const out = [...history]
    // Drop echoes the backend already persisted (tail user message matches).
    const pendingEchoes = echoes.filter((echo) => {
      const lastUser = [...history].reverse().find((m) => m.role === 'user')
      return !(lastUser && lastUser.content === echo.content)
    })
    if (pendingEchoes.length !== echoes.length) {
      this.echoBySession.set(sessionId, pendingEchoes)
    }
    out.push(...pendingEchoes)
    for (const runId of this.sessionRunIds(sessionId)) {
      const run = this.projection.run(runId)
      if (run) out.push(...reduceRunMessages(run.events))
    }
    return out
  }

  approvals(): ApprovalView[] {
    return reviewsToApprovalViews([...this.reviews.values()])
  }

  questions(): AskUserQuestionView[] {
    return [...this.reviews.values()]
      .filter((r) => r.kind === 'question' && r.status === 'pending')
      .map((r) => ({
        question_id: r.id,
        question: r.prompt || '',
        // VIVY questions are free-text; no choices exist in the payload.
        choices: [],
        allow_other: true,
        created_at: typeof r.created_at === 'number'
          ? new Date(r.created_at * 1000).toISOString()
          : '',
        timeout_seconds: typeof r.expires_at === 'number' && typeof r.created_at === 'number'
          ? Math.max(0, r.expires_at - r.created_at)
          : 0,
      }))
  }

  plan(): PlanRuntimeState | null {
    const sessionId = this.currentSessionId
    const work = sessionId ? this.workBySession.get(sessionId) : undefined
    if (!work) return null
    const runtime = workToPlanRuntime(work)
    if (runtime) runtime.todos = this.todosBySession.get(sessionId as string) ?? []
    return runtime
  }

  isSubmitting(id: string): boolean {
    return this.submitting.has(id)
  }

  // --- Session operations --------------------------------------------------

  async refreshSessions(): Promise<void> {
    const list = await this.client.sessionList()
    this.projection.applySnapshot({ type: 'session/list', result: list })
    this.emit()
  }

  async newSession(): Promise<void> {
    const session = await this.client.sessionCreate()
    await this.refreshSessions()
    await this.loadSession(session.id)
  }

  async loadSession(sessionId: string): Promise<boolean> {
    const detail = await this.client.sessionGet(sessionId)
    this.projection.applySnapshot({ type: 'session/get', result: detail })
    this.currentSessionId = sessionId
    for (const m of detail.messages) {
      if (m.run_id) this.runOwner.set(m.run_id, sessionId)
    }
    await this.refreshWork(sessionId).catch(() => undefined)
    void this.refreshInteractions()
    // Re-subscribe to any runs that still have live events after a reopen.
    for (const runId of this.sessionRunIds(sessionId)) {
      if (this.projection.run(runId)?.needsResync) void this.client.runSubscribe(runId)
    }
    this.emit()
    return true
  }

  async deleteSession(sessionId: string): Promise<void> {
    await this.client.sessionDelete(sessionId)
    this.projection.sessions.delete(sessionId)
    this.projection.messages.delete(sessionId)
    this.workBySession.delete(sessionId)
    this.todosBySession.delete(sessionId)
    if (this.currentSessionId === sessionId) this.currentSessionId = null
    await this.refreshSessions()
    if (!this.currentSessionId) {
      const next = this.sessions()[0]
      if (next) await this.loadSession(next.session_key)
    }
    this.emit()
  }

  async renameSession(sessionId: string, title: string): Promise<void> {
    await this.client.sessionRename(sessionId, title)
    await this.refreshSessions()
  }

  // --- Turn lifecycle --------------------------------------------------------

  async send(text: string): Promise<void> {
    const content = text.trim()
    if (!content) return
    if (!this.currentSessionId) {
      const session = await this.client.sessionCreate()
      await this.refreshSessions()
      this.currentSessionId = session.id
      this.projection.applySnapshot({
        type: 'session/get',
        result: { session, messages: [] },
      })
    }
    const sessionId = this.currentSessionId
    const echo: ChatMessage = {
      id: generateMessageId(),
      role: 'user',
      content,
      timestamp: Date.now(),
    }
    const echoes = this.echoBySession.get(sessionId) ?? []
    echoes.push(echo)
    this.echoBySession.set(sessionId, echoes)
    this.sendFailed = false
    this.emit()
    try {
      const started = await this.client.turnStart(sessionId, content)
      this.activeRunId = started.run_id
      this.runOwner.set(started.run_id, sessionId)
      // A failed subscribe leaves the run without events; resync covers it.
      void this.client.runSubscribe(started.run_id).catch(() => undefined)
      this.emit()
    } catch (e) {
      if (e instanceof VivyCallError && e.unknownOutcome) {
        await this.reconcileAmbiguousSend(sessionId, content, echo.id)
        return
      }
      this.sendFailed = true
      this.echoBySession.set(
        sessionId,
        (this.echoBySession.get(sessionId) ?? []).filter((m) => m.id !== echo.id),
      )
      this.emit()
      throw e
    }
  }

  /**
   * A mutating turn/start timed out or lost transport: never resend. Read
   * session/messages to see whether the user message landed and session/work
   * for an adopted run id; keep the local echo until the backend echoes it.
   */
  private async reconcileAmbiguousSend(
    sessionId: string,
    content: string,
    echoId: string,
  ): Promise<void> {
    try {
      const [msgs, work] = await Promise.all([
        this.client.sessionMessages(sessionId),
        this.client.sessionWork(sessionId).catch(() => null),
      ])
      const landed = msgs.messages.some(
        (m: SessionMessage) => m.role === 'user' && m.content === content,
      )
      if (landed) {
        this.projection.messages.set(sessionId, msgs.messages)
        this.echoBySession.set(
          sessionId,
          (this.echoBySession.get(sessionId) ?? []).filter((m) => m.id !== echoId),
        )
      }
      if (work?.current_run_id) {
        this.activeRunId = work.current_run_id
        this.runOwner.set(work.current_run_id, sessionId)
        void this.client.runSubscribe(work.current_run_id)
      }
      if (!landed && !work?.current_run_id) {
        // Nothing reached the backend: keep the echo, flag the send.
        this.sendFailed = true
      }
      this.emit()
    } catch {
      // Reconciliation itself failed — keep the echo; the next refresh retries.
      this.emit()
    }
  }

  /** run/cancel is the only cancellation path. */
  async stop(): Promise<void> {
    const runId = this.activeRunId
    if (!runId) return
    await this.client.runCancel(runId)
  }

  clearLocalMessages(): void {
    const sessionId = this.currentSessionId
    if (sessionId) this.echoBySession.delete(sessionId)
    this.emit()
  }

  // --- Interactions (approvals / questions) ----------------------------------

  private applyReviewList(reviews: ReviewItem[]): void {
    this.reviews.clear()
    for (const review of reviews) this.reviews.set(review.id, review)
  }

  async refreshInteractions(): Promise<void> {
    if (this.interactionsRefresh) return this.interactionsRefresh
    this.interactionsRefresh = (async () => {
      const list = await this.client.reviewList()
      this.applyReviewList(list.reviews)
      this.emit()
    })().finally(() => {
      this.interactionsRefresh = null
    })
    return this.interactionsRefresh
  }

  /** Approve/deny a pending review by its backend id; errors stay visible. */
  async decideApproval(requestId: string, allow: boolean): Promise<void> {
    this.submitting.add(requestId)
    this.actionErrors.delete(requestId)
    this.outcomeUnknown.delete(requestId)
    this.emit()
    try {
      await this.client.reviewRespond({
        review_id: requestId,
        action: allow ? 'approve' : 'deny',
      })
      const review = this.reviews.get(requestId)
      if (review) this.reviews.set(requestId, { ...review, status: allow ? 'approved' : 'denied' })
    } catch (e) {
      if (e instanceof VivyCallError) {
        if (e.unknownOutcome) {
          this.outcomeUnknown.add(requestId)
        } else {
          this.actionErrors.set(requestId, e.message)
        }
      } else {
        this.actionErrors.set(requestId, e instanceof Error ? e.message : String(e))
      }
    } finally {
      this.submitting.delete(requestId)
    }
    await this.refreshInteractions()
    this.emit()
  }

  /** Cancel a pending approval (legacy "cancel_approval"). */
  async cancelApproval(requestId: string): Promise<void> {
    this.submitting.add(requestId)
    this.actionErrors.delete(requestId)
    this.outcomeUnknown.delete(requestId)
    this.emit()
    try {
      await this.client.reviewRespond({ review_id: requestId, action: 'cancel' })
      const review = this.reviews.get(requestId)
      if (review) this.reviews.set(requestId, { ...review, status: 'cancelled' })
    } catch (e) {
      if (e instanceof VivyCallError) {
        if (e.unknownOutcome) this.outcomeUnknown.add(requestId)
        else this.actionErrors.set(requestId, e.message)
      } else {
        this.actionErrors.set(requestId, e instanceof Error ? e.message : String(e))
      }
    } finally {
      this.submitting.delete(requestId)
    }
    await this.refreshInteractions()
    this.emit()
  }

  async answerQuestion(questionId: string, answer: string): Promise<void> {
    this.submitting.add(questionId)
    this.actionErrors.delete(questionId)
    this.emit()
    try {
      await this.client.reviewRespond({ review_id: questionId, action: 'answer', answer })
      const review = this.reviews.get(questionId)
      if (review) this.reviews.set(questionId, { ...review, status: 'answered' })
    } catch (e) {
      this.actionErrors.set(
        questionId,
        e instanceof VivyCallError ? e.message : e instanceof Error ? e.message : String(e),
      )
    } finally {
      this.submitting.delete(questionId)
    }
    await this.refreshInteractions()
    this.emit()
  }

  async cancelQuestion(questionId: string): Promise<void> {
    this.submitting.add(questionId)
    this.actionErrors.delete(questionId)
    this.emit()
    try {
      await this.client.reviewRespond({ review_id: questionId, action: 'cancel' })
      const review = this.reviews.get(questionId)
      if (review) this.reviews.set(questionId, { ...review, status: 'cancelled' })
    } catch (e) {
      this.actionErrors.set(
        questionId,
        e instanceof VivyCallError ? e.message : e instanceof Error ? e.message : String(e),
      )
    } finally {
      this.submitting.delete(questionId)
    }
    await this.refreshInteractions()
    this.emit()
  }

  // --- Planning / work -------------------------------------------------------

  async refreshWork(sessionId: string): Promise<void> {
    const inFlight = this.workRefresh.get(sessionId)
    if (inFlight) return inFlight
    const task = (async () => {
      const [work, todos] = await Promise.all([
        this.client.sessionWork(sessionId),
        this.client.sessionTodos(sessionId).catch(() => null),
      ])
      this.workBySession.set(sessionId, work)
      if (todos) this.todosBySession.set(sessionId, todos.todos.map(todoToPlanRuntime))
      if (work.current_run_id) this.runOwner.set(work.current_run_id, sessionId)
      this.emit()
    })().finally(() => {
      this.workRefresh.delete(sessionId)
    })
    this.workRefresh.set(sessionId, task)
    return task
  }

  /**
   * Decide the pending plan review through plan/decide. `execute_once`
   * approves, `revise` returns feedback, `start_goal` arms the goal loop.
   */
  async decidePlan(action: 'revise' | 'execute_once' | 'start_goal', feedback?: string): Promise<void> {
    const sessionId = this.currentSessionId
    if (!sessionId) throw new Error('no active session')
    const work = this.workBySession.get(sessionId)
    if (!work?.plan.submission_id) throw new Error('no pending plan submission')
    const result = await this.client.planDecide({
      session_id: sessionId,
      request_id: generateMessageId(),
      submission_id: work.plan.submission_id,
      action,
      ...(feedback ? { feedback } : {}),
      ...(typeof work.version === 'number' ? { expected_version: work.version } : {}),
    })
    this.workBySession.set(sessionId, result.work)
    this.emit()
  }

  /** Resume a paused goal loop (legacy "resume plan execution"). */
  async resumeWork(): Promise<void> {
    const sessionId = this.currentSessionId
    if (!sessionId) return
    const result = await this.client.goalResume(sessionId)
    this.workBySession.set(sessionId, result.work)
    this.emit()
  }

  /** Pause the goal loop (legacy "revoke plan execution"). */
  async pauseWork(): Promise<void> {
    const sessionId = this.currentSessionId
    if (!sessionId) return
    const result = await this.client.goalPause(sessionId)
    this.workBySession.set(sessionId, result.work)
    this.emit()
  }

  /** Leave the plan review surface (legacy "dismiss/exit plan review"). */
  async leavePlan(): Promise<void> {
    const sessionId = this.currentSessionId
    if (!sessionId) return
    const result = await this.client.planLeave(sessionId)
    this.workBySession.set(sessionId, result.work)
    this.emit()
  }

  /**
   * Latest compaction notification seen for the session. The domain event
   * is `context.compacted`; there is no separate start signal, so the UI
   * surfaces it as a completed compaction note until the next snapshot.
   */
  compaction: { at: number } | null = null
}

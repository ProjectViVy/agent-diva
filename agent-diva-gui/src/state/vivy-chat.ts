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
  type PermissionPreset,
  type ReviewItem,
  type RunEvent,
  type SessionMessage,
  type SessionWorkResult,
  type TurnAttachment,
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
import {
  assertFramedRequest,
  ChatAttachmentError,
  validateTurnAttachments,
} from './chat-images'
import { getSessionTrajectory } from '../api/vivy/observability'
import { mapHistoryMessages, reduceRunMessages } from './vivy-run-messages'
import { VivySessionProjection } from './vivy-session'
import { TrajectoryProjection, type TrajectoryView } from './vivy-trajectory'

/** VIVY RPC CodeNotFound — `run/cancel` reports it for a run the
 * restarted backend no longer holds in process. */
const RPC_CODE_NOT_FOUND = -32004

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

/** DN-2A send options: already-encoded image attachments plus the
 * permission preset to arm before the turn starts. */
export interface SendOptions {
  attachments?: TurnAttachment[]
  preset?: PermissionPreset
}

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

  /**
   * DN-2B retryable draft: set when a rewind succeeded but the follow-up
   * send failed — the rewound view stays visible and the original text +
   * image bytes survive for an explicit manual resend (never automatic).
   */
  retryDraft: { sessionId: string; content: string; attachments: TurnAttachment[] } | null = null

  /** Local cancellation notification for conversation mutations
   * (DN-2B → DN-6C voice subscribes instead of rewriting transitions).
   * Not a VIVY RPC — it only tells local consumers to drop session-bound
   * work (e.g. an in-flight voice request) before history changes. */
  private readonly invalidators = new Set<(reason: string) => void>()

  onConversationInvalidate(listener: (reason: string) => void): () => void {
    this.invalidators.add(listener)
    return () => this.invalidators.delete(listener)
  }

  private invalidateConversation(reason: string): void {
    for (const listener of this.invalidators) listener(reason)
  }

  /** DN-6C: run ids admitted to the fresh-primary-reply lane. Only runs
   * this UI started (turn/start, session/edit) or that session/work still
   * reports as the live run enter the set — replayed history runs and
   * child runs never do, so auto-read can never fire for them. Entries
   * drop on any terminal event, invalidation, or successful notify. */
  private readonly freshPrimaryRuns = new Set<string>()
  private readonly childRunIds = new Set<string>()
  private readonly freshReplyListeners = new Set<(reply: { runId: string; text: string }) => void>()

  onFreshReply(listener: (reply: { runId: string; text: string }) => void): () => void {
    this.freshReplyListeners.add(listener)
    return () => this.freshReplyListeners.delete(listener)
  }

  private markFreshPrimary(runId: string | null | undefined): void {
    if (runId) this.freshPrimaryRuns.add(runId)
  }

  private handleRunTerminal(runId: string, type: string): void {
    if (type !== 'run.completed' && type !== 'run.failed' && type !== 'run.cancelled') return
    this.childRunIds.delete(runId)
    const wasFresh = this.freshPrimaryRuns.delete(runId)
    if (!wasFresh || type !== 'run.completed') return
    if (this.runOwner.get(runId) !== this.currentSessionId) return
    // A needsResync run's event chain is incomplete — a replayed terminal
    // event from a resync can never count as a fresh reply.
    if (this.projection.run(runId)?.needsResync) return
    const text = this.projection.answer(runId)
    if (!text) return
    for (const listener of this.freshReplyListeners) listener({ runId, text })
  }

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
      const c = this.projection.connection
      if (c === 'gap' || c === 'lost') {
        this.traj.markGap()
      } else if (c === 'connected' && this.trajNeedsRefresh) {
        this.trajNeedsRefresh = false
        void this.refreshTrajectory()
      }
      this.emit()
    })
    this.detachEvents = await this.client.onEvent((event) => {
      if (event.kind !== 'vivy') return
      if (event.method === 'run/event') {
        const runEvent = (event.params as { event?: Partial<RunEvent> }).event
        const type = runEvent?.type ?? ''
        // OBS-07: the single owner also feeds the trajectory projection.
        if (runEvent && typeof runEvent.seq === 'number') {
          this.traj.applyRunEvent(runEvent as RunEvent)
          this.scheduleTrajectoryRefetch()
        }
        // DN-6C: a child run tags itself via child.started.parent_run_id —
        // its terminal event never enters the fresh-primary lane.
        if (type === 'child.started' && runEvent?.run_id) {
          const parentId = (runEvent.payload as { parent_run_id?: unknown } | undefined)?.parent_run_id
          if (typeof parentId === 'string' && parentId) this.childRunIds.add(runEvent.run_id)
        }
        if (runEvent?.run_id && !this.childRunIds.has(runEvent.run_id)) {
          this.handleRunTerminal(runEvent.run_id, type)
        }
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
    } else if (!this.currentSessionId) {
      this.traj.clear()
    }
    this.ready = true
    this.emit()
  }

  disconnect(): void {
    this.trajGen++
    this.traj.clear()
    this.trajError = null
    if (this.trajRefetchTimer !== null) {
      clearTimeout(this.trajRefetchTimer)
      this.trajRefetchTimer = null
    }
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
    this.invalidateConversation('session/new')
    const session = await this.client.sessionCreate()
    await this.refreshSessions()
    await this.loadSession(session.id)
  }

  async loadSession(sessionId: string): Promise<boolean> {
    this.invalidateConversation('session/load')
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
    void this.refreshTrajectory()
    this.emit()
    return true
  }

  // --- Trajectory (OBS-07, read-only consumer of this owner) -----------

  private readonly traj = new TrajectoryProjection()
  private trajGen = 0
  private trajError: string | null = null
  private trajRefetchTimer: ReturnType<typeof setTimeout> | null = null
  private trajNeedsRefresh = false

  trajectory(): TrajectoryView | null {
    return this.traj.view()
  }

  trajectoryError(): string | null {
    return this.trajError
  }

  /** Authoritative snapshot fetch, fenced by session + generation: a
   * late response for a previous session or an older request is dropped. */
  async refreshTrajectory(): Promise<void> {
    const sessionId = this.currentSessionId
    const gen = ++this.trajGen
    if (!sessionId) {
      this.traj.clear()
      this.emit()
      return
    }
    try {
      const snap = await getSessionTrajectory(this.client, sessionId)
      if (gen !== this.trajGen || this.currentSessionId !== sessionId) return
      this.traj.load(snap)
      this.trajError = null
      this.emit()
    } catch (e) {
      if (gen !== this.trajGen) return
      this.trajError = e instanceof Error ? e.message : String(e)
      this.emit()
    }
  }

  /** Live events mark retained rows stale; a debounced refetch
   * reconciles — events never fabricate rows. */
  private scheduleTrajectoryRefetch(): void {
    const view = this.traj.view()
    if (!view || !view.stale) return
    if (this.projection.connection !== 'connected') {
      this.trajNeedsRefresh = true
      return
    }
    if (this.trajRefetchTimer !== null) return
    this.trajRefetchTimer = setTimeout(() => {
      this.trajRefetchTimer = null
      void this.refreshTrajectory()
    }, 400)
  }

  async deleteSession(sessionId: string): Promise<void> {
    this.invalidateConversation('session/delete')
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

  /** One mutation lane per controller: every send — including its
   * permission preset write + readback — completes before the next one
   * starts, so duplicate clicks or a session switch cannot interleave
   * two mutation sequences. */
  private sendChain: Promise<void> = Promise.resolve()

  send(text: string, opts: SendOptions = {}): Promise<void> {
    const content = text.trim()
    if (!content) return Promise.resolve()
    const attachments = opts.attachments ?? []
    const sessionId = this.currentSessionId
    let echo: ChatMessage | null = null
    try {
      // Every validation runs before the first mutation — an unsupported
      // or oversized payload preserves the draft and sends nothing.
      validateTurnAttachments(attachments)
      if (sessionId) {
        assertFramedRequest('turn/start', this.turnStartParams(sessionId, content, attachments))
        echo = this.pushEcho(sessionId, content)
      }
    } catch (e) {
      return Promise.reject(e)
    }
    const task = this.sendChain.then(() =>
      this.performSend(content, attachments, opts, sessionId, echo),
    )
    this.sendChain = task.catch(() => undefined)
    return task
  }

  private turnStartParams(sessionId: string, content: string, attachments: TurnAttachment[]) {
    return {
      session_id: sessionId,
      text: content,
      ...(attachments.length ? { attachments } : {}),
    }
  }

  private pushEcho(sessionId: string, content: string): ChatMessage {
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
    return echo
  }

  private dropEcho(sessionId: string, echoId: string): void {
    this.echoBySession.set(
      sessionId,
      (this.echoBySession.get(sessionId) ?? []).filter((m) => m.id !== echoId),
    )
    this.emit()
  }

  private async performSend(
    content: string,
    attachments: TurnAttachment[],
    opts: SendOptions,
    knownSessionId: string | null,
    echo: ChatMessage | null,
  ): Promise<void> {
    let sessionId = knownSessionId
    try {
      if (!sessionId) {
        const session = await this.client.sessionCreate()
        await this.refreshSessions()
        this.currentSessionId = session.id
        this.projection.applySnapshot({
          type: 'session/get',
          result: { session, messages: [] },
        })
        sessionId = session.id
        assertFramedRequest(
          'turn/start',
          this.turnStartParams(sessionId, content, attachments),
        )
        echo = this.pushEcho(sessionId, content)
      }
      // The chosen preset is armed and confirmed before the turn starts; a
      // mismatch or ambiguous outcome blocks the send on the authoritative
      // session readback instead of sending under a stale policy.
      if (opts.preset) {
        await this.confirmPermission(sessionId, opts.preset)
      }
      const started = await this.client.turnStart(sessionId, content, attachments)
      this.activeRunId = started.run_id
      this.runOwner.set(started.run_id, sessionId)
      this.markFreshPrimary(started.run_id)
      // A failed subscribe leaves the run without events; resync covers it.
      void this.client.runSubscribe(started.run_id).catch(() => undefined)
      this.emit()
    } catch (e) {
      if (e instanceof VivyCallError && e.unknownOutcome && sessionId && echo) {
        await this.reconcileAmbiguousSend(sessionId, content, echo.id)
        return
      }
      this.sendFailed = true
      if (echo && sessionId) this.dropEcho(sessionId, echo.id)
      this.emit()
      throw e
    }
  }

  /**
   * Arm the chosen preset on the owning session and confirm it before the
   * turn starts. The `session/set_permission` result is the admitted
   * snapshot; a mismatch or an ambiguous outcome (timeout/transport loss
   * after the write was submitted) is reconciled by an authoritative
   * `session/get` readback — sending under an unconfirmed stale policy is
   * never allowed. Throws to block the send when the preset is not
   * admitted.
   */
  private async confirmPermission(sessionId: string, preset: PermissionPreset): Promise<void> {
    try {
      const session = await this.client.setSessionPermission(sessionId, preset)
      if (session.permission_preset === preset) {
        this.projection.sessions.set(sessionId, session)
        this.emit()
        return
      }
      // Definite response, wrong admitted value — fall through to the
      // authoritative readback instead of trusting a transient projection.
    } catch (e) {
      if (!(e instanceof VivyCallError && e.unknownOutcome)) throw e
      // Ambiguous outcome: the write may have landed — read, never rewrite.
    }
    const detail = await this.client.sessionGet(sessionId)
    this.projection.applySnapshot({ type: 'session/get', result: detail })
    if (detail.session.permission_preset !== preset) {
      throw new Error(
        `permission preset "${preset}" was not admitted (session reports ${
          detail.session.permission_preset ?? 'unset'
        })`,
      )
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
        this.markFreshPrimary(work.current_run_id)
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

  /** run/cancel is the only cancellation path. A restarted backend no
   * longer holds the run in process and answers not-found — read back the
   * authoritative run state instead of claiming settled. */
  async stop(): Promise<void> {
    const runId = this.activeRunId
    if (!runId) return
    this.invalidateConversation('run/cancel')
    try {
      await this.client.runCancel(runId)
    } catch (e) {
      if (!(e instanceof VivyCallError && e.code === RPC_CODE_NOT_FOUND)) throw e
      const run = await this.client.runGet(runId)
      this.projection.applySnapshot({ type: 'run/get', result: run })
      if (run.status !== 'active') this.activeRunId = null
      this.emit()
    }
  }

  // --- Selected-turn regeneration / history mutation (DN-2B) -----------------

  /** Resolve a selected assistant message to its originating user turn:
   * the nearest user history row before it — never the session's last
   * user turn, which may belong to an unrelated run. */
  private originUserMessage(sessionId: string, assistantMessageId: string): SessionMessage {
    const history = this.projection.messages.get(sessionId) ?? []
    const idx = history.findIndex((m) => m.id === assistantMessageId)
    if (idx === -1) throw new Error('message is not in the session history')
    for (let i = idx - 1; i >= 0; i -= 1) {
      if (history[i]!.role === 'user') return history[i]!
    }
    throw new Error('originating user turn not found')
  }

  /** Re-encode history-carried `data_url` attachments to the wire `data`
   * base64 form turn/start expects. Throws rather than silently dropping
   * an image the backend stored. */
  private decodeHistoryAttachments(msg: SessionMessage): TurnAttachment[] {
    const list = Array.isArray(msg.attachments) ? msg.attachments : []
    const out: TurnAttachment[] = []
    for (const a of list) {
      const match = /^data:([^;,]+);base64,(.*)$/s.exec(a.data_url ?? '')
      if (!match) {
        throw new ChatAttachmentError(
          'sniff_mismatch',
          `attachment ${a.name ?? 'unnamed'} carries no base64 data_url`,
        )
      }
      out.push({ name: a.name, mime_type: a.mime_type || match[1]!, data: match[2]! })
    }
    return out
  }

  /** Regenerate the turn that produced the selected assistant message.
   * Text-only turns go through the atomic `session/edit`; turns with
   * images validate the original bytes first, then rewind inclusively at
   * the originating user message and resend via normal `turn/start`. */
  regenerate(assistantMessageId: string): Promise<void> {
    const sessionId = this.currentSessionId
    if (!sessionId) return Promise.resolve()
    let origin: SessionMessage
    try {
      origin = this.originUserMessage(sessionId, assistantMessageId)
    } catch (e) {
      return Promise.reject(e)
    }
    const task = this.sendChain.then(() => this.performRegenerate(sessionId, origin))
    this.sendChain = task.catch(() => undefined)
    return task
  }

  private async performRegenerate(sessionId: string, origin: SessionMessage): Promise<void> {
    this.invalidateConversation('turn/regenerate')
    const attachments = this.decodeHistoryAttachments(origin)
    if (attachments.length === 0) {
      const edited = await this.client.sessionEdit(sessionId, origin.id, origin.content)
      this.activeRunId = edited.run_id
      this.runOwner.set(edited.run_id, sessionId)
      this.markFreshPrimary(edited.run_id)
      void this.client.runSubscribe(edited.run_id).catch(() => undefined)
      await this.refreshSessionView(sessionId)
      this.emit()
      return
    }
    // Every validation runs before the first mutation: an unsupported or
    // oversized original payload blocks regeneration instead of losing
    // the image or degrading to a text-only resend.
    validateTurnAttachments(attachments)
    assertFramedRequest('turn/start', this.turnStartParams(sessionId, origin.content, attachments))
    await this.client.sessionRewind(sessionId, origin.id)
    await this.refreshSessionView(sessionId).catch(() => undefined)
    try {
      const started = await this.client.turnStart(sessionId, origin.content, attachments)
      this.activeRunId = started.run_id
      this.runOwner.set(started.run_id, sessionId)
      this.markFreshPrimary(started.run_id)
      void this.client.runSubscribe(started.run_id).catch(() => undefined)
      this.emit()
    } catch (e) {
      this.retryDraft = { sessionId, content: origin.content, attachments }
      this.emit()
      throw e
    }
  }

  /** Inclusive rewind at a user message: the message and everything after
   * leave the live view; its content is retained as a retryable draft. */
  rewind(userMessageId: string): Promise<void> {
    const sessionId = this.currentSessionId
    if (!sessionId) return Promise.resolve()
    const msg = (this.projection.messages.get(sessionId) ?? []).find(
      (m) => m.id === userMessageId,
    )
    const task = this.sendChain.then(async () => {
      this.invalidateConversation('turn/rewind')
      await this.client.sessionRewind(sessionId, userMessageId)
      if (msg?.role === 'user') {
        this.retryDraft = {
          sessionId,
          content: msg.content,
          attachments: this.decodeHistoryAttachments(msg),
        }
      }
      await this.refreshSessionView(sessionId)
      this.emit()
    })
    this.sendChain = task.catch(() => undefined)
    return task
  }

  /** Inclusive fork: history up to the message is copied into a new
   * session and the view switches to it (new FrozenCore snapshot). */
  async fork(messageId: string): Promise<void> {
    const sessionId = this.currentSessionId
    if (!sessionId) return
    this.invalidateConversation('turn/fork')
    const forked = await this.client.sessionFork(sessionId, messageId)
    await this.refreshSessions()
    await this.loadSession(forked.session_id)
  }

  private async refreshSessionView(sessionId: string): Promise<void> {
    const detail = await this.client.sessionGet(sessionId)
    this.projection.applySnapshot({ type: 'session/get', result: detail })
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
      if (work.current_run_id) {
        this.runOwner.set(work.current_run_id, sessionId)
        this.markFreshPrimary(work.current_run_id)
      }
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
  async decidePlan(
    action: 'revise' | 'execute_once' | 'start_goal',
    feedback?: string,
    goal?: { objective: string; max_rounds: number },
  ): Promise<void> {
    const sessionId = this.currentSessionId
    if (!sessionId) throw new Error('no active session')
    const work = this.workBySession.get(sessionId)
    if (!work?.plan.submission_id) throw new Error('no pending plan submission')
    if (action === 'start_goal' && (!goal?.objective?.trim() || !goal.max_rounds)) {
      throw new Error('start_goal requires objective and max_rounds')
    }
    const result = await this.client.planDecide({
      session_id: sessionId,
      request_id: generateMessageId(),
      submission_id: work.plan.submission_id,
      action,
      ...(feedback ? { feedback } : {}),
      ...(goal
        ? { objective: goal.objective.trim(), max_rounds: goal.max_rounds }
        : {}),
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

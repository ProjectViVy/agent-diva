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
  type PlanDecideParams,
  type PlanGetResult,
  type ReviewListParams,
  type ReviewListResult,
  type ReviewRespondParams,
  type SessionMessagesResult,
  type SessionMessage,
  type SessionTodosResult,
  type SessionWorkResult,
  type SessionWorkSubscribeResult,
  type PermissionPreset,
  type TurnAttachment,
  type TurnStartResult,
  type WorkCommitResult,
  type VivySession,
  type WireEvent,
  type ModelSelectParams,
  type ProviderRefreshParams,
  type ProviderUpsertParams,
  type VivyProviderEntryResult,
  type VivyProvidersResult,
  type VivySettingsResult,
  type SettingsUpdateParams,
  type McpUpsertParams,
  type ChannelUpdateParams,
  type CronJobWriteParams,
  type VivyChannelEnvelope,
  type VivyChannelStatus,
  type VivyCronJob,
  type VivyCronListResult,
  type VivyMarketplaceFeatured,
  type VivyMarketplaceInstallResult,
  type VivyMarketplaceSkill,
  type VivyMcpListResult,
  type VivyMcpServer,
  type VivySkillRevision,
  type VivySkillSummary,
  type VivySkillView,
  type VivyTokenUsageSnapshot,
  type TrajectorySession,
  type ChildListResult,
  type DiagnosticQuery,
  type DiagnosticPage,
  type GuiLogBatch,
  type GuiLogAck,
  type VivyToolsResult,
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
  turnStart(sessionId: string, text: string, attachments?: TurnAttachment[]): Promise<TurnStartResult> {
    return this.call(
      'turn/start',
      {
        session_id: sessionId,
        text,
        ...(attachments?.length ? { attachments } : {}),
      },
      { mutation: true },
    )
  }
  /** Mutating preset write; the returned session DTO is the authoritative
   * admitted snapshot — callers must read `permission_preset` back before
   * treating the choice as armed. */
  setSessionPermission(sessionId: string, preset: PermissionPreset): Promise<VivySession> {
    return this.call(
      'session/set_permission',
      { session_id: sessionId, preset },
      { mutation: true },
    )
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

  // --- Source-verified work/review/todo methods (VIVY pinned rpc source) ---

  reviewList(params?: ReviewListParams): Promise<ReviewListResult> {
    return this.call('review/list', params ?? null)
  }
  reviewRespond(params: ReviewRespondParams): Promise<unknown> {
    return this.call('review/respond', params, { mutation: true })
  }
  sessionTodos(sessionId: string): Promise<SessionTodosResult> {
    return this.call('session/todos', { session_id: sessionId })
  }
  sessionTodoUpdate(sessionId: string, todoId: string, status: string): Promise<unknown> {
    return this.call('session/todo/update', { session_id: sessionId, id: todoId, status }, { mutation: true })
  }
  sessionWork(sessionId: string): Promise<SessionWorkResult> {
    return this.call('session/work', { session_id: sessionId })
  }
  sessionWorkSubscribe(sessionId: string, afterSeq = 0): Promise<SessionWorkSubscribeResult> {
    return this.call('session/work/subscribe', { session_id: sessionId, after_seq: afterSeq })
  }
  planGet(sessionId: string, submissionId: string): Promise<PlanGetResult> {
    return this.call('plan/get', { session_id: sessionId, submission_id: submissionId })
  }
  planDecide(params: PlanDecideParams): Promise<WorkCommitResult> {
    return this.call('plan/decide', params, { mutation: true })
  }
  private workMutation(method: string, sessionId: string): Promise<WorkCommitResult> {
    return this.call(
      method,
      { session_id: sessionId, request_id: crypto.randomUUID() },
      { mutation: true },
    )
  }
  planEnter(sessionId: string): Promise<WorkCommitResult> {
    return this.workMutation('plan/enter', sessionId)
  }
  planLeave(sessionId: string): Promise<WorkCommitResult> {
    return this.workMutation('plan/leave', sessionId)
  }
  goalResume(sessionId: string): Promise<WorkCommitResult> {
    return this.workMutation('goal/resume', sessionId)
  }
  goalPause(sessionId: string): Promise<WorkCommitResult> {
    return this.workMutation('goal/pause', sessionId)
  }

  /** Durable session compaction now; busy-session is a -32009 conflict. */
  contextCompact(sessionId: string): Promise<unknown> {
    return this.call('context/compact', { session_id: sessionId }, { mutation: true })
  }

  // --- Source-verified settings surface (internal/rpc/control.go) ---

  settingsGet(): Promise<VivySettingsResult> {
    return this.call('settings/get')
  }
  settingsUpdate(params: SettingsUpdateParams): Promise<VivySettingsResult> {
    return this.call('settings/update', params, { mutation: true })
  }
  settingsProviders(): Promise<VivyProvidersResult> {
    return this.call('settings/providers')
  }
  providerUpsert(params: ProviderUpsertParams): Promise<VivyProviderEntryResult> {
    return this.call('settings/providers/upsert', params, { mutation: true })
  }
  providerDelete(id: string): Promise<{ deleted: boolean }> {
    return this.call('settings/providers/delete', { id }, { mutation: true })
  }
  /** Live GET /models probe + registry persist (OpenAI-compatible only). */
  providerRefresh(params: ProviderRefreshParams): Promise<VivyProviderEntryResult> {
    return this.call('settings/providers/refresh', params, { mutation: true })
  }
  /** Atomic active-model change; -32009 conflict while runs are active. */
  modelSelect(params: ModelSelectParams): Promise<VivyProvidersResult> {
    return this.call('settings/model/select', params, { mutation: true })
  }

  // --- Source-verified tools / MCP / skills / marketplace surface ---

  toolsList(): Promise<VivyToolsResult> {
    return this.call('tools/list')
  }
  /** Replaces the operator tools_enabled overlay with the given set. */
  toolsSetActive(tools: string[]): Promise<unknown> {
    return this.call('tools/set-active', { tools }, { mutation: true })
  }
  mcpList(): Promise<VivyMcpListResult> {
    return this.call('settings/mcp')
  }
  /** Full-replace by name — every field the server keeps must be sent. */
  mcpUpsert(params: McpUpsertParams): Promise<VivyMcpServer> {
    return this.call('settings/mcp/upsert', params, { mutation: true })
  }
  mcpDelete(name: string): Promise<{ deleted: boolean; name: string }> {
    return this.call('settings/mcp/delete', { name }, { mutation: true })
  }
  /** Live tools-listing probe; unreachable server reports state unavailable. */
  mcpProbe(name: string): Promise<VivyMcpServer> {
    return this.call('settings/mcp/probe', { name }, { mutation: true })
  }
  skillsList(): Promise<{ skills: VivySkillSummary[] }> {
    return this.call('skills/list')
  }
  skillGet(name: string): Promise<VivySkillView> {
    return this.call('skills/get', { name })
  }
  /** Compare-and-swap on the content hash; stale hash returns -32009. */
  skillSetEnabled(name: string, enabled: boolean, baseHash: string): Promise<VivySkillSummary> {
    return this.call('skills/set-enabled', { name, enabled, base_hash: baseHash }, { mutation: true })
  }
  /** Pending HITL skill mutations (decisions stay in the run review flow). */
  skillRevisionsList(): Promise<{ revisions: VivySkillRevision[] }> {
    return this.call('skills/revisions/list')
  }
  marketplaceSearch(q: string, limit?: number): Promise<{ skills: VivyMarketplaceSkill[] }> {
    return this.call('skills/marketplace/search', { q, limit })
  }
  marketplaceFeatured(): Promise<VivyMarketplaceFeatured> {
    return this.call('skills/marketplace/featured')
  }
  marketplaceInstall(id: string, mode?: 'create' | 'upgrade'): Promise<VivyMarketplaceInstallResult> {
    return this.call('skills/marketplace/install', { id, mode }, { mutation: true })
  }
  marketplaceCheck(name: string): Promise<{ name: string; status: string; marketplace_id?: string; snapshot_hash?: string }> {
    return this.call('skills/marketplace/check', { name })
  }

  // ---- slice C: channels / cron ----
  channelInspect(): Promise<VivyChannelStatus[]> {
    return this.call('channel/inspect')
  }
  channelGet(name: string): Promise<VivyChannelEnvelope> {
    return this.call('channel/get', { name })
  }
  channelUpdate(params: ChannelUpdateParams): Promise<VivyChannelEnvelope> {
    return this.call('channel/update', params, { mutation: true })
  }
  cronList(): Promise<VivyCronListResult> {
    return this.call('cron/list')
  }
  cronCreate(params: CronJobWriteParams): Promise<{ job: VivyCronJob }> {
    return this.call('cron/create', params, { mutation: true })
  }
  cronUpdate(id: string, params: CronJobWriteParams): Promise<{ job: VivyCronJob }> {
    return this.call('cron/update', { id, ...params }, { mutation: true })
  }
  cronDelete(id: string): Promise<{ deleted: boolean }> {
    return this.call('cron/delete', { id }, { mutation: true })
  }
  cronTrigger(id: string): Promise<{ job: VivyCronJob }> {
    return this.call('cron/trigger', { id }, { mutation: true })
  }
  statsTokens(params: { period?: string; tz_offset_minutes?: number; session_limit?: number } = {}): Promise<VivyTokenUsageSnapshot> {
    return this.call<VivyTokenUsageSnapshot>('stats/tokens', params)
  }
  sessionTrajectory(sessionId: string, limit?: number): Promise<TrajectorySession> {
    return this.call<TrajectorySession>('trajectory/session', {
      session_id: sessionId,
      ...(limit ? { limit } : {}),
    })
  }
  childList(parentRunId: string): Promise<ChildListResult> {
    return this.call<ChildListResult>('child/list', { parent_run_id: parentRunId })
  }
  diagnosticsLogs(query: DiagnosticQuery): Promise<DiagnosticPage> {
    return this.call<DiagnosticPage>('diagnostics/logs', query)
  }
  diagnosticsGuiAppend(batch: GuiLogBatch): Promise<GuiLogAck> {
    return this.call<GuiLogAck>('diagnostics/gui/append', batch, { mutation: true })
  }

  cronStop(id: string): Promise<{ stopped: boolean }> {
    return this.call('cron/stop', { id }, { mutation: true })
  }
}

/** Last-messages helper kept generic for projection consumers. */
export type { SessionMessage }

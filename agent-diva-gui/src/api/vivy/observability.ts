/**
 * Observability snapshot requests (OBS-06). One typed snapshot call —
 * `stats/tokens` — keeps the console honest about what it displays: the
 * v2 coverage record partitions observed attempts, `request_count`
 * stays "usage reports", and `cost_known:false` means the cost field is
 * a zero placeholder, never a free bill.
 *
 * OBS-07 adds `trajectory/session`; OBS-08 adds `diagnostics/logs` and
 * `diagnostics/gui/append`. Client-side GUI batches are capped at 50
 * records / 256 KiB serialized — well under the producer's 500-record /
 * 4 MiB ceiling — so the ABI keeps headroom. A failed append surfaces as
 * an error; callers never parse accepted-prefix counts out of error
 * text and never blindly retry an ambiguous write.
 */
import type { VivyClient } from './client'
import type {
  DiagnosticPage,
  DiagnosticQuery,
  GuiLogAck,
  GuiLogBatch,
  TrajectorySession,
  VivyTokenUsageSnapshot,
} from './contracts'

export interface TokenUsageParams {
  period?: string
  tz_offset_minutes?: number
  session_limit?: number
}

/** `stats/tokens` request on the given client — no implicit singleton. */
export function getTokenUsage(
  client: VivyClient,
  params: TokenUsageParams = {},
): Promise<VivyTokenUsageSnapshot> {
  return client.statsTokens(params)
}

/** `trajectory/session` request — bounded (default 20, max 50 runs). */
export function getSessionTrajectory(
  client: VivyClient,
  sessionId: string,
  limit?: number,
): Promise<TrajectorySession> {
  return client.sessionTrajectory(sessionId, limit)
}

/** `diagnostics/logs` request — source/date/after/limit/level/query
 * verbatim; gap/has_more/truncated stay producer-owned. */
export function readDiagnostics(
  client: VivyClient,
  query: DiagnosticQuery,
): Promise<DiagnosticPage> {
  return client.diagnosticsLogs(query)
}

export const GUI_BATCH_MAX_RECORDS = 50
export const GUI_BATCH_MAX_BYTES = 256 * 1024

/** `diagnostics/gui/append` with the client-side batch guard. Rejects
 * oversized batches instead of silently splitting them — chunking is
 * the caller's (recorder's) job. */
export function appendGuiLogs(
  client: VivyClient,
  batch: GuiLogBatch,
): Promise<GuiLogAck> {
  if (batch.records.length > GUI_BATCH_MAX_RECORDS) {
    return Promise.reject(
      new Error(`gui log batch too large: ${batch.records.length} records > ${GUI_BATCH_MAX_RECORDS}`),
    )
  }
  const serialized = JSON.stringify(batch)
  if (serialized.length > GUI_BATCH_MAX_BYTES) {
    return Promise.reject(
      new Error(`gui log batch too large: ${serialized.length} bytes > ${GUI_BATCH_MAX_BYTES}`),
    )
  }
  return client.diagnosticsGuiAppend(batch)
}

/**
 * Observability snapshot requests (OBS-06). One typed snapshot call —
 * `stats/tokens` — keeps the console honest about what it displays: the
 * v2 coverage record partitions observed attempts, `request_count`
 * stays "usage reports", and `cost_known:false` means the cost field is
 * a zero placeholder, never a free bill.
 */
import type { VivyClient } from './client'
import type { TrajectorySession, VivyTokenUsageSnapshot } from './contracts'

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

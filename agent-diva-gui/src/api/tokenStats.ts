import { vivyClient } from './vivy/instance';
import type { VivyTokenUsageSnapshot } from './vivy/contracts';

export type TimeRangePeriod = '1d' | '3d' | '1w' | '1m' | '6m' | '1y';
export type TokenUsageSnapshot = VivyTokenUsageSnapshot;

/**
 * Fetch the VIVY `stats/tokens` snapshot: totals, model shares, provider
 * groups, timeline and per-session usage in a single call.
 */
export function getTokenStats(
  period: TimeRangePeriod = '1d',
  sessionLimit = 20,
): Promise<TokenUsageSnapshot> {
  return vivyClient.statsTokens({
    period,
    tz_offset_minutes: new Date().getTimezoneOffset(),
    session_limit: sessionLimit,
  });
}

/**
 * Format token count for display
 */
export function formatTokenCount(count: number): string {
  if (count >= 1_000_000) {
    return `${(count / 1_000_000).toFixed(2)}M`;
  }
  if (count >= 1_000) {
    return `${(count / 1_000).toFixed(1)}K`;
  }
  return count.toString();
}

/**
 * Format cost for display
 */
export function formatCost(cost: number): string {
  if (cost < 0.01) {
    return `$${cost.toFixed(4)}`;
  }
  return `$${cost.toFixed(2)}`;
}

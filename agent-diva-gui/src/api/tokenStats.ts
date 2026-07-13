// Token usage statistics API

import { invoke } from "@tauri-apps/api/core";

export interface UsageTotal {
  total_input: number;
  total_output: number;
  total_tokens: number;
  total_cache_creation: number;
  total_cache_read: number;
  request_count: number;
  total_cost: number;
}

export interface UsageSummary {
  group_key: string;
  total_input: number;
  total_output: number;
  total_tokens: number;
  total_cache_creation: number;
  total_cache_read: number;
  request_count: number;
  total_cost: number;
}

export interface TimelinePoint {
  time_bucket: string;
  total_input: number;
  total_output: number;
  total_tokens: number;
  request_count: number;
}

export interface SessionUsage {
  session_id: string;
  total_input: number;
  total_output: number;
  total_tokens: number;
  request_count: number;
  total_cost: number;
  primary_model: string;
  channel: string | null;
  last_activity: string;
}

export interface ModelDistribution {
  model: string;
  percentage: number;
  total_tokens: number;
}

export interface InMemoryStats {
  total_tokens: number;
  total_input: number;
  total_output: number;
  request_count: number;
  total_cost: number;
}

export type TimeRangePeriod = '1h' | '1d' | '3d' | '1w' | '1m' | '6m' | '1y';
export type GroupByDimension = 'endpoint' | 'model' | 'operation_type' | 'session' | 'channel';
export type TimeIntervalType = 'minute' | 'hour' | 'day';

/**
 * Get total token usage statistics
 */
export async function getTokenUsageTotal(period: TimeRangePeriod = '1h'): Promise<UsageTotal> {
  return invoke<UsageTotal>('get_token_usage_total', { period });
}

/**
 * Get token usage summary grouped by dimension
 */
export async function getTokenUsageSummary(
  period: TimeRangePeriod = '1h',
  groupBy: GroupByDimension = 'endpoint'
): Promise<UsageSummary[]> {
  return invoke<UsageSummary[]>('get_token_usage_summary', {
    period,
    groupBy
  });
}

/**
 * Get token usage timeline for charting
 */
export async function getTokenUsageTimeline(
  period: TimeRangePeriod = '1h',
  interval?: TimeIntervalType
): Promise<TimelinePoint[]> {
  return invoke<TimelinePoint[]>('get_token_usage_timeline', {
    period,
    interval: interval || (period === '1h' ? 'minute' : (period === '1d' ? 'hour' : 'day'))
  });
}

/**
 * Get session-level token usage details
 */
export async function getTokenUsageSessions(
  period: TimeRangePeriod = '1h',
  limit: number = 20
): Promise<SessionUsage[]> {
  return invoke<SessionUsage[]>('get_token_usage_sessions', {
    period,
    limit
  });
}

/**
 * Get model distribution percentages
 */
export async function getTokenUsageModels(
  period: TimeRangePeriod = '1h'
): Promise<ModelDistribution[]> {
  return invoke<ModelDistribution[]>('get_token_usage_models', {
    period
  });
}

/**
 * Get real-time in-memory statistics
 */
export async function getTokenUsageRealtime(): Promise<InMemoryStats> {
  return invoke<InMemoryStats>('get_token_usage_realtime');
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

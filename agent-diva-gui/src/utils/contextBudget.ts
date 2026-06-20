import type { BudgetConfigShape } from '../types/toolsConfig';

export interface BudgetStatus {
  history_estimated: number;
  history_budget: number;
  pressure_ratio: number;
  should_compact: boolean;
}

export interface BudgetMessageLike {
  role: 'user' | 'agent' | 'system' | 'tool';
  content?: string;
  reasoning?: string;
  rawMeta?: Record<string, unknown>;
  fromHistory?: boolean;
}

const DEFAULT_BUDGET: BudgetConfigShape = {
  max_tokens: 180000,
  system_budget_ratio: 0.15,
  compact_threshold_ratio: 0.8,
  keep_recent_count: 10,
};

const HISTORY_WINDOW_MESSAGES = 50;

export function normalizeBudgetConfig(
  config?: Partial<BudgetConfigShape> | null
): BudgetConfigShape {
  return {
    max_tokens: Number(config?.max_tokens) > 0
      ? Number(config?.max_tokens)
      : DEFAULT_BUDGET.max_tokens,
    system_budget_ratio: typeof config?.system_budget_ratio === 'number'
      ? config.system_budget_ratio
      : DEFAULT_BUDGET.system_budget_ratio,
    compact_threshold_ratio: typeof config?.compact_threshold_ratio === 'number'
      ? config.compact_threshold_ratio
      : DEFAULT_BUDGET.compact_threshold_ratio,
    keep_recent_count: Number(config?.keep_recent_count) > 0
      ? Number(config?.keep_recent_count)
      : DEFAULT_BUDGET.keep_recent_count,
  };
}

export function estimateTokens(text: string): number {
  if (!text) return 0;
  return Math.ceil(Array.from(text).length / 3);
}

function estimateExtraTokens(values: unknown): number {
  if (!Array.isArray(values)) return 0;
  return values.reduce((total, value) => {
    try {
      return total + estimateTokens(JSON.stringify(value));
    } catch {
      return total + estimateTokens(String(value));
    }
  }, 0);
}

function estimateMessageTokens(message: BudgetMessageLike): number {
  let total = estimateTokens(message.content || '');
  if (message.reasoning) {
    total += estimateTokens(message.reasoning);
  }
  if (message.rawMeta?.tool_calls) {
    total += estimateExtraTokens(message.rawMeta.tool_calls);
  }
  if (message.rawMeta?.thinking_blocks) {
    total += estimateExtraTokens(message.rawMeta.thinking_blocks);
  }
  return total;
}

function normalizeMessages(messages: BudgetMessageLike[]): BudgetMessageLike[] {
  const relevant = messages.filter((message) =>
    message.role === 'user' || message.role === 'agent' || message.role === 'tool'
  );

  if (
    relevant.length === 1 &&
    relevant[0].role === 'agent' &&
    !relevant[0].fromHistory
  ) {
    return [];
  }

  const start = Math.max(0, relevant.length - HISTORY_WINDOW_MESSAGES);
  let history = relevant.slice(start);
  const firstUserIndex = history.findIndex((message) => message.role === 'user');
  if (firstUserIndex > 0) {
    history = history.slice(firstUserIndex);
  }
  return history;
}

export function computeBudgetStatus(
  messages: BudgetMessageLike[],
  config?: Partial<BudgetConfigShape> | null
): BudgetStatus {
  const budget = normalizeBudgetConfig(config);
  const history = normalizeMessages(messages);
  const history_estimated = history.reduce(
    (total, message) => total + estimateMessageTokens(message),
    0
  );

  const systemBudget = Math.floor(budget.max_tokens * budget.system_budget_ratio);
  const history_budget = Math.max(0, budget.max_tokens - systemBudget);
  const compactThreshold = Math.floor(history_budget * budget.compact_threshold_ratio);
  const pressure_ratio = history_budget > 0
    ? history_estimated / history_budget
    : (history_estimated > 0 ? Number.POSITIVE_INFINITY : 0);

  return {
    history_estimated,
    history_budget,
    pressure_ratio,
    should_compact: history_estimated > compactThreshold && history_estimated > 0,
  };
}

export function budgetPressurePercent(status: BudgetStatus): number {
  return Math.round(status.pressure_ratio * 100);
}

import { describe, expect, it } from 'vitest';
import { budgetPressurePercent, computeBudgetStatus } from './contextBudget';

describe('computeBudgetStatus', () => {
  it('treats the local welcome placeholder as an empty session', () => {
    const status = computeBudgetStatus([
      {
        role: 'agent',
        content: 'welcome',
        fromHistory: false,
      },
    ]);

    expect(status.history_estimated).toBe(0);
    expect(status.should_compact).toBe(false);
    expect(budgetPressurePercent(status)).toBe(0);
  });

  it('uses the configured budget for current session history', () => {
    const status = computeBudgetStatus(
      [
        { role: 'user', content: 'x'.repeat(3600) },
        { role: 'agent', content: 'y'.repeat(3600), reasoning: 'z'.repeat(1200) },
      ],
      {
        max_tokens: 3000,
        system_budget_ratio: 0,
        compact_threshold_ratio: 0.8,
        keep_recent_count: 10,
      }
    );

    expect(status.history_budget).toBe(3000);
    expect(status.history_estimated).toBeGreaterThan(1000);
    expect(status.should_compact).toBe(true);
    expect(budgetPressurePercent(status)).toBeGreaterThan(80);
  });
});

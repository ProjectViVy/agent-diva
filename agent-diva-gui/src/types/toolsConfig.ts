import type { MentleToolConfigShape } from '../api/desktop';

export interface BudgetConfigShape {
  max_tokens: number;
  system_budget_ratio: number;
  compact_threshold_ratio: number;
  keep_recent_count: number;
}

export interface ToolsConfigShape {
  web: {
    search: {
      provider: string;
      enabled: boolean;
      api_key: string;
      max_results: number;
    };
    fetch: {
      enabled: boolean;
    };
  };
  mentle: MentleToolConfigShape;
  budget: BudgetConfigShape;
}

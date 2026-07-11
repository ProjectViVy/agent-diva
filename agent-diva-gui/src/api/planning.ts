/**
 * Planning domain DTOs matching the Rust backend types in planning_service.rs.
 */

export interface PlanSummary {
  id: string;
  title: string;
  goal: string;
  phase: string;
  status: string;
  todo_count: number;
  todo_completed: number;
  is_active: boolean;
}

export interface PlanDetail {
  id: string;
  revision?: number | null;
  title: string;
  goal: string;
  phase: string;
  status: string;
  strategy: string | null;
  summary?: string;
  markdown?: string;
  assumptions: string[];
  risks: string[];
  open_questions: string[];
  verification_verdict: string | null;
  steps: StepDetail[];
  todos: TodoDetail[];
  created_at: string;
  updated_at: string;
}

export interface StepDetail {
  id: string;
  plan_id: string;
  ordinal: number;
  title: string;
  rationale: string | null;
  expected_output: string | null;
  status: string;
  evidence_ref: string | null;
  created_at: string;
  updated_at: string;
}

export interface TodoDetail {
  id: string;
  plan_step_id: string | null;
  title: string;
  detail: string | null;
  status: string; // 'in_progress' | 'pending' | 'blocked' | 'completed'
  priority: string; // 'high' | 'low' | 'medium'
  evidence_ref: string | null;
  block_reason: string | null;
  updated_at: string;
}

export interface PlanRuntimeStep {
  id: string;
  ordinal: number;
  title: string;
  rationale: string | null;
  expected_output: string | null;
  status: string;
}

export interface PlanRuntimeTodo {
  id: string;
  plan_step_id: string | null;
  title: string;
  detail: string | null;
  status: string;
  priority: string;
  evidence_ref: string | null;
  block_reason: string | null;
  updated_at: string;
}

export interface PlanRuntimeState {
  plan_id: string;
  revision?: number | null;
  title: string;
  goal: string;
  phase: string;
  status: string;
  strategy: string | null;
  summary: string;
  markdown?: string;
  /** Soft completeness warnings (missing sections, etc.); do not block approval. */
  validation_issues?: string[];
  steps: PlanRuntimeStep[];
  todos: PlanRuntimeTodo[];
  created_at: string;
  updated_at: string;
}

const PLAN_REQUIRED_SECTIONS = ['目标', '范围', '计划步骤', '风险与假设', '验证方法'] as const;

/** Soft completeness hints for plan report markdown (mirrors core report_validation_issues). */
export function planReportValidationIssues(markdown: string | undefined | null): string[] {
  if (!markdown || !markdown.trim()) {
    return ['计划报告正文为空'];
  }
  const issues: string[] = [];
  const lines = markdown.split(/\r?\n/);
  if (!lines.some((line) => line.trimStart().startsWith('# '))) {
    issues.push('计划报告缺少标题');
  }
  for (const section of PLAN_REQUIRED_SECTIONS) {
    const heading = `## ${section}`;
    if (!lines.some((line) => line.trim() === heading)) {
      issues.push(`计划报告缺少章节：${section}`);
    }
  }
  return issues;
}

export interface PlanApprovalReceipt {
  plan_id: string;
  revision: number;
  approved_at: string;
  todo_policy: 'Never' | 'Optional' | 'Always';
  todos_materialized: boolean;
}

export interface PlanApprovalResult {
  plan: PlanRuntimeState;
  receipt: PlanApprovalReceipt;
}

export interface PlanSnapshotMetadata {
  kind: 'plan_snapshot';
  version: number;
  plan: PlanRuntimeState;
}

export interface PlanStreamEvent {
  plan: PlanRuntimeState;
  todo?: PlanRuntimeTodo | null;
}

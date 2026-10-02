/**
 * Planning domain DTOs matching the Rust backend types in planning_service.rs.
 */

import type { SessionTodo, SessionWorkResult } from './vivy/contracts';

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
  /** Completeness failures that block approval until the plan is revised. */
  validation_issues?: string[];
  steps: PlanRuntimeStep[];
  todos: PlanRuntimeTodo[];
  created_at: string;
  updated_at: string;
  execution_id?: string | null;
  initialization_status?: 'Pending' | 'Ready' | 'Blocked';
  initialization_error?: string | null;
}

const PLAN_REQUIRED_SECTION_ALIASES = [
  ['目标', 'Goal', 'Objective'],
  ['范围', 'Scope'],
  ['计划步骤', 'Implementation Steps', 'Plan Steps', 'Steps'],
  ['风险与假设', 'Risks and Assumptions', 'Risks & Assumptions', 'Risks', 'Assumptions'],
  ['验证方法', 'Verification', 'Verification Method', 'Test Plan'],
] as const;

/** Completeness checks for plan report markdown (mirrors the core submission gate). */
export function planReportValidationIssues(markdown: string | undefined | null): string[] {
  if (!markdown || !markdown.trim()) {
    return ['计划报告正文为空'];
  }
  const issues: string[] = [];
  const lines = markdown.split(/\r?\n/);
  if (!lines.some((line) => line.trimStart().startsWith('# '))) {
    issues.push('计划报告缺少标题');
  }
  for (const aliases of PLAN_REQUIRED_SECTION_ALIASES) {
    if (!aliases.some((section) => lines.some((line) => line.trim() === `## ${section}`))) {
      issues.push(`计划报告缺少章节：${aliases[0]}`);
    }
  }
  return issues;
}

// ---------------------------------------------------------------------------
// VIVY work adapter (DN-2): session/work + session/todos map onto the
// drawer-facing PlanRuntimeState. Field vocabulary verified against the pinned
// VIVY work_control source (activation "armed"|"disarmed", plan review_status
// none|pending|accepted|rejected|cancelled).
// ---------------------------------------------------------------------------

function asRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

const TODO_STATUS_TO_PLAN: Record<string, PlanRuntimeTodo['status']> = {
  pending: 'pending',
  in_progress: 'running',
  completed: 'completed',
  cancelled: 'failed',
};

/** Map one VIVY session todo to a plan-runtime todo row. */
export function todoToPlanRuntime(todo: SessionTodo): PlanRuntimeTodo {
  return {
    id: todo.id,
    plan_step_id: null,
    title: todo.subject,
    detail: todo.description ?? null,
    status: TODO_STATUS_TO_PLAN[todo.status] ?? 'pending',
    priority: 'normal',
    evidence_ref: null,
    block_reason: todo.blocked_by.length > 0 ? `blocked by ${todo.blocked_by.join(', ')}` : null,
    updated_at: '',
  };
}

/**
 * Map a session/work snapshot to PlanRuntimeState. Returns null when no plan
 * is active — the caller clears the plan surfaces rather than showing a
 * fabricated idle card.
 */
export function workToPlanRuntime(work: SessionWorkResult): PlanRuntimeState | null {
  if (!work.plan.active) return null;
  const goal = asRecord(work.goal);
  const phase = work.plan.review_status === 'pending'
    ? 'AwaitingApproval'
    : work.activation === 'armed'
      ? 'Execute'
      : work.plan.review_status === 'accepted'
        ? 'Execute'
        : 'Draft';
  return {
    plan_id: work.plan.submission_id || 'work',
    revision: typeof work.version === 'number' ? work.version : undefined,
    title: (goal && typeof goal.objective === 'string' && goal.objective) || 'Plan',
    goal: (goal && typeof goal.objective === 'string' ? goal.objective : '') || '',
    phase,
    status: work.plan.review_status === 'pending' ? 'AwaitingApproval' : phase,
    strategy: work.plan.feedback || null,
    summary: work.plan.feedback || '',
    markdown: work.plan.markdown || '',
    validation_issues: [],
    steps: [],
    todos: [],
    created_at: '',
    updated_at: '',
  };
}

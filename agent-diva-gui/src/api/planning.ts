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

/** Placeholder shown when a tagged plan body was demuxed into the approval card. */
export const PLAN_DEMUX_PLACEHOLDER = '已生成计划报告，请在下方审批卡片中查看并批准。';

/** Generic titles that should yield to a better H1 / first content line. */
const GENERIC_PLAN_TITLES = new Set([
  'plan',
  'plan report',
  'markdown plan report',
  '计划',
  '计划报告',
  'formal plan',
  'formal plan report',
]);

function isGenericPlanTitle(title: string | null | undefined): boolean {
  const t = (title || '').trim().toLowerCase();
  if (!t) return true;
  return GENERIC_PLAN_TITLES.has(t);
}

/** First markdown H1 (`# title`), ignoring `##` section headings. */
export function extractPlanTitleFromMarkdown(markdown: string | null | undefined): string | null {
  if (!markdown) return null;
  for (const raw of markdown.split(/\r?\n/)) {
    const line = raw.trim();
    if (line.startsWith('# ') && !line.startsWith('## ')) {
      const title = line.slice(2).trim();
      return title || null;
    }
  }
  return null;
}

/** First non-heading, non-empty body line (often the bare freeform title). */
export function extractPlanLeadLine(markdown: string | null | undefined): string | null {
  if (!markdown) return null;
  for (const raw of markdown.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) continue;
    // Skip pure section labels without body.
    if (['目标', '范围', '计划步骤', '风险与假设', '验证方法', 'Goal', 'Scope', 'Steps'].includes(line)) {
      continue;
    }
    return line;
  }
  return null;
}

/**
 * Resolve a user-facing plan title. Prefer real content over generic fallbacks
 * like "Plan" / "计划报告" so cards do not show "计划 · Plan · Plan".
 */
export function resolvePlanDisplayTitle(plan: {
  title?: string | null;
  goal?: string | null;
  markdown?: string | null;
  summary?: string | null;
  strategy?: string | null;
}): string {
  const body = plan.markdown || plan.summary || plan.strategy || '';
  const fromH1 = extractPlanTitleFromMarkdown(body);
  const fromLead = extractPlanLeadLine(body);
  const candidates = [plan.title, fromH1, fromLead, plan.goal];
  for (const candidate of candidates) {
    const cleaned = (candidate || '').replace(/\uFFFD/g, '').trim();
    if (cleaned && !isGenericPlanTitle(cleaned)) {
      return cleaned;
    }
  }
  for (const candidate of candidates) {
    const cleaned = (candidate || '').replace(/\uFFFD/g, '').trim();
    if (cleaned) return cleaned;
  }
  return '计划报告';
}

/**
 * Strip the first complete line-oriented `<proposed_plan>…</proposed_plan>` block.
 * Mirrors `agent_diva_core::planning::strip_proposed_plan_block`.
 * Returns the original text when no complete block is present.
 */
export function stripProposedPlanBlock(text: string): string {
  const lines = text.split(/\r?\n/);
  const start = lines.findIndex((line) => line.trim() === '<proposed_plan>');
  if (start < 0) return text;
  const endRel = lines.slice(start + 1).findIndex((line) => line.trim() === '</proposed_plan>');
  if (endRel < 0) return text;
  const end = start + 1 + endRel;
  const kept = [...lines.slice(0, start), ...lines.slice(end + 1)];
  const collapsed: string[] = [];
  let blankRun = 0;
  for (const line of kept) {
    if (line.trim() === '') {
      blankRun += 1;
      if (blankRun > 1) continue;
    } else {
      blankRun = 0;
    }
    collapsed.push(line);
  }
  return collapsed.join('\n').trim();
}

/**
 * Prefer authoritative final SSE content over accumulated stream deltas.
 * Backend may demux `<proposed_plan>` and replace the bubble with a short notice.
 */
export function applyFinalAssistantContent(
  streamedContent: string,
  finalContent: string | null | undefined,
): string {
  if (typeof finalContent === 'string' && finalContent.length > 0) {
    return finalContent;
  }
  return streamedContent || (finalContent ?? '');
}

/**
 * Prefer full plan-document markdown for history/approval cards.
 * Rebuilds a readable document when older snapshots only stored `title: goal`.
 */
export function planDocumentMarkdown(plan: {
  title?: string;
  goal?: string;
  markdown?: string | null;
  summary?: string | null;
  strategy?: string | null;
  steps?: Array<{ title: string; status?: string; rationale?: string | null }>;
}): string {
  const candidates = [plan.markdown, plan.summary, plan.strategy]
    .map((value) => (typeof value === 'string' ? value.trim() : ''))
    .filter(Boolean);
  for (const candidate of candidates) {
    // Full docs have headings or multiple lines; reject the old "title: goal" line.
    if (candidate.includes('\n') || candidate.startsWith('#') || candidate.startsWith('##')) {
      return candidate;
    }
    if (plan.goal && candidate === `${plan.title || ''}: ${plan.goal}`.trim()) {
      continue;
    }
    // Strategy-only body still beats a stub.
    if (candidate.length > (plan.goal?.length ?? 0) + 24) {
      return candidate;
    }
  }

  const title = resolvePlanDisplayTitle(plan);
  const lines: string[] = [`# ${title}`, ''];
  if (plan.goal?.trim() && plan.goal.trim() !== title) {
    lines.push('## 目标', plan.goal.trim(), '');
  }
  if (
    plan.strategy?.trim()
    && plan.strategy.trim() !== plan.goal?.trim()
    && !plan.strategy.includes('\n')
    && !plan.strategy.trim().startsWith('#')
  ) {
    lines.push('## Strategy', plan.strategy.trim(), '');
  }
  if (plan.steps?.length) {
    lines.push('## 计划步骤', '');
    plan.steps.forEach((step, index) => {
      const status = step.status ? ` [${step.status}]` : '';
      lines.push(`${index + 1}. **${step.title}**${status}`);
      if (step.rationale?.trim()) {
        lines.push(`   ${step.rationale.trim()}`);
      }
    });
    lines.push('');
  }
  return lines.join('\n').trim() + '\n';
}

type PlanSnapshotLike = {
  plan_id?: string;
  title?: string;
  goal?: string;
  phase?: string;
  markdown?: string | null;
  summary?: string | null;
  strategy?: string | null;
  steps?: Array<{ title: string; status?: string; rationale?: string | null }>;
  todos?: unknown[];
};

/**
 * Lifecycle phases that deserve a durable "历史计划" card in chat history.
 * Draft Plan/Explore snapshots were often leaked from the global active-plan
 * singleton into unrelated sessions — hide those from the timeline.
 */
export function isPlanHistoryLifecyclePhase(phase?: string | null): boolean {
  if (!phase) return false;
  const normalized = phase.trim().toLowerCase();
  return [
    'awaitingapproval',
    'execute',
    'verify',
    'completed',
    'failed',
    'partial',
    'approved',
    'inprogress',
  ].includes(normalized);
}

/**
 * True when a history card has real plan body (not a "Plan" shell with empty doc).
 * Used to suppress junk plan_snapshot rows that still land in session history.
 */
export function isRenderablePlanSnapshot(plan: PlanSnapshotLike | null | undefined): boolean {
  if (!plan) return false;
  // Stale global draft dumps (phase Plan/Explore) are not chat history events.
  if (!isPlanHistoryLifecyclePhase(plan.phase)) {
    return false;
  }
  const body = planDocumentMarkdown(plan).trim();
  if (!body) return false;

  const lines = body
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  const contentLines = lines.filter((line) => !line.startsWith('#'));
  const hasSections = body.includes('##');
  const hasSteps = (plan.steps?.length ?? 0) > 0;
  const hasTodos = (plan.todos?.length ?? 0) > 0;
  const title = resolvePlanDisplayTitle(plan);

  // Pure H1 shell: "# Plan" / "# 计划报告" with no body.
  if (contentLines.length === 0 && !hasSteps && !hasTodos) {
    return false;
  }
  // Generic title + only a one-line goal and no structure → junk / incomplete stub.
  if (isGenericPlanTitle(title) && !hasSections && contentLines.length < 2 && !hasSteps) {
    return false;
  }
  return true;
}

/** Higher is better — prefer full report markdown over later thin Execute stubs. */
export function planSnapshotQuality(plan: PlanSnapshotLike | null | undefined): number {
  if (!plan || !isRenderablePlanSnapshot(plan)) return -1;
  const body = planDocumentMarkdown(plan);
  let score = Math.min(body.length, 4000);
  if (body.includes('##')) score += 800;
  if (!isGenericPlanTitle(resolvePlanDisplayTitle(plan))) score += 300;
  if ((plan.steps?.length ?? 0) > 0) score += 100;
  if ((plan.todos?.length ?? 0) > 0) score += 50;
  // Prefer approval-time full report over a later empty Execute card.
  const phase = (plan.phase || '').toLowerCase();
  if (phase === 'awaitingapproval') score += 40;
  return score;
}

/**
 * Collapse plan_snapshot history cards:
 * - drop non-renderable junk ("历史计划 / Plan" shells)
 * - keep the highest-quality card per plan_id (not merely the latest stub)
 */
export function collapsePlanHistoryMessages<T extends {
  planSnapshot?: PlanSnapshotLike | null;
}>(messages: T[]): T[] {
  const bestIndexByKey = new Map<string, number>();
  messages.forEach((msg, index) => {
    const snap = msg.planSnapshot;
    if (!snap || !isRenderablePlanSnapshot(snap)) return;
    const key = (snap.plan_id || resolvePlanDisplayTitle(snap) || `idx-${index}`).trim()
      || `idx-${index}`;
    const prev = bestIndexByKey.get(key);
    if (prev == null) {
      bestIndexByKey.set(key, index);
      return;
    }
    const prevScore = planSnapshotQuality(messages[prev].planSnapshot);
    const nextScore = planSnapshotQuality(snap);
    // Prefer quality; on a tie keep the later message (more recent phase).
    if (nextScore >= prevScore) {
      bestIndexByKey.set(key, index);
    }
  });
  if (bestIndexByKey.size === 0) {
    // No renderable snapshots — strip all plan_snapshot messages.
    return messages.filter((msg) => !msg.planSnapshot);
  }

  const keep = new Set(bestIndexByKey.values());
  return messages.filter((msg, index) => {
    if (!msg.planSnapshot) return true;
    return keep.has(index);
  });
}

/**
 * Sanitize assistant history for display: drop demuxed plan tags so reloads
 * do not re-show the raw protocol block next to the approval card.
 */
export function sanitizeAssistantPlanContent(content: string): string {
  if (!content.includes('<proposed_plan>')) {
    return content;
  }
  const stripped = stripProposedPlanBlock(content);
  if (stripped.trim().length === 0) {
    return PLAN_DEMUX_PLACEHOLDER;
  }
  // Incomplete / unclosed tags — still hide the protocol marker from the bubble.
  if (stripped.includes('<proposed_plan>') || stripped.includes('</proposed_plan>')) {
    return stripped
      .split(/\r?\n/)
      .filter((line) => {
        const t = line.trim();
        return t !== '<proposed_plan>' && t !== '</proposed_plan>';
      })
      .join('\n')
      .trim() || PLAN_DEMUX_PLACEHOLDER;
  }
  return stripped;
}

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

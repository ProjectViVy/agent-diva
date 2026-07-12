/**
 * Demux chat message content that embeds Codex-style `<proposed_plan>` blocks.
 * Mirrors the line-oriented rules in agent-diva-core `extract_proposed_plan`.
 */

export interface ProposedPlanParts {
  /** Text outside the plan tags (before the opening tag). */
  preface: string;
  /** Inner plan body (between tags), possibly soft-normalized for display. */
  planMarkdown: string;
  /** Text after the closing tag. */
  epilogue: string;
  /** True when a complete non-empty plan block was found. */
  hasPlan: boolean;
}

const SECTION_ALIASES: ReadonlyArray<readonly [string, string]> = [
  ['目标', '目标'],
  ['范围', '范围'],
  ['计划步骤', '计划步骤'],
  ['步骤', '计划步骤'],
  ['实现步骤', '计划步骤'],
  ['风险与假设', '风险与假设'],
  ['风险', '风险与假设'],
  ['假设', '风险与假设'],
  ['验证方法', '验证方法'],
  ['验证', '验证方法'],
  ['测试计划', '验证方法'],
];

/** Line-oriented extract of the first complete `<proposed_plan>` block. */
export function demuxProposedPlan(content: string | null | undefined): ProposedPlanParts {
  if (!content) {
    return { preface: '', planMarkdown: '', epilogue: '', hasPlan: false };
  }
  const lines = content.split(/\r?\n/);
  const start = lines.findIndex((line) => line.trim() === '<proposed_plan>');
  if (start < 0) {
    return { preface: content, planMarkdown: '', epilogue: '', hasPlan: false };
  }
  const endRel = lines.slice(start + 1).findIndex((line) => line.trim() === '</proposed_plan>');
  if (endRel < 0) {
    // Incomplete while streaming — keep raw content for normal markdown.
    return { preface: content, planMarkdown: '', epilogue: '', hasPlan: false };
  }
  const end = start + 1 + endRel;
  const preface = lines.slice(0, start).join('\n').replace(/\s+$/u, '');
  const inner = lines.slice(start + 1, end).join('\n');
  const epilogue = lines
    .slice(end + 1)
    .join('\n')
    .replace(/^\s+/u, '');
  if (!inner.trim()) {
    return { preface: content, planMarkdown: '', epilogue: '', hasPlan: false };
  }
  return {
    preface,
    planMarkdown: formatPlanBodyForDisplay(inner),
    epilogue,
    hasPlan: true,
  };
}

/**
 * Soft-normalize freeform plan bodies so PlanDocument markdown looks intentional:
 * - first non-heading content line → H1 when missing
 * - bare section labels (目标/范围/…) → `##` headings
 */
export function formatPlanBodyForDisplay(body: string): string {
  const cleaned = body.replace(/\uFFFD/g, '');
  const lines = cleaned.split(/\r?\n/);
  let hasH1 = lines.some((line) => {
    const t = line.trimStart();
    return t.startsWith('# ') && !t.startsWith('## ');
  });

  const out: string[] = [];
  for (const raw of lines) {
    const trimmed = raw.trim();
    if (!trimmed) {
      out.push('');
      continue;
    }

    const section = matchSectionLabel(trimmed);
    if (section) {
      out.push(`## ${section}`);
      continue;
    }

    if (!hasH1 && !trimmed.startsWith('## ') && !trimmed.startsWith('# ')) {
      const title = trimmed.replace(/^#+\s*/u, '').trim() || '计划报告';
      out.push(`# ${title}`);
      hasH1 = true;
      continue;
    }

    out.push(raw.replace(/\s+$/u, ''));
  }

  // Drop leading blank lines for tighter cards.
  while (out.length && !out[0].trim()) out.shift();
  let result = out.join('\n');
  if (result && !result.endsWith('\n')) result += '\n';
  return result;
}

function matchSectionLabel(line: string): string | null {
  let trimmed = line.trim();
  if (trimmed.startsWith('## ')) {
    trimmed = trimmed.slice(3).trim();
  } else if (trimmed.startsWith('# ') && !trimmed.startsWith('## ')) {
    return null;
  }

  if (
    (trimmed.startsWith('**') && trimmed.endsWith('**') && trimmed.length > 4) ||
    (trimmed.startsWith('__') && trimmed.endsWith('__') && trimmed.length > 4)
  ) {
    trimmed = trimmed.slice(2, -2).trim();
  }
  trimmed = trimmed.replace(/[:：]\s*$/u, '').trim();

  for (const [alias, canonical] of SECTION_ALIASES) {
    if (trimmed === alias) return canonical;
  }
  return null;
}
